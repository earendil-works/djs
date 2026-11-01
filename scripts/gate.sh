#!/usr/bin/env bash
# The merge gate. Must pass on a branch before it is merged into main.
# Keep this fast; extend it as the project grows (test262 ratchet, difftests, ...).
set -euo pipefail
cd "$(dirname "$0")/.."

# Full width for the gate (only one runs at a time, under the merge lock); the repo's
# .cargo/config.toml caps everything else at 6 (D012).
ncpu=$(sysctl -n hw.ncpu 2>/dev/null || nproc)
export CARGO_BUILD_JOBS="$ncpu" RUST_TEST_THREADS="$ncpu" DJS_JOBS="$ncpu"

echo "== fmt";   cargo fmt --all --check
echo "== clippy"; cargo clippy --workspace --all-targets --quiet -- -D warnings
echo "== test";  cargo test --workspace --quiet
echo "== no_std/wasm build of djs-core"
cargo build -p djs-core --target wasm32-unknown-unknown --quiet
# Clippy for wasm32 in release (no debug_assertions), so warnings that only appear in
# that configuration fail the gate too.
echo "== clippy (wasm32, release)"
cargo clippy -p djs-core -p djs-wasm --target wasm32-unknown-unknown --release --quiet -- -D warnings

# Additional gate steps registered by issues go below this line.
echo "== builtins/gen is up to date (0017)"
./scripts/bgen.sh --check

# 0018: the host-side JS package (node >= 22.6 for type stripping), and the wire format
# across languages: node encodes a corpus, djs decodes it.
# 0023: djs-wasm, which the js tests (sandbox.test.ts) load. A debug build is enough.
echo "== djs-wasm"
cargo build -p djs-wasm --target wasm32-unknown-unknown --features compiled-prelude --quiet
export DJS_WASM="$PWD/target/wasm32-unknown-unknown/debug/djs_wasm.wasm"
echo "== js tests"; node --test js/test/*.test.ts
echo "== wire cross-language"
wire_dir=$(mktemp -d)
node js/test/gen-cross.ts "$wire_dir" > /dev/null
DJS_WIRE_CROSS="$wire_dir" cargo test -p djs-core --quiet wire_cross -- --ignored
# 0021: the outbound reader, on heap images of the same corpus plus hand-built cases.
echo "== heap reader cross-language"
heap_dir=$(mktemp -d)
DJS_WIRE_CROSS="$wire_dir" DJS_READER_DIR="$heap_dir" \
  cargo test -p djs-core --quiet reader_images -- --ignored
node js/test/reader-cross.ts "$wire_dir" "$heap_dir"
rm -rf "$wire_dir" "$heap_dir"
echo "== difftest on djs: ratchet, plus suspension and GC stress (0016)"
cargo run --quiet -p djs-difftest -- --engine djs --ratchet --suspend-every 97 --gc-stress --quiet --no-diff > target/difftest-djs.log \
  || { cat target/difftest-djs.log; exit 1; }
grep -E '^(stress|ratchet):' target/difftest-djs.log
echo "== test262 on djs: the active group against the ratchet (0007)"
./scripts/fetch-test262.sh > /dev/null
cargo run --quiet -p djs-test262 -- run --engine djs --group active --quiet \
  --expected tests/test262/expected-failures.txt > target/test262-djs.log \
  || { cat target/test262-djs.log; exit 1; }
grep -E '^ratchet:|^\| active' target/test262-djs.log
echo "gate: OK"
