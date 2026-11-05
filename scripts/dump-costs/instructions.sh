#!/usr/bin/env bash
# Instructions retired by dumping, loading and re-dumping (the loaded VM's dump) at every
# idle point of the native dump benchmark (crates/djs-core/src/vm/dump_perf.rs), one of
# each per point: stable under machine load, where timings are not (0198, 0215). macOS
# (`/usr/bin/time -l`).
#
#   scripts/dump-costs/instructions.sh [corpus|cases|all] [filter]
set -euo pipefail
cd "$(dirname "$0")/../.."
mode=${1:-all}
filter=${2:-}
bin=$(scripts/timeout.sh 900 cargo test --release -p djs-core --lib vm::dump_perf --no-run 2>&1 |
  grep -o 'target/release/deps/djs_core-[a-z0-9]*' | head -1)
count() {
  (cd crates/djs-core && env DJS_DUMP_PERF="$mode" DJS_DUMP_PERF_FILTER="$filter" DJS_DUMP_PERF_REPS=1 "$@" \
    /usr/bin/time -l "../../$bin" vm::dump_perf --ignored 2>&1 | awk '/instructions retired/ {print $1}')
}
none=$(count DJS_DUMP_PERF_SKIP=all)
dump=$(count DJS_DUMP_PERF_SKIP=load)
load=$(count DJS_DUMP_PERF_SKIP=redump)
full=$(count DJS_DUMP_PERF_SKIP=)
echo "dump: $(( (dump - none) / 1000000 )) M instructions; load: $(( (load - dump) / 1000000 )) M; re-dump: $(( (full - load) / 1000000 )) M ($mode $filter)"
