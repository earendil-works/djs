#!/usr/bin/env bash
# Merge the current issue branch into main. Run from inside an issue worktree.
#
# 1. Rebase the branch onto main and regenerate the generated files (D1791). No lock.
# 2. Gate the result in the shared gate worktree .worktrees/gate (D1792), holding the gate
#    lock, which guards that worktree; not the merge lock (D2666).
# 3. Take the merge lock. If main is still where step 1 put the branch, fast-forward it
#    and release: the merge lock is held for seconds. If main moved meanwhile, release
#    it and go back to 1: the new gate finds every leg whose inputs main's move did not
#    touch in its cache (D2662), so it costs what the move can affect. After two such
#    lost attempts the third holds the merge lock from 1 to 3, as before, so no branch
#    starves.
#
# main only ever fast-forwards to a commit that was itself gated.
set -euo pipefail

# bash reads a script while it runs it, and the rebases below can rewrite this file: run
# a copy.
if [ -z "${DJS_MERGE_SCRIPTS:-}" ]; then
  export DJS_MERGE_SCRIPTS; DJS_MERGE_SCRIPTS=$(cd "$(dirname "$0")" && pwd)
  copy=$(mktemp "${TMPDIR:-/tmp}/merge-to-main.XXXXXX")
  cp "$0" "$copy"
  exec bash "$copy" "$@"
fi
rm -f "$0"

branch=$(git branch --show-current)
case "$branch" in
  issue/*) ;;
  *) echo "merge-to-main: run this from an issue/* branch (current: '$branch')" >&2; exit 1 ;;
esac

main_root=$(git worktree list --porcelain | awk '/^worktree /{print $2; exit}')
here=$(git rev-parse --show-toplevel)
if [ "$here" = "$main_root" ]; then
  echo "merge-to-main: run this from the issue worktree, not the main checkout" >&2
  exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "merge-to-main: worktree has uncommitted changes; commit them first" >&2
  exit 1
fi

scratch="$main_root/work/scratch"
mkdir -p "$scratch"
gate_wt="$main_root/.worktrees/gate"
merge_lock="$scratch/merge.lock"
gate_lock="$scratch/gate.lock"
# shellcheck source=merge-lock.sh
source "$DJS_MERGE_SCRIPTS/merge-lock.sh"

# One line per merge in work/scratch/merge-log.tsv (0079): when it started, the seconds
# spent waiting for locks, rebasing and regenerating, gating, in total, how it ended, the
# seconds the merge lock was held, and the number of attempts.
t_start=$(date +%s); wait_s=0; rebase_s=0; gate_s=0; held_s=0; attempts=0
outcome=failed-before-lock
held=()   # the locks this process holds, released on exit
log_merge() {
  local when; when=$(date -r "$t_start" '+%F %T')
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$when" "$branch" "$wait_s" "$rebase_s" "$gate_s" \
    $(($(date +%s) - t_start)) "$outcome" "$held_s" "$attempts" >> "$scratch/merge-log.tsv"
  # The gate's per-step times (gate.py writes them), one line per step.
  if [ -n "${gate_ran:-}" ] && [ -f "$gate_wt/target/gate-steps.tsv" ]; then
    sed "s|^|$when\t$branch\t|" "$gate_wt/target/gate-steps.tsv" >> "$scratch/gate-steps.tsv"
  fi
  # The gate's 10 slowest unit tests and test262 tests (empty when those legs were cached).
  if [ "$outcome" = merged ]; then
    awk -F'\t' -v p="$when\t$branch" '{print p "\ttest\t" $1 "\t" $2 "::" $3}' \
      "$gate_wt/target/gate-slowest-tests.tsv" >> "$scratch/slow-tests.tsv" 2>/dev/null || true
    awk -v p="$when\t$branch" '{sub("s$", "", $1); print p "\ttest262\t" $1 "\t" $2}' OFS='\t' \
      "$gate_wt/target/gate-slowest-test262.txt" >> "$scratch/slow-tests.tsv" 2>/dev/null || true
  fi
}
release() { local l; for l in ${held[@]+"${held[@]}"}; do rm -rf "$l"; done; held=(); }
trap 'release; log_merge' EXIT
take() {  # take LOCK: wait for it, counting the wait
  local t; t=$(date +%s)
  take_merge_lock "$1" "$branch" 360 || exit 1
  held+=("$1")
  wait_s=$((wait_s + $(date +%s) - t))
}
drop() {  # drop LOCK
  local keep=() l
  rm -rf "$1"
  for l in ${held[@]+"${held[@]}"}; do [ "$l" = "$1" ] || keep+=("$l"); done
  held=(${keep[@]+"${keep[@]}"})
}

# The fast-forward needs a clean primary checkout: check before a gate (0079).
check_primary() {
  if ! git -C "$main_root" diff --quiet || ! git -C "$main_root" diff --cached --quiet; then
    echo "merge-to-main: the primary checkout ($main_root) has uncommitted changes;" \
         "the coordinator commits there under the lock (scripts/with-merge-lock.sh)" >&2
    exit 1
  fi
}

# Rebase onto main, regenerate, commit; sets $base (main) and $sha (the commit to gate).
rebase_and_regen() {
  local t; t=$(date +%s)
  outcome=failed-rebase
  echo "== rebase onto main"
  # Generated files merge through drivers and are regenerated below (D1791).
  ./scripts/setup-merge-drivers.sh
  local old_base; old_base=$(git merge-base main HEAD)
  base=$(git rev-parse main)
  git rebase main
  # Regenerate the generated files (and the heap layout when both main and the branch
  # changed heap/), plus every file a merge driver kept one side of, and commit the result,
  # so the gate sees a consistent tree.
  local regen=()
  if git diff --name-only "$old_base" main | grep -q '^crates/djs-core/src/heap/' &&
     git diff --name-only main HEAD | grep -q '^crates/djs-core/src/heap/'; then
    regen+=(--layout)
  fi
  echo "== regenerate generated files ${regen[*]:-}"
  ./scripts/regen.sh ${regen[@]+"${regen[@]}"}
  if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    local id=${branch#issue/}
    git commit -qam "${id%%-*}: regenerate generated files after the rebase onto main"
    echo "merge-to-main: committed regenerated files"
  fi
  sha=$(git rev-parse HEAD)
  rebase_s=$((rebase_s + $(date +%s) - t))
}

# Gate $sha in the persistent gate worktree (D1792): its target/ was last built for the
# previous gate, so builds are incremental against main, and its legs' results are in the
# shared cache (D2662). This worktree is not touched.
gate() {
  local t; t=$(date +%s)
  outcome=failed-gate
  if [ ! -e "$gate_wt/.git" ]; then
    echo "== creating the gate worktree $gate_wt"
    git worktree add -q --detach "$gate_wt" "$sha"
  fi
  git -C "$gate_wt" checkout -q --detach --force "$sha"
  git -C "$gate_wt" clean -fdq   # untracked files only; target/ is ignored and stays
  gate_ran=1
  echo "== gating $(git rev-parse --short "$sha") in $gate_wt"
  (cd "$gate_wt" && ./scripts/gate.sh)
  gate_s=$((gate_s + $(date +%s) - t))
}

fast_forward() {
  outcome=failed-ff
  echo "== fast-forwarding main to $(git rev-parse --short "$sha")"
  git -C "$main_root" merge -q --ff-only "$sha"
  outcome=merged
  echo "merge-to-main: main is now at $(git rev-parse --short main)"
}

while :; do
  attempts=$((attempts + 1))
  check_primary
  if [ "$attempts" -ge 3 ]; then
    # Two attempts lost to a moving main: hold the merge lock from rebase to fast-forward.
    echo "== main moved twice while gating; this attempt holds the merge lock throughout"
    # Locks in one order everywhere, the gate lock first, so two merges never wait on each
    # other.
    take "$gate_lock"
    take "$merge_lock"; t_held=$(date +%s)
    check_primary
    rebase_and_regen
    gate
    fast_forward
    held_s=$((held_s + $(date +%s) - t_held))
    break
  fi
  rebase_and_regen
  echo "== waiting for the gate worktree"
  take "$gate_lock"
  # A merge-to-main from before D2666 gates under the merge lock alone: wait until none
  # is gating before taking over the worktree.
  take "$merge_lock"; drop "$merge_lock"
  # main moved while this waited for the worktree: gate on the new main right away.
  if [ "$(git rev-parse main)" != "$base" ]; then
    echo "== main moved while waiting; rebasing again"
    rebase_and_regen
  fi
  gate
  drop "$gate_lock"
  take "$merge_lock"; t_held=$(date +%s)
  if [ "$(git rev-parse main)" = "$base" ]; then
    check_primary
    fast_forward
    held_s=$((held_s + $(date +%s) - t_held))
    break
  fi
  echo "== main moved while gating ($(git rev-parse --short "$base") -> $(git rev-parse --short main));" \
       "rebasing and gating again (legs main's change did not touch are cached)"
  drop "$merge_lock"
  held_s=$((held_s + $(date +%s) - t_held))
done
