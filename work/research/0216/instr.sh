#!/bin/sh
# instr.sh A B: instructions retired by `djs resume` (release) of every dump under A and the
# same path under B (B's with the positional `$djs.k` shim), summed; the dumps' runs to their
# end are the same work, so the difference is the load's. macOS `/usr/bin/time -l`.
djs=target/release/djs
shim='{ const k0 = $djs.k; $djs.k = (K, f, at, p) => k0(K, { f, at }, p); }'
count() { /usr/bin/time -l "$djs" resume "$1" 2>&1 >/dev/null | awk '/instructions retired/ {print $1}'; }
ta=0; tb=0; n=0
for a in $(cd "$1" && find . -name '*.js' | sort); do
  awk -v s="$shim" 'NR==3{print s} {print}' "$2/$a" > /tmp/0216-i.js
  x=$(count "$1/$a"); y=$(count /tmp/0216-i.js)
  ta=$((ta + x)); tb=$((tb + y)); n=$((n+1))
  [ -n "$VERBOSE" ] && echo "$a $x $y"
done
echo "$n dumps: A $((ta / 1000)) K instructions, B $((tb / 1000)) K ($(( (tb - ta) * 100 / ta ))%)"
