#!/bin/sh
# Builds snaplab into the repo's target dir, extracts pi's prelude from
# tests/pi/prelude-source.ts and runs the cases. Usage: run.sh [out-dir] [case.js ...]
# (default: work/scratch/snapshot-research, all cases).
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
out=${1:-"$root/work/scratch/snapshot-research"}
[ $# -gt 0 ] && shift
mkdir -p "$out"
if [ $# -eq 0 ]; then set -- "$here"/cases/*.js; fi
node --input-type=module -e "
import { PRELUDE_SOURCE } from '$root/tests/pi/prelude-source.ts';
import { writeFileSync } from 'node:fs';
writeFileSync('$out/pi-prelude.js', PRELUDE_SOURCE);
"
(cd "$here" && CARGO_TARGET_DIR="$root/target" nice -n 10 cargo build -q && rm -f Cargo.lock)
"$root/scripts/timeout.sh" 900 "$root/target/debug/snaplab" "$out" "$out/pi-prelude.js" "$@"
