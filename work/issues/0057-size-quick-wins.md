# 0057: Size quick wins Q1–Q6 (from 0052)

status: open
agent: -
branch: issue/0052-bloat-investigation (continue on it: merge the 0052 report and scripts first, then these)
depends-on: 0052
touches: Cargo.toml / .cargo/config.toml (the wasm link flag only), crates/djs-core/src/compiler/ (emitter inlining, a boxed error), crates/djs-core/src/heap/ (out_of_bounds), crates/djs-core/src/interp/ (a boxed Fault), scripts/build-wasm.sh (strip by default), crates/djs-bench + tests/bench/size (track the new numbers)

## Goal
Land 0052's measured quick wins (work/issues/0052-bloat-investigation.md, "Quick wins";
patches in work/scratch/0052/patches/):
- Q1 `--compress-relocations` for the wasm link (−53.7 KB code, same code otherwise).
- Q2 `#[inline(never)]` on the bytecode emitter helpers (−34.6 KB).
- Q3 box the compiler error (−27.5 KB).
- Q4 `out_of_bounds()` without arguments in release (keep the offsets under
  `debug_assertions`) (−16.4 KB).
- Q5 box `Fault` (−9.8 KB).
- Q6 `scripts/build-wasm.sh` strips by default (`--keep-names` for profiling), and the
  size track records stripped and gzip/brotli sizes.

## Acceptance criteria
- [ ] Each change measured on its own (code and gzip), and perf checked on the engine
      track and the compile-throughput benchmark (a quiet moment, interleaved), with no
      regression beyond noise.
- [ ] The gate green, ratchets unchanged.
- [ ] docs/host.md / ARCH performance table updated with the new size numbers.

## Log

## Follow-ups
- S1 (LZ4 base image decoded once per instance): needs the owner's decision.
- S2 (a leaner heap/image layout) and S3 (bounds check per cell): later, as their own
  issues.
- wasm-opt: binaryen 117 exists in ~/Library/Caches/.wasm-pack. Its memory-packing pass
  makes gzip worse.

## Result
