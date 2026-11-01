#!/usr/bin/env bash
# Builds a variant of the shipped wasm module in a scratch copy of the tree, for size
# experiments that must never land (0123): `scripts/size/variant.sh NAME [PATCH.py] [--
# CARGO ARGS...]`, with the build environment (RUSTFLAGS, CARGO_PROFILE_WASM_*, a
# `+toolchain` as the first cargo arg) taken from the caller. PATCH.py, if given, runs
# in the copy's root and edits sources. Prints the code/data sizes and gzip of the result,
# which is kept as $OUT/NAME.wasm (OUT: work/scratch/size-variants, gitignored).
set -euo pipefail
cd "$(dirname "$0")/../.."
root=$(pwd)
name=$1
shift
patch=""
if [ $# -gt 0 ] && [ "$1" != "--" ]; then
  patch=$1
  shift
fi
[ "${1:-}" = "--" ] && shift
out=${OUT:-$root/work/scratch/size-variants}
src=$out/$name/src
mkdir -p "$src"
rsync -a --delete --exclude target --exclude work --exclude vendor --exclude node_modules \
  --exclude .git --exclude .worktrees "$root/" "$src/"
if [ -n "$patch" ]; then
  (cd "$src" && python3 "$root/$patch")
fi
toolchain=()
if [ $# -gt 0 ] && [[ "$1" == +* ]]; then
  toolchain=("$1")
  shift
fi
(cd "$src" && CARGO_TARGET_DIR="$out/$name/target" nice -n 10 cargo ${toolchain[@]+"${toolchain[@]}"} \
  build -q -p djs-wasm --target wasm32-unknown-unknown --profile wasm "$@")
cp "$out/$name/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm" "$out/$name.wasm"
python3 - "$out/$name.wasm" "$name" <<'EOF'
import gzip, sys
b = open(sys.argv[1], "rb").read()
def leb(i):
    r = s = 0
    while True:
        x = b[i]; i += 1; r |= (x & 0x7F) << s; s += 7
        if x < 0x80: return r, i
i, sec, strip = 8, {}, b[:8]
while i < len(b):
    sid = b[i]; size, j = leb(i + 1)
    if sid != 0: strip += b[i:j + size]
    sec[sid] = b[j:j + size]
    i = j + size
gz = lambda d: len(gzip.compress(d, 9))
c, d = sec.get(10, b""), sec.get(11, b"")
print(f"{sys.argv[2]}: module {len(strip):,} gzip {gz(strip):,} | code {len(c):,} gzip {gz(c):,} | data {len(d):,} gzip {gz(d):,}")
EOF
