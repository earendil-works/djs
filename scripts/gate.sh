#!/usr/bin/env bash
# The merge gate. Must pass on a branch before it is merged into main.
# Keep this fast; extend it as the project grows (test262 ratchet, difftests, ...).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== fmt";   cargo fmt --all --check
echo "== clippy"; cargo clippy --workspace --all-targets --quiet -- -D warnings
echo "== test";  cargo test --workspace --quiet
echo "== no_std/wasm build of djs-core"
cargo build -p djs-core --target wasm32-unknown-unknown --quiet

# Additional gate steps registered by issues go below this line.
echo "== builtins/gen is up to date (0017)"
./scripts/bgen.sh --check

# 0018: the host-side JS package (node >= 22.6 for type stripping), and the wire format
# across languages: node encodes a corpus, djs decodes it.
echo "== js tests"; node --test js/test/*.test.ts
echo "== wire cross-language"
wire_dir=$(mktemp -d)
node js/test/gen-cross.ts "$wire_dir" > /dev/null
DJS_WIRE_CROSS="$wire_dir" cargo test -p djs-core --quiet wire_cross -- --ignored
rm -rf "$wire_dir"
echo "gate: OK"
