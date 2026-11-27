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
# - `-Cllvm-args=-inline-threshold=40` (D2255): less inlining, for size.
# - With RUSTC_BOOTSTRAP=1, the one place where djs uses unstable rustc and cargo flags
#   (DJS_STABLE_ONLY=1 turns them all off; the module then builds on plain stable):
#   - `-Zlocation-detail=none` (D2333): no file/line/column of panic locations.
#   - core and alloc built from source (`CARGO_UNSTABLE_BUILD_STD`, cargo's
#     `-Zbuild-std`) with `-Cpanic=immediate-abort` (D3090): a panic is a bare
#     `unreachable` trap, without the panic call sites' arguments, messages and core's
#     panic machinery (−10.8 KB gzip). Release modules recorded no panic message anyway
#     (D2332); js/src/trap.ts names such a trap an engine panic. Needs the toolchain's
#     rust-src component (checked below).
#   - `-Zshare-generics=no` (D3090): at opt-level s rustc shares generic instances with
#     upstream crates, and with core built at s that made djs-core call core's
#     out-of-line `copy_from_slice_impl` from the arena accessors (+40% instructions in
#     loops); without sharing, every crate instantiates its own and inlines it.
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
  # A lower inlining threshold than opt-level s's: -3.2 KB gzip for +0.4% instructions
  # on pi-like, json and calls (D2255).
  "-Cllvm-args=-inline-threshold=40"
)
if [ "${DJS_STABLE_ONLY:-}" != 1 ]; then
  if [ ! -f "$sysroot/lib/rustlib/src/rust/library/core/Cargo.toml" ]; then
    echo "wasm-build-env.sh: the shipped module builds core and alloc from source and needs" \
      "the rust-src component: rustup component add rust-src (or DJS_STABLE_ONLY=1 for a" \
      "plain stable build)" >&2
    exit 1
  fi
  flags+=("-Zlocation-detail=none" "-Zunstable-options" "-Cpanic=immediate-abort" "-Zshare-generics=no")
  echo "RUSTC_BOOTSTRAP=1"
  echo "CARGO_UNSTABLE_BUILD_STD=core,alloc"
fi
if [ -n "${RUSTFLAGS:-}" ]; then
  read -r -a extra <<< "$RUSTFLAGS"
  flags+=("${extra[@]}")
fi
(IFS=$'\x1f'; printf 'CARGO_ENCODED_RUSTFLAGS=%s\n' "${flags[*]}")
