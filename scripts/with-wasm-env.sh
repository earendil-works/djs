#!/usr/bin/env bash
# Runs CMD... in the shipped wasm module's build environment (scripts/wasm-build-env.sh).
set -euo pipefail
# Captured first: a failure inside `< <(...)` would not stop the script.
env=$("$(dirname "$0")/wasm-build-env.sh")
while IFS= read -r line; do
  export "${line%%=*}=${line#*=}"
done <<< "$env"
exec "$@"
