#!/usr/bin/env bash
# pi's codemode tests in the adapter's next mode with a dump and a resume in a new instance
# at every idle point (D2650, D2651), on the release dump module (target/djs-dump.wasm,
# built here: one LTO build). For the gate's full run, not every merge (0176). Passes when
# only the tests in tests/pi/expected-failures.txt fail.
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/build-wasm.sh --dump > /dev/null
DJS_WASM="$PWD/target/djs-dump.wasm" node js/pi/pi-tests.ts --next --dump-every-idle \
  --out target/pi-codemode-dump --expected tests/pi/expected-failures.txt > target/pi-dump.log 2>&1 \
  || { cat target/pi-dump.log; exit 1; }
grep -E '^pi tests:' target/pi-dump.log
