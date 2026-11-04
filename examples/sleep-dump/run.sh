#!/bin/sh
# Builds djs-wasm with the dumper (the `dump` feature, off in the shipped module; the
# gate's build) and runs the example:
#   examples/sleep-dump/run.sh [ms]             one process: suspend, drop the sandbox, resume
#   examples/sleep-dump/run.sh --restart [ms]   two processes: the first suspends into
#                                               out/sleep.dump.js and exits, the second resumes
#   examples/sleep-dump/run.sh --node [ms]      the same, the second process continuing the
#                                               dump in node itself instead of djs
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
cargo build -p djs-wasm --target wasm32-unknown-unknown --features compiled-prelude,dump --quiet
export DJS_WASM="$root/target/wasm32-unknown-unknown/debug/djs_wasm.wasm"
if [ "$1" = "--restart" ]; then
  shift
  echo "--- node sleep-dump.ts start (process 1)"
  node examples/sleep-dump/sleep-dump.ts start "$@"
  echo "--- node sleep-dump.ts resume (process 2)"
  exec node examples/sleep-dump/sleep-dump.ts resume
fi
if [ "$1" = "--node" ]; then
  shift
  echo "--- node sleep-dump.ts start (process 1)"
  node examples/sleep-dump/sleep-dump.ts start "$@"
  echo "--- node sleep-dump.ts node (process 2, no djs)"
  exec node examples/sleep-dump/sleep-dump.ts node
fi
exec node examples/sleep-dump/sleep-dump.ts both "$@"
