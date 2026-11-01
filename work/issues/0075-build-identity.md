# 0075: Build identity and Float32 NaN hardening

status: done
agent: num
branch: issue/0075-build-identity
depends-on: -
touches: crates/djs-core/build.rs, crates/djs-core/src/builtins/ (typed arrays, Math.fround)

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [x] Cargo features and the libm version in BUILD_HASH. Float32 encoding and `Math.fround` write the canonical f32 NaN explicitly. Small.
- [x] The criteria in 0055's report for this item.
- [x] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log
- Reassigned from determinism to num (coordinator).
- build.rs: hashes the sorted CARGO_FEATURE_* names (minus `default`) and the libm version,
  read from an `=` pin in Cargo.toml (pinned `libm = "=0.2.16"`; lock unchanged). The
  inputs are emitted as `vm::BUILD_INPUTS`; `vm::tests::build_inputs_list_features_and_libm`
  checks them against cfg!(feature) and Cargo.lock (passes with and without default
  features).
- heap/typed.rs: `f64_to_f32_bits` / `f32_bits_to_f64` with explicit NaN; used by Float32
  encode/decode (typed arrays, DataView) and Math.fround. Float16 NaN is 0x7e00 without the
  input's sign. Unit test `heap::tests::float_nan_encoding_is_canonical` with bit patterns.
- test262 Math/fround, DataView, Float32Array, Float16Array, TypedArrayConstructors: all
  pass; difftest ratchet OK and stress modes identical.

## Follow-ups

## Result
`BUILD_HASH` now covers djs-core's enabled cargo features and the libm version (pinned
with `=`, read by build.rs; listed in `vm::BUILD_INPUTS`). Float32 writes (typed arrays,
DataView) and Math.fround produce the canonical f32 NaN `0x7fc00000` explicitly, Float32
reads give the canonical f64 NaN, and Float16 NaN is always `0x7e00` (D1750). No JS-visible
change on the measured platforms; ratchets unchanged. A libm bump now means editing the
pin in crates/djs-core/Cargo.toml.
