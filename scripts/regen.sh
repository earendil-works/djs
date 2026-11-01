#!/usr/bin/env bash
# Rewrites the generated files from their sources (0079, D1791):
#   builtins/gen/prelude.js, builtins/gen/resume-points.txt   (always; scripts/bgen.sh)
#   the INTRINSICS table in bytecode/intrinsics.rs            (--intrinsics)
#   js/src/layout.generated.ts                                (--layout)
# A file the merge driver kept one side of (listed in $GIT_DIR/djs-regen) is always
# regenerated. merge-to-main.sh runs this after the rebase and commits the result, so
# merges of generated text never need resolving by hand.
set -euo pipefail
cd "$(dirname "$0")/.."
intrinsics=false layout=false
for a in "$@"; do
  case "$a" in
    --intrinsics) intrinsics=true ;;
    --layout) layout=true ;;
    *) echo "regen.sh: unknown option $a" >&2; exit 2 ;;
  esac
done
marker=$(git rev-parse --git-path djs-regen)
if [ -f "$marker" ]; then
  grep -q 'bytecode/intrinsics.rs' "$marker" && intrinsics=true
  grep -q 'layout.generated.ts' "$marker" && layout=true
  rm -f "$marker"
fi

./scripts/bgen.sh

if $intrinsics; then
  # The table test prints the expected entries when the table is out of date.
  out=$(cargo test -q -p djs-core --lib bytecode::intrinsics::tests::table_matches_sources 2>&1 || true)
  if echo "$out" | grep -q "replace its entries with:"; then
    echo "$out" | python3 -c '
import re, sys
out = sys.stdin.read()
entries = [l for l in out.split("replace its entries with:", 1)[1].split("\n") if l.startswith("    i(")]
p = "crates/djs-core/src/bytecode/intrinsics.rs"
lines = open(p).read().split("\n")
start = next(i for i, l in enumerate(lines) if l.startswith("pub const INTRINSICS:"))
end = next(i for i in range(start, len(lines)) if lines[i] == "];")
open(p, "w").write("\n".join(lines[: start + 1] + entries + lines[end:]))
'
    echo "regen: rewrote the INTRINSICS table"
  fi
fi

if $layout; then
  DJS_UPDATE_LAYOUT=1 cargo test -q -p djs-core --lib layout_generated > /dev/null
fi
