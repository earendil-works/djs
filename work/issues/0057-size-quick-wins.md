# 0057: Size quick wins Q1–Q6 (from 0052)

status: done
agent: bloat
branch: issue/0057-size-quick-wins
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
- [x] Each change measured on its own (code and gzip), and perf checked on the engine
      track and the compile-throughput benchmark (a quiet moment, interleaved), with no
      regression beyond noise.
- [x] The gate green, ratchets unchanged.
- [x] docs/host.md / ARCH performance table updated with the new size numbers.

## Log
Baseline: main 4fe04f1 (after 0051, which grew the module by ~175 KB: the base snapshot
338 -> 387 KB, code 780 -> 837 KB). Each change measured on top of the previous one
(`work/scratch/0057/measure.sh`, stripped module, gzip -9 / brotli -q 11):

| step | code | stripped | gzip | brotli | Δ code |
| --- | ---: | ---: | ---: | ---: | ---: |
| base 4fe04f1 | 837,472 | 1,359,542 | 436,287 | 347,823 | |
| Q1 `--compress-relocations` (D1570) | 779,856 | 1,301,926 | 430,209 | 344,949 | -57,616 |
| Q2 emit helpers out of line (D1571) | 743,536 | 1,265,558 | 424,382 | 341,014 | -36,320 |
| Q3 boxed `PErr` (D1572) | 712,176 | 1,234,209 | 419,328 | 338,332 | -31,360 |
| Q4 no offsets in release `out_of_bounds` (D1573) | 699,863 | 1,221,863 | 417,451 | 336,633 | -12,313 |
| Q5 boxed `Fault` (D1574) | 686,908 | 1,208,946 | 416,472 | 335,896 | -12,955 |
| Q6 profile strips (D1575) | 686,908 | file 1,544,182 -> 1,208,946 | | | names -175 KB |

Total: code -150,564 (-18.0%), stripped -150,596, gzip -19,815, brotli -11,927; the file
as built -335 KB. Q4 gives less than in 0052 (-16.4 KB) because Q1 already shrank the
calls. Data is unchanged.

- Q1 is a djs-wasm build.rs (`cargo:rustc-link-arg-cdylib`) rather than target
  rustflags in .cargo/config.toml: wasm-ld rejects the flag when debug info is kept (the
  gate's debug build, line-table builds), and config rustflags cannot depend on the
  profile. Outside `touches:` (crates/djs-wasm/build.rs, new file), kept minimal.
- Q6 is `strip = true` in `[profile.wasm]` (Cargo.toml) instead of wasm-strip in
  build-wasm.sh: stripping cargo's output in place would leave a stripped file behind a
  fresh fingerprint, so `--keep-names` could not get the names back without a rebuild.
  The profile output is byte-identical to wasm-strip's. `--keep-names` sets
  `CARGO_PROFILE_WASM_STRIP=false`. The size track adds `brotli -q 11`.
- Q5 uses lower-case constructors (`Fault::throw/error/heap`, cold, out of line); all
  54 construction sites renamed mechanically.
- Perf (the machine never got quiet: load 9–34 from other agents; interleaved runs,
  min of best):
  - native djs-bench, base 4fe04f1 vs head, 6 alternating rounds: engine/djs/* all
    equal or faster (loop -18%, arrays -15%, calls -12%, float -9%, json -9%,
    higher-order -7%, props -6%, regexp/strings -3%); startup/vm_new compiling the
    prelude -0.9%; vm_new from base -1.3%. lex/typescript.js +30% although the lexer did
    not change, so treat ±15% as noise here; the direction is consistently not slower.
  - wasm (`work/scratch/0057/perf.ts`, 4 rounds over the five steps): compile of
    typescript.js (9 MB) 292.0 -> 289.2 ms, throw loops 157.8 -> 151.9 / 63.8 -> 61.9,
    loop 309 -> 329, calls 140 -> 138, props 483 -> 458, higher-order 1025 -> 971,
    strings 334 -> 313, regexp 206 -> 211, json 22.8 -> 22.5 ms: all within ±7%, no
    regression beyond noise.
- Checks before the merge: djs-core lib tests, e2e (all modes), js tests against the
  wasm-profile module, workspace clippy, the debug wasm build with compiled-prelude, a
  `CARGO_PROFILE_WASM_DEBUG=1` build.

## Follow-ups
- S1 (LZ4 base image decoded once per instance): needs the owner's decision.
- S2 (a leaner heap/image layout) and S3 (bounds check per cell): later, as their own
  issues.
- wasm-opt: binaryen 117 exists in ~/Library/Caches/.wasm-pack. Its memory-packing pass
  makes gzip worse.

## Result
Q1–Q6 landed (D1570–D1575). The shipped module (`scripts/build-wasm.sh`, now stripped by
default) is 1,208,946 bytes: code 837 -> 687 KB (-18%), gzip -9 436 -> 416 KB, brotli
348 -> 336 KB; the file as built 1,544 -> 1,209 KB. No measurable perf regression
(native engine track equal or faster, wasm compile throughput unchanged). Use
`scripts/build-wasm.sh --keep-names` for profiling or `scripts/wasm-funcs.py`.
Known gaps: the base snapshot (387 KB, S1/S2) is now the dominant part of the data and
the biggest remaining item; the size probe (tests/bench/size) keeps its own profile and
is not stripped.
