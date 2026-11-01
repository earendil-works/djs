#!/usr/bin/env bash
# Prints CARGO_ENCODED_RUSTFLAGS (0x1f-separated) for every build of the shipped wasm
# module (D2331): `--remap-path-prefix` so that no absolute path of the build machine
# (home directory, cargo registry, rustup toolchain, checkout) ends up in the module's
# panic locations. Used by scripts/build-wasm.sh, the gate's size step and djs-bench's
# wasm builds, so they all produce the same module. Any RUSTFLAGS of the caller come
# after the remaps.
#
# rustc applies the last matching remap, so the catch-all for $HOME comes first. The
# rust-src path maps to /rustc/<commit>, as the precompiled std's own paths are.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
sysroot=$(rustc --print sysroot)
commit=$(rustc -vV | sed -n 's/^commit-hash: //p')
cargo_home=${CARGO_HOME:-$HOME/.cargo}
flags=(
  "--remap-path-prefix=$HOME=~"
  "--remap-path-prefix=$cargo_home=/cargo"
  "--remap-path-prefix=$sysroot/lib/rustlib/src/rust=/rustc/$commit"
  "--remap-path-prefix=$root=/djs"
)
if [ -n "${RUSTFLAGS:-}" ]; then
  read -r -a extra <<< "$RUSTFLAGS"
  flags+=("${extra[@]}")
fi
(IFS=$'\x1f'; printf '%s' "${flags[*]}")
