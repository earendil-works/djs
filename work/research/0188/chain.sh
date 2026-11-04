#!/bin/bash
# chain.sh prog.js [--body]: dump at idle point 1, then residualize, resume to the next
# idle point, dump again (today's exporter on a VM running continuations), residualize...
# The concatenated output must equal the uninterrupted run's.
D=${DJS:-../../../target/debug/djs}; T=../../../scripts/timeout.sh
p=$1; body=$2; n=$(basename $p .js); w=chain/$n; rm -rf $w; mkdir -p $w
full=$($T 30 $D run $body $p 2>&1 | grep -v "^    at ")
out=$($T 30 $D run $body --dump-after 1 $w/d1.js $p 2>&1 | grep -v "^    at ")
[ -f $w/d1.js ] || { echo "$n: no idle point"; exit 0; }
k=1; maxlen=0
# Idle point 1 of a resumed run is the loaded state itself (dumping there writes the same
# dump again: a fixed point, which looped forever once), so each link dumps at idle point 2.
while [ $k -le 50 ]; do
  node residualize.mjs $w/d$k.js > $w/r$k.js 2>$w/r$k.log || { echo "$n: refused at link $k: $(grep refused: $w/r$k.log)"; exit 0; }
  s=$(wc -c < $w/r$k.js); [ $s -gt $maxlen ] && maxlen=$s
  seg=$($T 30 $D resume --dump-after 2 $w/d$((k+1)).js $w/r$k.js 2>&1 | grep -v "^    at " | grep -v "^djs: --dump-after 2: the run ended")
  out="$out${out:+${seg:+$'\n'}}$seg"
  [ -f $w/d$((k+1)).js ] || break
  k=$((k+1))
done
if [ "$out" == "$full" ]; then echo "$n: ok, $k links, largest residual $maxlen bytes"; else echo "$n: DIFF after $k links"; diff <(echo "$full") <(echo "$out") | head -5; fi
