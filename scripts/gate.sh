#!/usr/bin/env bash
# The merge gate. Must pass on a branch before it is merged into main.
# Keep this fast; extend it as the project grows (test262 ratchet, difftests, ...).
set -euo pipefail
cd "$(dirname "$0")/.."

# Full width for the gate (only one runs at a time, under the merge lock); the repo's
# .cargo/config.toml caps everything else at 6 (D012).
ncpu=$(sysctl -n hw.ncpu 2>/dev/null || nproc)
export CARGO_BUILD_JOBS="$ncpu" RUST_TEST_THREADS="$ncpu" DJS_JOBS="$ncpu"
# Not niced by scripts/timeout.sh: everything else yields to the gate (D1793).
export DJS_GATE=1

# Each step's wall and CPU seconds (children, from `times`) go to target/gate-steps.tsv,
# one line per step plus a total, for measuring the gate (0079).
mkdir -p target
step_log=target/gate-steps.tsv
: > "$step_log"
gate_start=$(date +%s)
step_name="" step_t=$gate_start step_c=0
# `times` must run in this shell (not in a $(...) subshell) to see the children's time.
cpu() { times > target/.gate-times
  cpu_s=$(tail -1 target/.gate-times | awk '{gsub("m"," ");gsub("s",""); print $1*60+$2+$3*60+$4}'); }
step() {
  local now c; now=$(date +%s); cpu; c=$cpu_s
  if [ -n "$step_name" ]; then
    printf '%s\t%s\t%s\n' "$step_name" $((now - step_t)) "$(echo "$c - $step_c" | bc)" >> "$step_log"
  fi
  step_name="$1" step_t=$now step_c=$c
  [ "$1" = END ] || echo "== $1"
}

# D020: the shipped module for the size budget (last step). Its build (LTO, one codegen
# unit: mostly one core for a few minutes) runs in its own target directory beside the
# other steps.
size_log=target/size-build.log
( CARGO_TARGET_DIR=target/size scripts/with-wasm-env.sh cargo build -p djs-wasm --target wasm32-unknown-unknown \
    --profile wasm --quiet > "$size_log" 2>&1 ) &
size_pid=$!
# D2411: the slim module (scripts/build-wasm.sh --slim), beside it for the same reason.
slim_log=target/size-slim-build.log
( CARGO_TARGET_DIR=target/size-slim scripts/with-wasm-env.sh cargo build -p djs-wasm --target wasm32-unknown-unknown \
    --profile wasm --no-default-features --quiet > "$slim_log" 2>&1 ) &
slim_pid=$!
trap 'kill "$size_pid" "$slim_pid" 2>/dev/null || true' EXIT

step "fmt";   cargo fmt --all --check
step "clippy"; cargo clippy --workspace --all-targets --quiet -- -D warnings
step "test"
# Through scripts/test-runner.sh, which records each test's time (0079): the slowest are
# printed below and kept in target/gate-slowest-tests.tsv, so a slow new test shows at
# once; merge-to-main collects them in work/scratch/slow-tests.tsv.
host=$(rustc -vV | sed -n 's/^host: //p' | tr 'a-z-' 'A-Z_')
: > target/test-times.tsv
env "CARGO_TARGET_${host}_RUNNER=$PWD/scripts/test-runner.sh" \
  DJS_TEST_TIMES="$PWD/target/test-times.tsv" cargo test --workspace --quiet
echo "slowest tests:"
sort -rn target/test-times.tsv | head -10 | tee target/gate-slowest-tests.tsv |
  awk -F'\t' '{printf "  %7.2fs  %s  %s\n", $1, $2, $3}'
step "no_std/wasm build of djs-core"
cargo build -p djs-core --target wasm32-unknown-unknown --quiet
# Clippy for wasm32 in release (no debug_assertions), so warnings that only appear in
# that configuration fail the gate too. With djs-wasm's docs-only `docs` feature
# (`djs_disasm`, 0113), so the docs site's module keeps compiling;
# the shipping build leaves it off.
step "clippy (wasm32, release)"
cargo clippy -p djs-core -p djs-wasm --features djs-wasm/docs --target wasm32-unknown-unknown --release --quiet -- -D warnings

# Additional gate steps registered by issues go below this line.
step "builtins/gen is up to date (0017)"
./scripts/bgen.sh --check
# 0103: the guide's generated regions and diagrams are fresh, its examples print what
# they printed, its links and decision IDs resolve (D2100).
step "builtin registry is append-only (D2371)"; python3 scripts/registry.py check main
step "guide (docs/guide)"; cargo run --quiet -p djs-docs -- check

# 0018: the host-side JS package (node >= 22.6 for type stripping), and the wire format
# across languages: node encodes a corpus, djs decodes it.
# 0023: djs-wasm, which the js tests (sandbox.test.ts) load. A debug build is enough.
step "djs-wasm"
cargo build -p djs-wasm --target wasm32-unknown-unknown --features compiled-prelude,dump --quiet
export DJS_WASM="$PWD/target/wasm32-unknown-unknown/debug/djs_wasm.wasm"
# 0175: the same build without the dumper, the main module of packaging B (D2635):
# js/test/dumpresume.test.ts moves images between it and the module above. Its own target
# directory, so the two modules do not overwrite each other.
CARGO_TARGET_DIR=target/wasm-nodump cargo build -p djs-wasm --target wasm32-unknown-unknown --features compiled-prelude --quiet
export DJS_WASM_NODUMP="$PWD/target/wasm-nodump/wasm32-unknown-unknown/debug/djs_wasm.wasm"
step "js tests"; node --test js/test/*.test.ts
# 0072: native and wasm builds leave byte-identical heap images and spend the same fuel.
step "native vs wasm images"; ./scripts/determinism/native-vs-wasm.sh "$DJS_WASM"
step "wire cross-language"
wire_dir=$(mktemp -d)
node js/test/gen-cross.ts "$wire_dir" > /dev/null
DJS_WIRE_CROSS="$wire_dir" cargo test -p djs-core --quiet wire_cross -- --ignored
# 0021: the outbound reader, on heap images of the same corpus plus hand-built cases.
step "heap reader cross-language"
heap_dir=$(mktemp -d)
DJS_WIRE_CROSS="$wire_dir" DJS_READER_DIR="$heap_dir" \
  cargo test -p djs-core --quiet reader_images -- --ignored
node js/test/reader-cross.ts "$wire_dir" "$heap_dir"
rm -rf "$wire_dir" "$heap_dir"
step "difftest on djs: ratchet, plus suspension and GC stress (0016), heap images (0072), a dump at every idle point (0172)"
cargo run --quiet -p djs-difftest -- --engine djs --ratchet --suspend-every 97 --gc-stress --determinism --dump-every-idle --quiet --no-diff > target/difftest-djs.log \
  || { cat target/difftest-djs.log; exit 1; }
grep -E '^(stress|ratchet):' target/difftest-djs.log
# 0063: the same corpus with the @native builtins as bytecode; both backends must pass.
step "difftest on djs, bytecode backend (0063)"
cargo run --quiet -p djs-difftest -- --engine djs --backend bytecode --ratchet --quiet --no-diff > target/difftest-djs-bytecode.log \
  || { cat target/difftest-djs-bytecode.log; exit 1; }
grep -E '^ratchet:' target/difftest-djs-bytecode.log
# 0161: the unchecked accessor build (D2520). The verifier against corrupt images and e2e
# (debug: the accessors assert their bounds), then the corpus in release, where they
# really do not check, with suspension and GC stress.
step "unsafe-heap build (D2520)"
cargo test -p djs-core --features unsafe-heap --quiet --test verify_fuzz --test e2e
cargo run --quiet --release -p djs-difftest --features djs-core/unsafe-heap -- --engine djs --ratchet \
  --suspend-every 97 --gc-stress --quiet --no-diff > target/difftest-unsafe.log \
  || { cat target/difftest-unsafe.log; exit 1; }
grep -E '^(stress|ratchet):' target/difftest-unsafe.log
step "test262 on djs: the active group against the ratchet (0007)"
./scripts/fetch-test262.sh > /dev/null
cargo run --quiet -p djs-test262 -- run --engine djs --group active --quiet --slowest 10 \
  --expected tests/test262/expected-failures.txt > target/test262-djs.log \
  || { cat target/test262-djs.log; exit 1; }
grep -E '^ratchet:|^\| active' target/test262-djs.log
echo "slowest test262 tests:"
sed -n '/^slowest:/,/^$/p' target/test262-djs.log | { grep -E '^ +[0-9]' || true; } |
  tee target/gate-slowest-test262.txt
# D2411: the slim build (no default features: no `full`, no `unicode`), natively in its own
# target directory: its djs-core and djs-prelude tests that hold in both builds, the
# codemode corpora with the stress modes, test262 without the families it leaves out
# (tests/test262-slim.toml) against its own ratchet, and pi's tests on the slim module with
# pi's prelude minus its lockdown, on frozen builtins.
step "slim build (D2411): unit tests, codemode corpora, test262"
CARGO_TARGET_DIR=target/slim cargo test -p djs-core --no-default-features --lib --quiet \
  -- vm::slim_tests vm::frozen_tests vm::registry
CARGO_TARGET_DIR=target/slim cargo test -p djs-prelude --no-default-features --features raw,registry --quiet
CARGO_TARGET_DIR=target/slim cargo run --quiet -p djs-difftest --no-default-features -- --engine djs --ratchet \
  --suspend-every 97 --gc-stress --determinism --quiet --no-diff codemode codemode-harvest > target/difftest-slim.log \
  || { cat target/difftest-slim.log; exit 1; }
grep -E '^(stress|ratchet):' target/difftest-slim.log
CARGO_TARGET_DIR=target/slim cargo run --quiet -p djs-test262 --no-default-features -- run --engine djs \
  --group active --quiet --expected tests/test262/expected-failures-slim.txt > target/test262-slim.log \
  || { cat target/test262-slim.log; exit 1; }
grep -E '^ratchet:|^\| active' target/test262-slim.log
step "slim module (D2411): pi's tests without lockdown"
wait "$slim_pid" || { cat "$slim_log"; exit 1; }
slim_wasm=target/size-slim/wasm32-unknown-unknown/wasm/djs_wasm.wasm
DJS_WASM="$PWD/$slim_wasm" node js/pi/pi-tests.ts --no-lockdown --out target/pi-codemode-slim \
  --expected tests/pi/slim-expected-failures.txt > target/pi-slim.log 2>&1 \
  || { cat target/pi-slim.log; exit 1; }
grep -E '^pi tests:' target/pi-slim.log
step "size budget (D020)"
wait "$size_pid" || { cat "$size_log"; exit 1; }
python3 scripts/size/budget.py target/size/wasm32-unknown-unknown/wasm/djs_wasm.wasm
python3 scripts/size/budget.py "$slim_wasm" --slim
# 0176: pi's tests in the adapter's next mode (prelude-next, D2650) on the release module.
step "pi's tests in the next mode (D2650)"
DJS_WASM="$PWD/target/size/wasm32-unknown-unknown/wasm/djs_wasm.wasm" node js/pi/pi-tests.ts --next \
  --out target/pi-codemode-next --expected tests/pi/expected-failures.txt > target/pi-next.log 2>&1 \
  || { cat target/pi-next.log; exit 1; }
grep -E '^pi tests:' target/pi-next.log
step END
cpu; printf 'total\t%s\t%s\n' $(($(date +%s) - gate_start)) "$cpu_s" >> "$step_log"
echo "gate: OK"
