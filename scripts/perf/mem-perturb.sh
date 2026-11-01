#!/usr/bin/env bash
# mem-perturb.sh [-n N] [--step BYTES] [--root DIR] WASM workloads...
#
# Peak wasm memory of djs-wasm per workload as a distribution (D2048). Peak memory is a
# deterministic but chaotic function of where the host allocator (dlmalloc) places the
# arena when it grows: a shift of a few KB in the live heap can move a workload's peak by
# several MiB either way. One run per build proves nothing; compare distributions.
#
# Each workload (scripts/perf/w/<name>.js) runs N times (default 24), run k with a kept
# string of k * STEP bytes (default 1000) allocated first, once each (wasm-run.ts ONCE=1).
# Prints per workload: min, mean, max and the N values (MiB).
#
# --root DIR runs DIR's scripts/perf/wasm-run.ts (and its JS reader): for a build of
# another commit, whose heap layout this checkout's reader does not know. The workloads
# always come from this checkout.
set -euo pipefail
here=$(cd "$(dirname "$0")/../.." && pwd)
n=24 step=1000 root=$here
while [ $# -gt 0 ]; do
  case "$1" in
    -n) n=$2; shift 2 ;;
    --step) step=$2; shift 2 ;;
    --root) root=$(cd "$2" && pwd); shift 2 ;;
    *) break ;;
  esac
done
[ $# -ge 2 ] || { echo "usage: scripts/perf/mem-perturb.sh [-n N] [--step BYTES] [--root DIR] WASM workloads..." >&2; exit 2; }
wasm=$(cd "$(dirname "$1")" && pwd)/$(basename "$1"); shift
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for x in "$@"; do
  vals=""
  for k in $(seq 0 $((n - 1))); do
    f=$tmp/$x-$k.js
    { echo "globalThis.__pad = \"x\".repeat($((k * step)));"; cat "$here/scripts/perf/w/$x.js"; } > "$f"
    m=$(cd "$root" && DJS_WASM=$wasm ONCE=1 "$here/scripts/timeout.sh" 600 node scripts/perf/wasm-run.ts "$f" 2>&1 |
      tail -1 | sed 's/.*memory //; s/ MiB//')
    vals="$vals $m"
  done
  echo "$x $vals" | awk '{mn = 1e9; mx = 0; s = 0
    for (i = 2; i <= NF; i++) { s += $i; if ($i < mn) mn = $i; if ($i > mx) mx = $i }
    printf "%-14s min %5.1f  mean %6.2f  max %5.1f |", $1, mn, s / (NF - 1), mx
    for (i = 2; i <= NF; i++) printf " %s", $i; print ""}'
done
