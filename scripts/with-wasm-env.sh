#!/usr/bin/env bash
# Runs CMD... in the shipped wasm module's build environment (scripts/wasm-build-env.sh).
set -euo pipefail
while IFS= read -r line; do
  export "${line%%=*}=${line#*=}"
done < <("$(dirname "$0")/wasm-build-env.sh")
exec "$@"
