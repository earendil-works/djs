#!/bin/sh
# Runs the example on the release module (scripts/build-wasm.sh: the shipped build):
#   examples/sleep-snapshot/run.sh [ms]             one process: snapshot, drop the sandbox, load
#   examples/sleep-snapshot/run.sh --restart [ms]   two processes: the first snapshots into
#                                                   out/sleep.snapshot and exits, the second loads it
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
wasm=target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
# Built once (a minute or two), and again only when a source of it is newer: asking cargo
# whether anything changed costs more than the rest of a run.
if [ ! -f $wasm ] || [ -n "$(find crates/djs-core crates/djs-prelude crates/djs-lzrc crates/djs-wasm \
  builtins tests/builtins Cargo.toml Cargo.lock -newer $wasm -type f | head -1)" ]; then
  echo "--- building $wasm"
  scripts/build-wasm.sh
fi
export DJS_WASM="$root/$wasm"
# Node caches the code it compiled from the TypeScript sources: a third of its startup.
export NODE_COMPILE_CACHE="$root/target/node-compile-cache"
if [ "$1" = "--restart" ]; then
  shift
  echo "--- node sleep-snapshot.ts start (process 1)"
  node examples/sleep-snapshot/sleep-snapshot.ts start "$@"
  echo "--- node sleep-snapshot.ts resume (process 2)"
  exec node examples/sleep-snapshot/sleep-snapshot.ts resume
fi
exec node examples/sleep-snapshot/sleep-snapshot.ts both "$@"
