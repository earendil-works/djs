#!/usr/bin/env bash
# Fetch the strip-types oracle (TESTING.md "TypeScript") into vendor/strip-types of the
# primary checkout: ts-blank-space and its TypeScript (npm, pinned) and a TS corpus
# (TypeScript's own src/ at a pinned tag).
set -euo pipefail
cd "$(dirname "$0")/../.."

BLANK_VERSION=0.9.0
TS_VERSION=5.9.3
common=$(git rev-parse --path-format=absolute --git-common-dir)
vendor="${DJS_STRIP_VENDOR:-$(dirname "$common")/vendor/strip-types}"
mkdir -p "$vendor"

if [ ! -f "$vendor/node/node_modules/ts-blank-space/package.json" ]; then
  mkdir -p "$vendor/node"
  cat > "$vendor/node/package.json" <<JSON
{ "private": true, "type": "module",
  "dependencies": { "ts-blank-space": "$BLANK_VERSION", "typescript": "$TS_VERSION" } }
JSON
  (cd "$vendor/node" && npm install --no-audit --no-fund --silent)
fi

if [ ! -d "$vendor/corpus/typescript-$TS_VERSION" ]; then
  tmp=$(mktemp -d)
  curl -fsSL "https://codeload.github.com/microsoft/TypeScript/tar.gz/refs/tags/v$TS_VERSION" \
    -o "$tmp/ts.tgz"
  mkdir -p "$vendor/corpus/typescript-$TS_VERSION"
  tar -xzf "$tmp/ts.tgz" -C "$vendor/corpus/typescript-$TS_VERSION" --strip-components=1 \
    "TypeScript-$TS_VERSION/src"
  rm -rf "$tmp"
fi
echo "$vendor"
