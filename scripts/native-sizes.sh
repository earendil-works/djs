#!/usr/bin/env bash
# The wasm code size of the Rust backend (0063): each @native builtin's state machine
# (and the leaf helpers it calls) in the `wasm` profile build of djs-wasm, largest first,
# and the totals. Needs wasm-objdump (wabt).
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/build-wasm.sh > /dev/null
wasm=target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
code=$(wasm-objdump -x -j Code "$wasm")
echo "$code" | awk '/8builtins3gen7natives/ {
    split($3, a, "="); n = $4
    sub(/^<.*8builtins3gen7natives[0-9]+/, "", n); sub(/>$/, "", n)
    printf "%8d  %s\n", a[2], n
  }' | sort -rn
echo "$code" | awk '
  /8builtins3gen7natives/ { split($3, a, "="); gen += a[2] }
  /6interp3run6native/ { split($3, a, "="); rt += a[2] }
  END {
    printf "%8d  total generated (builtins/gen/natives.rs)\n", gen
    printf "%8d  runtime (interp/run/native.rs)\n", rt
  }'
ls -l "$wasm" | awk '{print $5 "  djs_wasm.wasm"}'
