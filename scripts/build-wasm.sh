#!/usr/bin/env bash
# Builds djs-wasm with the `wasm` profile: target/wasm32-unknown-unknown/wasm/djs_wasm.wasm,
# the default module location of js/src/sandbox.ts. The profile strips the custom sections
# (names, producers) for shipping (D1575); --keep-names keeps the name section, for
# profiling and size attribution (scripts/wasm-funcs.py). Switching rebuilds the module.
set -euo pipefail
cd "$(dirname "$0")/.."
case "${1:-}" in
  --keep-names) export CARGO_PROFILE_WASM_STRIP=false ;;
  # The old opt-in flag: stripping is the default now.
  --strip | "") ;;
  *) echo "usage: scripts/build-wasm.sh [--keep-names]" >&2; exit 2 ;;
esac
cargo build -p djs-wasm --target wasm32-unknown-unknown --profile wasm --quiet
out=target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
ls -l "$out" | awk '{print $5 " bytes: " $9}'
