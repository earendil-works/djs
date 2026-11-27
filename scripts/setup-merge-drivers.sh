#!/usr/bin/env bash
# Registers the merge drivers that .gitattributes names (0079, D1791). Idempotent; the
# config is the repository's (shared by all worktrees). merge-to-main.sh runs it.
set -euo pipefail
cd "$(dirname "$0")/.."
git config merge.djs-generated.name "generated file: keep one side, scripts/regen.sh rewrites it"
git config merge.djs-generated.driver "python3 scripts/merge-driver.py generated %O %A %B %P"
git config merge.djs-intrinsics.name "intrinsics.rs: table conflicts are regenerated"
git config merge.djs-intrinsics.driver "python3 scripts/merge-driver.py intrinsics %O %A %B %P"
