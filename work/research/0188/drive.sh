#!/bin/bash
# For every dump in today/: residualize, resume both, compare the continued output.
D=${DJS:-../../../target/debug/djs}
T=../../../scripts/timeout.sh
mkdir -p resid/${DIR:-today}
ok=0; diff=0; ref=0
for d in ${DIR:-today}/*.dump.js; do
  b=$(basename $d)
  if ! node residualize.mjs $d > resid/${DIR:-today}/$b 2> resid/${DIR:-today}/$b.log; then
    echo "REFUSED $b: $(grep refused: resid/${DIR:-today}/$b.log | head -1)$(grep -m1 Error resid/${DIR:-today}/$b.log)"; ref=$((ref+1)); rm resid/${DIR:-today}/$b; continue
  fi
  a=$($T 30 $D resume $d 2>&1 | grep -v "^    at "); r=$($T 30 $D resume resid/${DIR:-today}/$b 2>&1 | grep -v "^    at ")
  if [ "$a" == "$r" ]; then ok=$((ok+1)); else diff=$((diff+1)); echo "DIFF $b"; echo "  today: $a" | head -3; echo "  resid: $r" | head -3; fi
done
echo "ok $ok diff $diff refused $ref"
