#!/usr/bin/env bash
# Run the djs benchmarks (crates/djs-bench) in a release build.
#
#   scripts/bench.sh [filter...]          run and print the results
#   scripts/bench.sh --record [filter...] also append them to tests/scores/bench.tsv
#   scripts/bench.sh --compare [REV]      compare with the last recorded run on this
#                                         machine, or with the run of commit REV
#   scripts/bench.sh --quick              every benchmark once (smoke test)
#
# Fetches the lexer input (typescript.js, pinned) into vendor/bench of the primary
# checkout on first use; without network that benchmark is skipped.
set -euo pipefail
cd "$(dirname "$0")/.."

TS_VERSION=5.9.3
TS_SHA256=10e108c9cf7d5f2879053dff18515fb405abf2ccef63eaaf017d9c571687a1d3
common=$(git rev-parse --path-format=absolute --git-common-dir)
vendor="${DJS_BENCH_VENDOR:-$(dirname "$common")/vendor/bench}"
ts="$vendor/typescript-$TS_VERSION.js"
if [ ! -f "$ts" ]; then
  mkdir -p "$vendor"
  tmp=$(mktemp -d)
  if curl -fsSL "https://registry.npmjs.org/typescript/-/typescript-$TS_VERSION.tgz" -o "$tmp/ts.tgz"; then
    got=$(shasum -a 256 "$tmp/ts.tgz" | cut -d' ' -f1)
    if [ "$got" = "$TS_SHA256" ]; then
      tar -xzf "$tmp/ts.tgz" -C "$tmp" package/lib/typescript.js
      mv "$tmp/package/lib/typescript.js" "$ts"
    else
      echo "bench.sh: typescript-$TS_VERSION.tgz has sha256 $got, expected $TS_SHA256" >&2
    fi
  else
    echo "bench.sh: could not fetch typescript $TS_VERSION; lex/typescript.js will be skipped" >&2
  fi
  rm -rf "$tmp"
fi

# `--compare REV` becomes `--compare=REV` when REV names a commit.
args=()
while [ $# -gt 0 ]; do
  if [ "$1" = "--compare" ] && [ $# -gt 1 ] && git rev-parse --verify --quiet "$2^{commit}" >/dev/null; then
    args+=("--compare=$(git rev-parse --short=10 "$2")")
    shift 2
  else
    args+=("$1")
    shift
  fi
done

export DJS_BENCH_VENDOR="$vendor"
exec cargo run --quiet --release -p djs-bench -- ${args[@]+"${args[@]}"}
