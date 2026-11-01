#!/usr/bin/env bash
# Builds djs-wasm with the `wasm` profile: target/wasm32-unknown-unknown/wasm/djs_wasm.wasm,
# the default module location of js/src/sandbox.ts. Pass --strip to also remove the custom
# sections (names, producers) with wasm-strip, for shipping.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p djs-wasm --target wasm32-unknown-unknown --profile wasm --quiet
out=target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
if [ "${1:-}" = "--strip" ]; then
  wasm-strip "$out"
fi
ls -l "$out" | awk '{print $5 " bytes: " $9}'
