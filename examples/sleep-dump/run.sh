#!/bin/sh
# Builds djs-wasm with the dumper (the `dump` feature, off in the shipped module; the
# gate's build) and runs the example: `examples/sleep-dump/run.sh [--restart] [--ms N]`.
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
cargo build -p djs-wasm --target wasm32-unknown-unknown --features compiled-prelude,dump --quiet
DJS_WASM="$root/target/wasm32-unknown-unknown/debug/djs_wasm.wasm" exec node examples/sleep-dump/sleep-dump.ts "$@"
