#!/usr/bin/env bash
# Run a command while holding the merge lock (the same lock merge-to-main.sh takes).
# The coordinator uses this for its own commits on main, so that it never moves main
# while an agent is between rebase and fast-forward.
#   scripts/with-merge-lock.sh git commit -qam "board: ..."
set -euo pipefail
main_root=$(git worktree list --porcelain | awk '/^worktree /{print $2; exit}')
lock="$main_root/work/scratch/merge.lock"
mkdir -p "$main_root/work/scratch"
waited=0
until mkdir "$lock" 2>/dev/null; do
  waited=$((waited + 1))
  if [ $waited -gt 720 ]; then echo "with-merge-lock: lock held for 60+ minutes" >&2; exit 1; fi
  sleep 5
done
echo "coordinator $(date)" > "$lock/owner"
trap 'rm -rf "$lock"' EXIT
"$@"
