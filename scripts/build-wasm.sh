#!/usr/bin/env bash
# Builds djs-wasm with the `wasm` profile: target/wasm32-unknown-unknown/wasm/djs_wasm.wasm,
# the one module djs ships (the npm package's wasm/djs.wasm) and the default module location
# of js/src/sandbox.ts. It has the dumper (the `dump` feature, with what it turns on: lazy
# functions, the compile cache, the prelude's names), so every host dumps and resumes with the
# module it runs (D3600). The profile strips the custom sections
# (names, producers) for shipping (D1575); --keep-names keeps the name section, for
# profiling and size attribution (scripts/wasm-funcs.py). Switching rebuilds the module.
# --docs builds the docs site's module (the `docs` feature: `djs_disasm` for the live
# examples' bytecode; and `dump`, for the dumps of their `debugger;` pauses, D3050) into
# its own target directory, target/docs-wasm, so it never replaces the shipping module.
# --slim builds the slim module (no default features: neither `full` nor `unicode`, D2411)
# in target/slim-wasm and copies it to target/djs-slim.wasm: no package ships it; it is a
# size reference and the module of pi's adapter tests without the lockdown.
# The shipped and slim modules (and --keep-names, the shipped one with names) read and write
# the arena unchecked (`unsafe-heap`, D3100): the wasm instance is the sandbox. The docs
# module keeps the checks.
set -euo pipefail
cd "$(dirname "$0")/.."
features=(--features unsafe-heap,cold-unicode,dump)
target=target
copy=
case "${1:-}" in
  --keep-names) export CARGO_PROFILE_WASM_STRIP=false ;;
  --docs) features=(--features docs,dump); target=target/docs-wasm ;;
  --slim) features=(--no-default-features --features unsafe-heap); target=target/slim-wasm; copy=target/djs-slim.wasm ;;
  # The old opt-in flag: stripping is the default now.
  --strip | "") ;;
  *) echo "usage: scripts/build-wasm.sh [--keep-names | --docs | --slim]" >&2; exit 2 ;;
esac
# The shipped module's build environment (D2331, D2333).
CARGO_TARGET_DIR="$target" scripts/with-wasm-env.sh cargo build -p djs-wasm --target wasm32-unknown-unknown --profile wasm --quiet ${features[@]+"${features[@]}"}
out=$target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
if [ -n "$copy" ]; then
  cp "$out" "$copy"
  out=$copy
fi
ls -l "$out" | awk '{print $5 " bytes: " $9}'
