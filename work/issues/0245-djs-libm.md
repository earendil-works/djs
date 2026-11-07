# 0245: djs-libm: libm's functions as a djs crate

status: in-progress
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
- [ ] crates/djs-libm with only what is needed; clippy/fmt clean under the workspace's rules
      (allow-lists for the copied code where its style trips clippy, with a comment)
- [ ] djs-core no longer depends on libm; `cargo tree -p djs-core -e normal` shows no
      third-party crate
- [ ] identical results: a test comparing djs-libm to libm 0.2.16 (dev-dependency, fine) on many
      inputs per function, including special values; test262 Math group and the corpus unchanged
- [ ] wasm size unchanged (or measured), native-vs-wasm determinism leg green
- [ ] D1150/D1750 references in ARCH.md updated

## Log

## Follow-ups

## Result
