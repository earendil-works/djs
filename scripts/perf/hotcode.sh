#!/bin/sh
# hotcode.sh NAME [WORKLOAD]: what V8 makes of the checkout's `run_hot` (0268, D3391).
# Builds the shipped module with function names into $DJS_PERF_DIR/hot/NAME-names.wasm,
# runs WORKLOAD (default tight) under node's default tiering and writes
#   hot-NAME.dis   run_hot's wasm (wasm-objdump)
#   code-NAME.txt  V8's code of run_hot (Liftoff first, TurboFan last)
#   sum-NAME.txt   v8arm.py's summary: dispatch, back edge, the path of every opcode
# and prints the summary's head. Compare two checkouts (or a change and its base) with
#   diff sum-A.txt sum-B.txt
# The names build runs the same code as the shipped one (only the name section differs),
# so slope.py and wmeasure.py can measure it.
set -e
name=${1:?usage: hotcode.sh NAME [WORKLOAD]}
work=${2:-tight}
tools=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$tools/../.." && pwd)
out=${DJS_PERF_DIR:-$root/work/scratch/perf}/hot
mkdir -p "$out"
cd "$root"
CARGO_TARGET_DIR=target/names-wasm CARGO_PROFILE_WASM_STRIP=false nice -n 10 scripts/with-wasm-env.sh \
  cargo build -p djs-wasm --target wasm32-unknown-unknown --profile wasm --quiet --features unsafe-heap
w=$out/$name-names.wasm
cp target/names-wasm/wasm32-unknown-unknown/wasm/djs_wasm.wasm "$w"
line=$(wasm-objdump -x -j Code "$w" | grep "Exec7run_hot>")
i=$(echo "$line" | sed 's/.*func\[\([0-9]*\)\].*/\1/')
size=$(echo "$line" | sed 's/.*size=\([0-9]*\).*/\1/')
wasm-objdump -d "$w" | awk -v a="func[$i]" -v b="func[$((i + 1))]" 'index($0,a" "){p=1} index($0,b" "){p=0} p' > "$out/hot-$name.dis"
DJS_WASM=$w ONCE=1 scripts/timeout.sh 300 node --print-wasm-code-function-index="$i" scripts/perf/wasm-run.ts "$work" > "$out/code-$name.txt" 2>&1
python3 "$tools/v8arm.py" "$out/code-$name.txt" > "$out/sum-$name.txt"
echo "$name: run_hot is wasm function $i, $size bytes, $(grep -c ' call ' "$out/hot-$name.dis") calls; $w"
awk '/^back-edge/{f=1} !/^   /&&!/^back-edge/&&f{exit} {print}' "$out/sum-$name.txt"
grep -E "^(Drop|GetLoc0|PutLoc0|Add|Lt|IncLoc|JumpIfTrue8|GetField|GetGlobal|Call0|Return) " "$out/sum-$name.txt"
