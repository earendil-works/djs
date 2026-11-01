#!/usr/bin/env bash
# trace.sh <case.js> <skip> <n> <out>: exact instruction trace of the native build (see
# trace.py; DJS overrides the binary, TRACE_BOUNDARY the fuel-check address of the build).
tools="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$tools/../.." && pwd)"
out="${DJS_PERF_DIR:-$root/work/scratch/perf}"
TRACE_SKIP=$2 TRACE_N=$3 TRACE_OUT=$4 "$root/scripts/timeout.sh" 1200 lldb -b -o "command script import $tools/trace.py" -o 'trace_run' -- "${DJS:-$out/target/release/djs}" run --body "$1" > /dev/null 2>&1
