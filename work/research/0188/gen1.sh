#!/bin/bash
# gen1.sh prog.js outdir: dumps at every idle point of a --body program
D=${DJS:-../../../target/debug/djs}; T=../../../scripts/timeout.sh
p=$1; o=$2; n=$(basename $p .js)
$T 30 $D run --body $p > $o/$n.out 2>&1
k=1
while [ $k -le 40 ] && $T 30 $D run --body --dump-after $k $o/$n.$k.dump.js $p >/dev/null 2>&1 && [ -f $o/$n.$k.dump.js ]; do k=$((k+1)); done
