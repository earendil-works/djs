#!/usr/bin/env bash
# Merge the current issue branch into main. Run from inside an issue worktree.
#
# Takes a lock (so two agents never merge at the same time), rebases the branch onto
# main, regenerates the generated files (D1791), runs the gate on that commit in the
# shared gate worktree .worktrees/gate (D1792), then fast-forwards main in the primary
# checkout to the gated commit.
set -euo pipefail

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

# One line per merge attempt in work/scratch/merge-log.tsv (0079): when it started, the
# lock wait, rebase + regeneration, gate and total seconds, and how it ended.
t_start=$(date +%s); t_lock=$t_start; t_rebased=$t_start; t_gated=$t_start
outcome=failed-before-lock
log_merge() {
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$(date -r "$t_start" '+%F %T')" "$branch" \
    $((t_lock - t_start)) $((t_rebased - t_lock)) $((t_gated - t_rebased)) \
    $(($(date +%s) - t_start)) "$outcome" >> "$main_root/work/scratch/merge-log.tsv"
}

lock="$main_root/work/scratch/merge.lock"
mkdir -p "$main_root/work/scratch"
waited=0
until mkdir "$lock" 2>/dev/null; do
  if [ $waited -eq 0 ]; then echo "merge-to-main: waiting for merge lock ($lock)"; fi
  waited=$((waited + 1))
  if [ $waited -gt 360 ]; then echo "merge-to-main: lock held for 30+ minutes; ask the coordinator" >&2; exit 1; fi
  sleep 5
done
echo "$branch $(date)" > "$lock/owner"
t_lock=$(date +%s); t_rebased=$t_lock; t_gated=$t_lock; outcome=failed-rebase
trap 'log_merge; rm -rf "$lock"' EXIT

# The fast-forward at the end needs a clean primary checkout: check now, not after a
# full gate (0079).
if ! git -C "$main_root" diff --quiet || ! git -C "$main_root" diff --cached --quiet; then
  echo "merge-to-main: the primary checkout ($main_root) has uncommitted changes;" \
       "the coordinator commits there under the lock (scripts/with-merge-lock.sh)" >&2
  exit 1
fi

echo "== rebase onto main"
# Generated files merge through drivers and are regenerated below (D1791).
./scripts/setup-merge-drivers.sh
old_base=$(git merge-base main HEAD)
git rebase main

# Regenerate the generated files (and the heap layout when both main and the branch
# changed heap/), plus every file a merge driver kept one side of, and commit the
# result, so the gate sees a consistent tree.
regen=()
if git diff --name-only "$old_base" main | grep -q '^crates/djs-core/src/heap/' &&
   git diff --name-only main HEAD | grep -q '^crates/djs-core/src/heap/'; then
  regen+=(--layout)
fi
echo "== regenerate generated files ${regen[*]:-}"
./scripts/regen.sh ${regen[@]+"${regen[@]}"}
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  id=${branch#issue/}
  git commit -qam "${id%%-*}: regenerate generated files after the rebase onto main"
  echo "merge-to-main: committed regenerated files"
fi

# The gate runs in one persistent worktree (D1792): its target/ was last built for the
# previous merge, so the build is incremental against main instead of against this
# worktree's target/, which may be hours behind and lacks the gate's other profiles
# (wasm32, release clippy). This worktree is not touched.
sha=$(git rev-parse HEAD)
gate_wt="$main_root/.worktrees/gate"
if [ ! -e "$gate_wt/.git" ]; then
  echo "== creating the gate worktree $gate_wt"
  git worktree add -q --detach "$gate_wt" "$sha"
fi
git -C "$gate_wt" checkout -q --detach --force "$sha"
git -C "$gate_wt" clean -fdq   # untracked files only; target/ is ignored and stays

t_rebased=$(date +%s); t_gated=$t_rebased; outcome=failed-gate
echo "== gate (in $gate_wt at $(git rev-parse --short "$sha"))"
(cd "$gate_wt" && ./scripts/gate.sh)

t_gated=$(date +%s); outcome=failed-ff
echo "== fast-forward main"
git -C "$main_root" merge --ff-only "$sha"
outcome=merged
echo "merge-to-main: main is now at $(git rev-parse --short main)"
