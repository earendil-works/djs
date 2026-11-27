#!/usr/bin/env bash
# Snapshots continue in another build by name (0302, D3712). Takes logical snapshots at
# every idle point of the codemode corpus and of the tests/dump programs with this tree's
# build, then makes a copy of the tree whose builtins list record fields in another order
# and give state constants other values (scripts/snapshot-perturb.py), checks that the
# copy is still right (the difftest corpus against its ratchet), and loads every snapshot
# into it at its idle point: each must continue as the copy's own run does, or refuse;
# none may continue differently.
#   scripts/snapshot-perturb.sh
# Builds in target/perturb (both trees; ~2 full builds the first time). The copy and the
# snapshots are in work/scratch/snapshot-perturb.
set -euo pipefail
cd "$(dirname "$0")/.."
root=$(pwd)
scratch="$root/work/scratch/snapshot-perturb"
copy="$scratch/copy"
if [ -d "$copy" ]; then git worktree remove --force "$copy" 2>/dev/null || rm -rf "$copy"; fi
rm -rf "$scratch"
mkdir -p "$scratch/snaps"
export CARGO_TARGET_DIR="$root/target/perturb"
t() { "$root/scripts/timeout.sh" "$@"; }

echo "perturb: this build's snapshots"
DJS_SNAP=1 DJS_SNAP_DIR="$scratch/snaps" t 1800 cargo test -q -p djs-core --lib -j 6 -- \
  vm::snapshot::tests::snapshot_harvest vm::snapshot::tests::snapshot_programs --ignored \
  > "$scratch/harvest.txt" 2>&1 || { cat "$scratch/harvest.txt"; exit 1; }
echo "perturb: $(ls "$scratch/snaps" | wc -l | tr -d ' ') snapshots"

# The tree as it is, committed or not, in a worktree of its own.
git worktree add -q --detach "$copy" HEAD
trap 'git worktree remove --force "$copy" >/dev/null 2>&1 || true' EXIT
rsync -a --delete --exclude .git --exclude target --exclude work --exclude vendor ./ "$copy/"
python3 scripts/snapshot-perturb.py "$copy"
(cd "$copy" && scripts/bgen.sh)

echo "perturb: the perturbed build against the difftest corpus"
(cd "$copy" && t 1800 cargo run -q -p djs-difftest -- --engine djs -j 6 --ratchet --quiet --no-diff) \
  > "$scratch/difftest.txt" 2>&1 || { cat "$scratch/difftest.txt"; exit 1; }
grep "^ratchet:" "$scratch/difftest.txt"

echo "perturb: this build's snapshots in the perturbed build"
(cd "$copy" && DJS_SNAP_IMPORT="$scratch/snaps" t 1800 cargo test -q -p djs-core --lib -j 6 -- \
  vm::snapshot::tests::snapshot_import_dir --ignored --nocapture) > "$scratch/import.txt" 2>&1 \
  || { cat "$scratch/import.txt"; exit 1; }
line=$(grep "^imported from" "$scratch/import.txt")
echo "perturb: ${line#*: }"
sed -n '/^imported from/,/^test result/p' "$scratch/import.txt" | sed '1d;$d' | head -40
same=$(sed -E 's/.* ([0-9]+) continue as.*/\1/' <<< "$line")
differ=$(sed -E 's/.* ([0-9]+) differ.*/\1/' <<< "$line")
if [ "$differ" != 0 ] || [ "$same" = 0 ]; then
  echo "perturb: FAILED ($differ continue differently, $same the same)"
  exit 1
fi
echo "perturb: OK"
