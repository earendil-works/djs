#!/usr/bin/env bash
# Native vs wasm determinism (0072, D1720): runs a few codemode bodies through djs-wasm's
# exports natively (djs-abidrive) and in the wasm build (abidrive.mjs, the same export
# calls) at two fuel slices, and requires identical output: the build hash, the raw and the
# collected heap image hashes, the number of stops (fuel) and the result.
# usage: scripts/determinism/native-vs-wasm.sh <djs_wasm.wasm>
set -euo pipefail
cd "$(dirname "$0")/../.."
wasm=${1:?usage: native-vs-wasm.sh <djs_wasm.wasm>}
cargo build --quiet -p djs-difftest --bin djs-abidrive
native=target/debug/djs-abidrive
fail=0
for case in floats_body general_body deep_body weakref; do
  for slice in 1000000 997; do
    a=$("$native" "scripts/determinism/js/$case.js" "$slice")
    b=$(node scripts/determinism/abidrive.mjs "$wasm" "scripts/determinism/js/$case.js" "$slice")
    if [ "$a" != "$b" ]; then
      echo "native-vs-wasm: $case.js at slice $slice differs:"
      diff <(echo "$a") <(echo "$b") || true
      fail=1
    fi
  done
done
[ "$fail" = 0 ] && echo "native-vs-wasm: OK, identical images and fuel"
exit "$fail"
