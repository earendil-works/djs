#!/usr/bin/env bash
# Builds QuickJS (the read-only ~/Development/dquickts sources, copied; never built in
# place) for size comparisons (0123, docs/size-vs-quickjs.md):
#   $OUT/qjs-Os.wasm    wasm32-wasi -Os, the core (no Temporal/Intl, no quickjs-libc),
#                       embed.c as the only entry point; with -g, so it has a name
#                       section and DWARF line tables (the code is the same as without)
#   $OUT/qjs-lines.txt  llvm-dwarfdump --debug-line of it (for scripts/size/areas.py c)
#   $OUT/bcsize         native: bytecode bytes of a compiled script
#                       (scripts/size/bytecode-density.py)
# Needs zig (wasi-libc) and homebrew llvm (llvm-dwarfdump). OUT: work/scratch/qjs-size.
set -euo pipefail
cd "$(dirname "$0")/../../.."
root=$(pwd)
src=${DQUICKTS:-$HOME/Development/dquickts}
out=${OUT:-$root/work/scratch/qjs-size}
mkdir -p "$out/src"
cp "$src"/*.c "$src"/*.h "$src"/VERSION "$out/src/"
cp "$root/scripts/size/qjs/embed.c" "$root/scripts/size/qjs/bcsize.c" "$out/src/"
cd "$out/src"
version=$(cat VERSION)
files=(quickjs.c libregexp.c libunicode.c cutils.c dtoa.c)
zig cc -target wasm32-wasi -Os -g -DCONFIG_VERSION="\"$version\"" -D_GNU_SOURCE -fwrapv \
  -ffunction-sections -fdata-sections -mexec-model=reactor -Wl,--gc-sections \
  embed.c "${files[@]}" -o "$out/qjs-Os.wasm"
"${LLVM_DWARFDUMP:-/opt/homebrew/opt/llvm/bin/llvm-dwarfdump}" --debug-line "$out/qjs-Os.wasm" > "$out/qjs-lines.txt"
clang -O1 -w -DCONFIG_VERSION="\"$version\"" -D_GNU_SOURCE -fwrapv bcsize.c "${files[@]}" -lm -o "$out/bcsize"
ls -l "$out/qjs-Os.wasm" "$out/bcsize"
