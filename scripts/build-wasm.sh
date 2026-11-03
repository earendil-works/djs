#!/usr/bin/env bash
# Builds djs-wasm with the `wasm` profile: target/wasm32-unknown-unknown/wasm/djs_wasm.wasm,
# the default module location of js/src/sandbox.ts. The profile strips the custom sections
# (names, producers) for shipping (D1575); --keep-names keeps the name section, for
# profiling and size attribution (scripts/wasm-funcs.py). Switching rebuilds the module.
# --docs builds the docs site's module (the `docs` feature: `djs_disasm` for the live
# examples' bytecode) into its own target directory, target/docs-wasm, so it never
# replaces the shipping module. --slim builds the slim module (no default features: neither
# `full` nor `unicode`, D2411) in target/slim-wasm and copies it to target/djs-slim.wasm.
set -euo pipefail
cd "$(dirname "$0")/.."
features=()
target=target
slim=false
case "${1:-}" in
  --keep-names) export CARGO_PROFILE_WASM_STRIP=false ;;
  --docs) features=(--features docs); target=target/docs-wasm ;;
  --slim) features=(--no-default-features); target=target/slim-wasm; slim=true ;;
  # The old opt-in flag: stripping is the default now.
  --strip | "") ;;
  *) echo "usage: scripts/build-wasm.sh [--keep-names | --docs | --slim]" >&2; exit 2 ;;
esac
# The shipped module's build environment (D2331, D2333).
CARGO_TARGET_DIR="$target" scripts/with-wasm-env.sh cargo build -p djs-wasm --target wasm32-unknown-unknown --profile wasm --quiet ${features[@]+"${features[@]}"}
out=$target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
if $slim; then
  cp "$out" target/djs-slim.wasm
  out=target/djs-slim.wasm
fi
ls -l "$out" | awk '{print $5 " bytes: " $9}'
