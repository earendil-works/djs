#!/usr/bin/env bash
# Merge the current issue branch into main. Run from inside an issue worktree.
#
# Takes a lock (so two agents never merge at the same time), rebases the branch onto
# main, runs the gate, then fast-forwards main in the primary checkout.
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
trap 'rm -rf "$lock"' EXIT

echo "== rebase onto main"
git rebase main

echo "== gate"
./scripts/gate.sh

echo "== fast-forward main"
git -C "$main_root" merge --ff-only "$branch"
echo "merge-to-main: main is now at $(git rev-parse --short main)"
