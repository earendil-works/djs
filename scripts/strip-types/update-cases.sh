#!/usr/bin/env bash
# Regenerate tests/strip-types/*.js with ts-blank-space (the gate compares djs to them).
set -euo pipefail
cd "$(dirname "$0")/../.."
vendor=$(scripts/strip-types/fetch.sh | tail -1)
tmp=$(mktemp -d)
STRIP_NODE_DIR="$vendor/node" node scripts/strip-types/blank.mjs "$tmp" tests/strip-types \
  tests/strip-types/*.ts
for f in tests/strip-types/*.ts; do
  name=$(basename "$f" .ts)
  if [ -f "$tmp/$name.ts" ]; then
    mv "$tmp/$name.ts" "tests/strip-types/$name.js"
  else
    echo "update-cases.sh: ts-blank-space did not erase $f" >&2
    exit 1
  fi
done
rm -rf "$tmp"
