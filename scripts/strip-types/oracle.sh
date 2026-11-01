#!/usr/bin/env bash
# The strip-types oracle (TESTING.md "TypeScript"): `djs strip-types` must match
# ts-blank-space byte for byte on every corpus file that ts-blank-space erases.
#
#   scripts/strip-types/oracle.sh
#
# Fetches the oracle and the corpus on first use (fetch.sh), runs ts-blank-space once
# into work/scratch/strip-types/expected (delete it to rerun), then
# `djs strip-types --compare`. The report (work/scratch/strip-types/report.txt) lists
# the differences and the files djs rejects (decorators, `using`, `accessor` and the
# conformance tests' deliberate JavaScript errors, which TypeScript only reports in
# its checker); the summary and the error clusters are printed. Exits 1 on a difference.
set -euo pipefail
cd "$(dirname "$0")/../.."
vendor=$(scripts/strip-types/fetch.sh | tail -1)
corpus="$vendor/corpus"
scratch=work/scratch/strip-types

if [ ! -d "$scratch/expected" ]; then
  mkdir -p "$scratch/expected"
  find "$corpus" -name '*.ts' ! -name '*.d.ts' -print0 \
    | STRIP_NODE_DIR="$vendor/node" xargs -0 node scripts/strip-types/blank.mjs \
        "$scratch/expected" "$corpus" > "$scratch/skipped.txt"
fi
cargo build -q --release -p djs-cli
status=0
scripts/timeout.sh 1800 target/release/djs strip-types --compare "$scratch/expected" "$corpus" \
  > "$scratch/report.txt" || status=$?
echo "ts-blank-space: $(grep -c '^unsupported' "$scratch/skipped.txt") files unsupported, $(grep -c '^invalid' "$scratch/skipped.txt") with syntax errors (not compared)"
tail -1 "$scratch/report.txt"
grep '^differ' "$scratch/report.txt" | head -20 || true
echo "djs syntax errors by message:"
grep '^error' "$scratch/report.txt" | sed 's/^error [^ ]*: //' | sed 's/[0-9][0-9]*/N/g' \
  | sort | uniq -c | sort -rn | head -20
exit "$status"
