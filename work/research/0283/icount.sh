#!/bin/bash
# icount.sh A.wasm B.wasm REPS N workloads...: min over N interleaved runs of instructions retired (node, REPS runs) minus REPS=0.
A=$1; B=$2; R=$3; N=$4; shift 4
ins() { /usr/bin/time -l node --wasm-sync-tier-up icount.ts $1 $2 $3 2>&1 >/dev/null | awk '/instructions retired/ {print $1}'; }
for w in "$@"; do
  a=999999999999999; b=$a; a0=$a; b0=$a
  for i in $(seq $N); do
    x=$(ins $A $w $R); [ $x -lt $a ] && a=$x
    x=$(ins $B $w $R); [ $x -lt $b ] && b=$x
    x=$(ins $A $w 0); [ $x -lt $a0 ] && a0=$x
    x=$(ins $B $w 0); [ $x -lt $b0 ] && b0=$x
  done
  python3 -c "a=($a-$a0)/$R; b=($b-$b0)/$R; print(f'$w A {a/1e6:.3f}M B {b/1e6:.3f}M B/A {b/a:.4f}')"
done
