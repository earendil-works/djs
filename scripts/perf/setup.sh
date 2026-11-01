#!/usr/bin/env bash
# setup.sh: the builds the perf tools use, in $DJS_PERF_DIR (default work/scratch/perf):
# native release with LTO and one codegen unit (stable layout, D1620) plus its dSYM and a
# full disassembly (all.s, for trace_report.py), and the wasm profile build with the name
# section (STRIP=debuginfo keeps names; strip=false conflicts with lld's reloc compression).
set -euo pipefail
tools="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$tools/../.." && pwd)"
out="${DJS_PERF_DIR:-$root/work/scratch/perf}"
mkdir -p "$out"
cd "$root"
CARGO_TARGET_DIR="$out/target" CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
  cargo build --release -p djs-cli
CARGO_TARGET_DIR="$out/target" CARGO_PROFILE_WASM_STRIP=debuginfo \
  scripts/with-wasm-env.sh cargo build -p djs-wasm --target wasm32-unknown-unknown --profile wasm
dsymutil "$out/target/release/djs" -o "$out/djs.dSYM"
"$(xcrun --find llvm-objdump)" -d --no-show-raw-insn "$out/target/release/djs" | grep -E "^\s*1[0-9a-f]+:" > "$out/all.s"
wasm-objdump -d "$out/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm" > "$out/wasm.dis" 2>/dev/null || true
echo "native: $out/target/release/djs; wasm: $out/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm"
