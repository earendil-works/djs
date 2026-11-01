#!/usr/bin/env bash
# Checks that nothing on the guide's site sticks out of its column (0114): builds the site,
# serves it, and loads every page in headless Chrome at a desktop, a tablet and a phone
# width (crates/djs-docs/tools/overflow.html). Code blocks and wide tables may scroll
# inside their own box; anything else past the article's edge is reported and fails.
#   scripts/docs-overflow.sh [-v]     -v also lists the blocks that scroll
# Needs Google Chrome (or chromium/google-chrome on PATH). Not part of the gate.
set -euo pipefail
cd "$(dirname "$0")/.."
verbose=""; [ "${1:-}" = -v ] && verbose="&v"
chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
[ -x "$chrome" ] || chrome=$(command -v google-chrome || command -v chromium || true)
[ -n "$chrome" ] || { echo "docs-overflow.sh: no Chrome found" >&2; exit 2; }
site=target/docs-overflow
cargo run -q -p djs-docs -- build --out "$site" >/dev/null
cp crates/djs-docs/tools/overflow.html "$site/"
port=$((8700 + RANDOM % 200))
python3 -m http.server -d "$site" "$port" >/dev/null 2>&1 &
server=$!
trap 'kill $server 2>/dev/null' EXIT
sleep 1
bad=0
for w in 1440 1000 390; do
  out=$("$chrome" --headless=new --disable-gpu --virtual-time-budget=60000 --window-size=1600,1000 \
    --dump-dom "http://localhost:$port/overflow.html?w=$w$verbose" 2>/dev/null \
    | python3 -c 'import html, re, sys; m = re.search(r"<pre id=\"out\">(.*?)</pre>", sys.stdin.read(), re.S); print(html.unescape(m.group(1)) if m else "")')
  echo "width $w: ${out:-(no result)}"
  if [ -z "$out" ] || echo "$out" | grep -v '^  (scrolls)' | grep -qv 'no overflow'; then bad=1; fi
done
exit $bad
