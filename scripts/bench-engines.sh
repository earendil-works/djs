#!/usr/bin/env bash
# bench-engines.sh: the engine comparison of docs/guide/benchmarks.md. Fetches the engines
# (pinned versions, checked by sha256) into vendor/engines of the primary checkout, builds
# the QuickJS releases with wasi-sdk and djs with scripts/build-wasm.sh, then runs
# js/bench/engines/run.ts over the codemode workload set (tests/bench/codemode).
#
#   scripts/bench-engines.sh [run.ts options]   fetch, build, run and print the tables
#   scripts/bench-engines.sh --setup            fetch and build only
#   scripts/status.py engines                   run it and record the numbers
#
# Bench on a quiet machine (load below ~4); run.ts prints the load it started at.
set -euo pipefail
cd "$(dirname "$0")/.."
root=$PWD
common=$(git rev-parse --path-format=absolute --git-common-dir)
V="${DJS_ENGINES:-$(dirname "$common")/vendor/engines}"
mkdir -p "$V/dl" "$V/build"

WASI_SDK=34.0
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) sdk_os=arm64-macos; sdk_sha=9c59398106b417f8f14913380fdf0097a8cc0ff4af9eb3ce0065a859e88d49e9 ;;
  Darwin-x86_64) sdk_os=x86_64-macos; sdk_sha=87d27fa8adc68dee59bfbf2e22a6d34ef717c34d6bf1d8af2a56fc929d9ce0eb ;;
  Linux-x86_64) sdk_os=x86_64-linux; sdk_sha=b761e3a0721dbae9c09a0059e5fdb2bf917d1b4a8a7b430fb3b5aafb0984b2c4 ;;
  Linux-aarch64) sdk_os=arm64-linux; sdk_sha=f7e243dff54d60bcc576e94d6166b69f410f2500ae4a9ceef34315be10e77971 ;;
  *) echo "bench-engines.sh: no wasi-sdk for $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac
QJS=2026-06-04
QJS_SHA=b376e839b322978313d929fd20663b11ba58b75df5a46c126dd19ea2fa70ad2a
NG=0.18.0
NG_SHA=6c05066a6b62b483e2ba0f83d7a16688c6958182b214bfd693529434cef84a9e

fetch() { # url file sha256
  local f="$V/dl/$2"
  if [ ! -f "$f" ]; then
    curl -fsSL "$1" -o "$f.part"
    mv "$f.part" "$f"
  fi
  local got
  got=$(shasum -a 256 "$f" | cut -d' ' -f1)
  if [ "$got" != "$3" ]; then
    echo "bench-engines.sh: $2 has sha256 $got, expected $3" >&2
    rm -f "$f"
    exit 1
  fi
}

# The npm engines (quickjs-wasi, quickjs-emscripten, monty): package-lock.json pins them
# with their integrity hashes.
if ! cmp -s js/bench/engines/package-lock.json "$V/npm/package-lock.json" || [ ! -d "$V/npm/node_modules" ]; then
  mkdir -p "$V/npm"
  cp js/bench/engines/package.json js/bench/engines/package-lock.json "$V/npm/"
  (cd "$V/npm" && npm ci --no-audit --no-fund --silent)
fi

# wasi-sdk and the QuickJS releases' sources.
sdk="$V/dl/wasi-sdk-$WASI_SDK-$sdk_os"
if [ ! -x "$sdk/bin/clang" ]; then
  fetch "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_SDK%%.*}/wasi-sdk-$WASI_SDK-$sdk_os.tar.gz" "wasi-sdk-$WASI_SDK-$sdk_os.tar.gz" "$sdk_sha"
  tar -xzf "$V/dl/wasi-sdk-$WASI_SDK-$sdk_os.tar.gz" -C "$V/dl"
  rm "$V/dl/wasi-sdk-$WASI_SDK-$sdk_os.tar.gz"
fi
fetch "https://bellard.org/quickjs/quickjs-$QJS.tar.xz" "quickjs-$QJS.tar.xz" "$QJS_SHA"
fetch "https://github.com/quickjs-ng/quickjs/releases/download/v$NG/quickjs-amalgam.zip" "quickjs-ng-$NG-amalgam.zip" "$NG_SHA"

# Both releases build the same way: -O2, a WASI reactor, js/bench/engines/driver.c as the
# embedding. Bellard's sources need three changes for WASI (no threads for Atomics, no
# malloc_usable_size, an unused setjmp.h); QuickJS-NG builds as released.
CFLAGS=(--target=wasm32-wasip1 -O2 -mexec-model=reactor -Wl,--strip-all)
stamp="$WASI_SDK ${CFLAGS[*]} $(shasum -a 256 js/bench/engines/driver.c | cut -d' ' -f1)"
build() { # name version src... (in the current directory)
  local out="$V/build/$1.wasm" name=$1 version=$2
  shift 2
  if [ -f "$out" ] && [ "$(cat "$out.stamp" 2>/dev/null)" = "$stamp $version" ]; then return; fi
  "$sdk/bin/clang" "${CFLAGS[@]}" -I. "$@" "$root/js/bench/engines/driver.c" -o "$out"
  echo "$stamp $version" > "$out.stamp"
}
(
  cd "$V/build" && rm -rf "quickjs-$QJS" && tar -xJf "$V/dl/quickjs-$QJS.tar.xz" && cd "quickjs-$QJS"
  sed -i.orig -e 's/^#if !defined(__EMSCRIPTEN__)$/#if !defined(__EMSCRIPTEN__) \&\& !defined(__wasi__)/' \
    -e 's/^#elif defined(__EMSCRIPTEN__)$/#elif defined(__EMSCRIPTEN__) || defined(__wasi__)/' quickjs.c
  sed -i.orig -e 's/^#include <setjmp.h>$//' dtoa.c
  build quickjs "$QJS" -DCONFIG_VERSION="\"$QJS\"" quickjs.c libregexp.c libunicode.c cutils.c dtoa.c
  echo "QuickJS $QJS (bellard.org), wasi-sdk $WASI_SDK -O2" > "$V/build/quickjs.wasm.version"
)
(
  cd "$V/build" && rm -rf "quickjs-ng-$NG" && mkdir "quickjs-ng-$NG" && cd "quickjs-ng-$NG"
  unzip -q "$V/dl/quickjs-ng-$NG-amalgam.zip"
  build quickjs-ng "$NG" quickjs-amalgam.c
  echo "QuickJS-NG $NG (release amalgamation), wasi-sdk $WASI_SDK -O2" > "$V/build/quickjs-ng.wasm.version"
)

# djs: the shipped module and the dump module (dumps move the image there).
nice -n 10 scripts/build-wasm.sh >/dev/null
nice -n 10 scripts/build-wasm.sh --dump >/dev/null

[ "${1:-}" = "--setup" ] && exit 0
export DJS_ENGINES="$V"
exec node js/bench/engines/run.ts "$@"
