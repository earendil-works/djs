#!/bin/sh
# Builds the djs CLI (dev profile, niced, -j 6) at each merge point and keeps the binary:
# work/scratch/0299/bins/<sha>/djs. One detached worktree, one shared target dir, so
# consecutive points build incrementally. A point that fails to build gets a FAILED file.
# Usage: build.sh < points   (lines "pos sha issue", as points.sh prints)
set -u
S=$(cd "$(dirname "$0")/../../.." && pwd)/work/scratch/0299
WT=$S/wt
mkdir -p "$S/bins"
[ -d "$WT" ] || git worktree add --detach "$WT" main >/dev/null 2>&1
while read -r pos sha issue; do
  out=$S/bins/$sha
  [ -x "$out/djs" ] || [ -f "$out/FAILED" ] && continue
  mkdir -p "$out"
  git -C "$WT" checkout -q --detach "$sha" || { echo checkout > "$out/FAILED"; continue; }
  t0=$(date +%s)
  if (cd "$WT" && nice -n 10 cargo build -q -j 6 -p djs-cli --target-dir "$S/target-hist") > "$out/build.log" 2>&1; then
    cp "$S/target-hist/debug/djs" "$out/djs"
    echo "$pos $sha $issue ok $(( $(date +%s) - t0 ))s"
  else
    echo build > "$out/FAILED"
    echo "$pos $sha $issue FAILED"
  fi
done
