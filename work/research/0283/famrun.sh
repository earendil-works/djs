#!/bin/bash
# famrun.sh NAME FILE...: the shipped module without FILE... in its prelude: sizes, idle arena and the codemode rows.
cd /Users/mitsuhiko/Development/djs/work/scratch/0283
./famwasm.sh "$@" > /dev/null 2>&1 || { echo "$1 build failed"; exit 1; }
w=w-$1.wasm
gz=$(gzip -9c $w | wc -c | tr -d ' ')
raw=$(stat -f %z $w)
arena=$(node idle.ts $w 2>&1 | awk '/^arena len/ {print $3}')
rows=$(node rows.ts $PWD/$w 2>&1 | grep -v -i warn | awk '{printf "%s:%s ", $1, $2}')
echo "$1 raw $raw gz $gz idle-arena $arena | $rows"
