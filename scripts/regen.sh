#!/usr/bin/env bash
# Rewrites the generated files from their sources (0079, D1791):
#   builtins/gen/prelude.js, builtins/gen/resume-points.txt   (scripts/bgen.sh)
#   the INTRINSICS table in bytecode/intrinsics.rs            (scripts/gen-intrinsics.py)
#   js/src/layout.generated.ts                                (--layout: builds djs-core)
# A file the merge driver kept one side of (listed in $GIT_DIR/djs-regen) is always
# regenerated. merge-to-main.sh runs this after the rebase and commits the result, so
# merges of generated text never need resolving by hand.
set -euo pipefail
cd "$(dirname "$0")/.."
layout=false
for a in "$@"; do
  case "$a" in
    --layout) layout=true ;;
    *) echo "regen.sh: unknown option $a" >&2; exit 2 ;;
  esac
done
marker=$(git rev-parse --git-path djs-regen)
if [ -f "$marker" ]; then
  if grep -q 'layout.generated.ts' "$marker"; then layout=true; fi
  rm -f "$marker"
fi

./scripts/bgen.sh

python3 scripts/gen-intrinsics.py

if $layout; then
  DJS_UPDATE_LAYOUT=1 cargo test -q -p djs-core --lib layout_generated > /dev/null
fi
