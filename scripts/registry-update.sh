#!/usr/bin/env bash
# Brings the builtin registry (tests/builtins/registry.tsv, D2371) up to date with this
# build: appends new builtins and intrinsics, tombstones the entries that no longer
# resolve (with REASON, e.g. the issue that removed them). Never renumbers.
#   scripts/registry-update.sh [REASON]
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -q -p djs-prelude --example registry -- update "${1:-removed}"
