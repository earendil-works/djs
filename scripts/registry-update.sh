#!/usr/bin/env bash
# Brings the builtin registry (tests/builtins/registry.tsv, D2371) up to date with this
# build: appends new builtins and intrinsics, tombstones the entries that no longer
# resolve (with REASON, e.g. the issue that removed them). Never renumbers. Then writes
# the ids the slim build leaves out (tests/builtins/registry-slim.txt, D2411) from a slim
# build of the same tree (its own target directory: the features differ).
#   scripts/registry-update.sh [REASON]
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -q -p djs-prelude --example registry -- update "${1:-removed}"
CARGO_TARGET_DIR=target/slim cargo run -q -p djs-prelude --no-default-features \
  --features raw,registry --example registry -- update
