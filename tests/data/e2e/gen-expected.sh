#!/usr/bin/env bash
# Regenerates the .expected file of every case from QuickJS (the oracle).
#
# Each case runs strict, with a console.log that formats values the way `djs run` does
# (vm::display): strings raw, Errors as `Name: message`, -0/NaN/Infinity as String(),
# undefined, functions as `function`, everything else JSON.stringify. An uncaught
# exception adds a last line `Uncaught <formatted>`.
#
# A case whose first line is `// mode: body` runs as a codemode body,
# `(async (tools, console) => { <case> })`, otherwise as a Script.
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
  const j = JSON.stringify(v);
  return j === undefined ? String(v) : j;
}
var console = { log: (...a) => print(a.map(__fmt).join(" ")) };'
for f in *.js; do
  tmp=$(mktemp)
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
