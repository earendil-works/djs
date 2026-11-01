#!/usr/bin/env bash
# The build environment of the shipped wasm module, as `NAME=value` lines (one per
# variable), for every build of it: scripts/build-wasm.sh, the gate's size step,
# djs-bench's size and wasm groups and scripts/size/variant.sh, so they all produce the
# same module. Apply it with `scripts/with-wasm-env.sh CMD...` (or parse the lines).
#
# - CARGO_ENCODED_RUSTFLAGS (0x1f-separated): `--remap-path-prefix` so that no absolute
#   path of the build machine (home directory, cargo registry, rustup toolchain,
#   checkout) ends up in the module (D2331); any RUSTFLAGS of the caller come after.
#   rustc applies the last matching remap, so the catch-all for $HOME comes first; the
#   rust-src path maps to /rustc/<commit>, as the precompiled std's own paths are.
# - `-Zlocation-detail=none` with RUSTC_BOOTSTRAP=1 (D2333): the one place where djs
#   uses an unstable rustc flag. It drops the file/line/column of panic locations from the
#   module. DJS_STABLE_ONLY=1 turns it off; the module then builds on plain stable.
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
if [ "${DJS_STABLE_ONLY:-}" != 1 ]; then
  flags+=("-Zlocation-detail=none")
  echo "RUSTC_BOOTSTRAP=1"
fi
if [ -n "${RUSTFLAGS:-}" ]; then
  read -r -a extra <<< "$RUSTFLAGS"
  flags+=("${extra[@]}")
fi
(IFS=$'\x1f'; printf 'CARGO_ENCODED_RUSTFLAGS=%s\n' "${flags[*]}")
