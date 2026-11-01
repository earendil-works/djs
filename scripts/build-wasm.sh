#!/usr/bin/env bash
# Builds djs-wasm with the `wasm` profile: target/wasm32-unknown-unknown/wasm/djs_wasm.wasm,
# the default module location of js/src/sandbox.ts. The profile strips the custom sections
# (names, producers) for shipping (D1575); --keep-names keeps the name section, for
# profiling and size attribution (scripts/wasm-funcs.py). Switching rebuilds the module.
# --docs builds the docs site's module (the `docs` feature: `djs_disasm` for the live
# examples' bytecode) into its own target directory, target/docs-wasm, so it never
# replaces the shipping module.
set -euo pipefail
cd "$(dirname "$0")/.."
features=()
target=target
case "${1:-}" in
  --keep-names) export CARGO_PROFILE_WASM_STRIP=false ;;
  --docs) features=(--features docs); target=target/docs-wasm ;;
  # The old opt-in flag: stripping is the default now.
  --strip | "") ;;
  *) echo "usage: scripts/build-wasm.sh [--keep-names | --docs]" >&2; exit 2 ;;
esac
# No absolute build-machine paths in the module (D2331).
CARGO_ENCODED_RUSTFLAGS=$(scripts/wasm-rustflags.sh)
export CARGO_ENCODED_RUSTFLAGS
CARGO_TARGET_DIR="$target" cargo build -p djs-wasm --target wasm32-unknown-unknown --profile wasm --quiet ${features[@]+"${features[@]}"}
out=$target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
ls -l "$out" | awk '{print $5 " bytes: " $9}'
