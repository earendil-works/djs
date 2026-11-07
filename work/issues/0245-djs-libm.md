# 0245: djs-libm: libm's functions as a djs crate

status: done
agent: libm
branch: issue/0245-djs-libm
depends-on: -
touches: crates/djs-libm (new), crates/djs-core/Cargo.toml, crates/djs-core/build.rs (BUILD_INPUTS: the libm version), every `libm::` call site in djs-core, Cargo.lock, docs/decisions (D1150/D1750 superseded by D3174), ARCH.md (dependencies paragraph)
guide: any guide page that names libm (grep)

## Goal

D3174. Copy the libm 0.2.16 functions djs-core uses (`grep -rhoE "libm::[a-z0-9_]+"
crates/djs-core/src`: trunc, floor, ceil, fmod, ldexp, sqrt, fabs, sin, cos, tan, asin, acos,
atan, atan2, sinh, cosh, tanh, asinh, acosh, atanh, exp, expm1, log, log2, log10, log1p, pow,
hypot, cbrt) with their helpers into `crates/djs-libm` (`#![no_std]`, no dependencies, its
LICENSE files and a header naming the source and version), keeping the `arch`
implementations libm selects by default (wasm32 `f64.floor`/`sqrt`/..., x86 SSE2 sqrt) so the
generated code and every result stay identical. djs-core depends on it instead of libm; the
build hash input of D1750 hashes djs-libm's sources (or its version) instead of libm's version.

## Acceptance criteria
- [x] crates/djs-libm with only what is needed; clippy/fmt clean under the workspace's rules
      (allow-lists for the copied code where its style trips clippy, with a comment)
- [x] djs-core no longer depends on libm; `cargo tree -p djs-core -e normal` shows no
      third-party crate
- [x] identical results: a test comparing djs-libm to libm 0.2.16 (dev-dependency, fine) on many
      inputs per function, including special values; test262 Math group and the corpus unchanged
- [x] wasm size unchanged (or measured), native-vs-wasm determinism leg green
- [x] D1150/D1750 references in ARCH.md updated

## Log

2026-11-07 (libm)
- crates/djs-libm: libm 0.2.16's files for the 29 functions and what they reach (fma via
  cbrt, scalbn, rem_pio2/_large, k_*, expo2/k_expo2, generic/*, support/*), cut
  mechanically (scratch scripts copy.py/prune.py in work/scratch/0245/): f32/f16/f128
  variants, libm's tests, off-by-default cfg items, unreachable trait items. A diff against
  libm shows only removed lines plus re-export lists and three djs-libm comments.
- Found: in libm's *default* build wasm32 does not use the `f64.floor`/`f64.sqrt` arch
  routines (they need `unstable-intrinsics`); it runs the generic code. So nothing wasm
  specific was copied. Kept: x86 SSE2 sqrt + runtime-detected fma, aarch64 fsqrt/fmadd.
- tests/same_as_libm.rs: bit-for-bit against libm (dev-dep) for every export, specials,
  thresholds and 200k random inputs each; checked that a one-ulp change of a k_sin
  constant fails it; passes natively and under x86_64 (Rosetta).
- djs-core: `libm::` -> `djs_libm::` (51 sites); build.rs hashes djs-libm/src with the
  sources; BUILD_INPUTS is features only (test renamed build_inputs_list_features).
  `cargo tree -p djs-core -e normal`: only djs-libm.
- Root Cargo.toml (outside touches:): `[profile.dev.package.djs-libm] opt-level = 1`, as
  libm had through `"*"` (D1785). scripts/size/areas.py, scripts/wasm-funcs.py: the libm
  area regex also matches `djs_libm::`.
- Measured: shipped module 979,984 bytes vs 980,158 on main (-174): rem_pio2_large is now
  inlined into rem_pio2 (its only caller, rem_pio2f is gone); every other function the same
  size. native-vs-wasm images OK; test262 built-ins/Math 327/327; difftest math/number/
  dquickts ratchet OK; djs-core --lib, djs-prelude green; workspace + wasm32 clippy clean.
- Guide: snapshots.md and determinism.md (build hash wording). ARCH.md dependencies
  paragraph. Decision D3215 (index line; D1150/D1750 index lines marked superseded).

## Follow-ups

## Result
djs-core has no third-party runtime crate: its math is `crates/djs-libm`, libm 0.2.16's code
(MIT, LICENSE.txt), pruned to what djs-core calls, with libm's default arch routines, and
proven identical by `cargo test -p djs-libm` (bit-for-bit comparison against libm 0.2.16 as a
dev-dependency). The build hash covers djs-libm's sources. Updating: copy a newer libm's
files the same way (D3215 lists the cuts), rerun the comparison test with the new version
as the dev-dependency, record a decision. Shipped module -174 bytes.
