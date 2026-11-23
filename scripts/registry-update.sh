#!/usr/bin/env bash
# Brings the builtin registry (tests/builtins/registry.tsv, D2371, D3644) up to date with
# this build: drops the entries that no longer resolve, appends new builtins and
# intrinsics, and numbers the entries by position.
#   scripts/registry-update.sh
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -q -p djs-prelude --example registry -- update
