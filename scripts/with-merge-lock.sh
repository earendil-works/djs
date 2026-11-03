#!/usr/bin/env bash
# Run a command while holding the merge lock (the same lock merge-to-main.sh takes).
# The coordinator uses this for its own commits on main, so that it never moves main
# while an agent is between rebase and fast-forward.
#   scripts/with-merge-lock.sh git commit -qam "board: ..."
set -euo pipefail
main_root=$(git worktree list --porcelain | awk '/^worktree /{print $2; exit}')
lock="$main_root/work/scratch/merge.lock"
mkdir -p "$main_root/work/scratch"
# shellcheck source=merge-lock.sh
source "$(dirname "$0")/merge-lock.sh"
take_merge_lock "$lock" coordinator 720 || exit 1
trap 'rm -rf "$lock"' EXIT
"$@"
