#!/usr/bin/env bash
# Regenerates the .expected file of every case from QuickJS (the oracle).
#
# Each case runs strict, with a console.log that formats values the way `djs run` does
# (vm::display): strings raw, Errors as `Name: message`, -0/NaN/Infinity as String(),
# BigInts as `10n`,
# undefined, functions as `function`, everything else JSON.stringify. An uncaught
# exception adds a last line `Uncaught <formatted>`.
#
# A case whose first line is `// mode: body` runs as a codemode body,
# `(async (tools, console) => { <case> })`; one whose first line is `// mode: module` is a
# module graph: the entry module main.js, then a module per `// file: <name>` line (the
# e2e test's loader strips a leading `./` from specifiers), imported dynamically after
# the console is set up; otherwise it runs as a Script.
# CASES="a.js b.js" regenerates only those cases.
set -euo pipefail
cd "$(dirname "$0")"
QJS="${QJS:-$HOME/Development/dquickts/qjs}"
# Local time is UTC, as in djs without a host offset (D1383).
export TZ=UTC
PRELUDE='function __fmt(v) {
  if (typeof v === "string") return v;
  if (v instanceof Error) return v.message ? v.name + ": " + v.message : v.name;
  if (v === undefined) return "undefined";
  if (typeof v === "number" && (Object.is(v, -0) || !isFinite(v))) return Object.is(v, -0) ? "-0" : String(v);
  if (typeof v === "function") return "function";
  if (typeof v === "symbol") return "Symbol(" + (v.description ?? "") + ")";
  if (typeof v === "bigint") return v + "n";
  const j = JSON.stringify(v);
  return j === undefined ? String(v) : j;
}
var console = { log: (...a) => print(a.map(__fmt).join(" ")) };'
for f in ${CASES:-*.js}; do
  tmp=$(mktemp)
  if head -1 "$f" | grep -q '^// mode: module'; then
    dir=$(mktemp -d)
    awk -v dir="$dir" 'BEGIN { out = dir "/main.js" }
      /^\/\/ file: / { out = dir "/" $3; next }
      { print > out }' "$f"
    { echo "$PRELUDE" | sed 's/^var console = /globalThis.console = /; s/^function __fmt/globalThis.__fmt = function/'
      echo 'import("./main.js").catch((e) => print("Uncaught " + __fmt(e)));'; } > "$dir/run.js"
    "$QJS" -m "$dir/run.js" > "${f%.js}.expected" 2>&1 || true
    rm -rf "$dir" "$tmp"
    continue
  fi
  if head -1 "$f" | grep -q '^// mode: body'; then
    { echo '"use strict";'; echo "$PRELUDE"
      echo '(async (tools, console) => {'; cat "$f"; echo
      echo '})(undefined, console).catch((e) => print("Uncaught " + __fmt(e)));'; } > "$tmp"
  else
    { echo '"use strict";'; echo "$PRELUDE"; cat "$f"; } > "$tmp"
  fi
  # qjs reports an uncaught exception on stderr as `Name: message` plus stack lines.
  if ! "$QJS" "$tmp" > "${f%.js}.expected" 2> "$tmp.err"; then
    echo "Uncaught $(head -1 "$tmp.err")" >> "${f%.js}.expected"
  fi
  rm -f "$tmp" "$tmp.err"
done
ls *.expected | wc -l
