#!/bin/sh
# check-djs.sh A B: runs every dump under A and the same path under B with `djs resume`
# (the v2 and sleep shapes: later/print/tools hosts) and compares their output. The
# rewritten ones get a shim for the minimal format's positional `$djs.k(K, f, at, proto)`
# after their header (the CLI's runtime is main's).
djs=${DJS:-target/debug/djs}
shim='{ const k0 = $djs.k, gp = [Object.getPrototypeOf(function* () {}), Object.getPrototypeOf(async function* () {})]; $djs.k = (K, f, at, p) => k0(K, { f, at }, p ?? (gp.includes(Object.getPrototypeOf(K)) ? f.prototype : undefined)); }'
n=0; bad=0
for a in $(cd "$1" && find . -name '*.js' | sort); do
  n=$((n+1))
  x=$(scripts/timeout.sh 30 $djs resume "$1/$a" 2>&1)
  awk -v s="$shim" 'NR==3{print s} {print}' "$2/$a" > /tmp/0216-shim.js
  y=$(scripts/timeout.sh 30 $djs resume /tmp/0216-shim.js 2>&1 | sed 's|/tmp/0216-shim.js|'"$1/$a"'|g')
  if [ "$x" != "$y" ]; then bad=$((bad+1)); echo "DIFF $a"; echo "$x" | head -5; echo "---"; echo "$y" | head -5; fi
done
echo "djs: $n dumps, $bad differ"
