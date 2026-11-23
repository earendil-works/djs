#!/usr/bin/env bash
# pi's codemode tests in the adapter's next mode with a dump and a resume in a new instance
# at every idle point (D2650, D2651; plain dumps, D2830), on the shipped module (it has the
# dumper, D3600): MODULE, else scripts/build-wasm.sh's build (built here). For the gate's
# full run, not every merge (0176). Passes when only the tests in
# tests/pi/expected-failures.txt fail.
#   scripts/pi-dump-every-idle.sh [MODULE]
set -euo pipefail
cd "$(dirname "$0")/.."
module=${1:-}
if [ -z "$module" ]; then
  scripts/build-wasm.sh > /dev/null
  module=$PWD/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
fi
DJS_WASM="$module" node js/pi/pi-tests.ts --next --dump-every-idle \
  --out target/pi-codemode-dump --expected tests/pi/expected-failures.txt > target/pi-dump.log 2>&1 \
  || { cat target/pi-dump.log; exit 1; }
grep -E '^pi tests:' target/pi-dump.log
