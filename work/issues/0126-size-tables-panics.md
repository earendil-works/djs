# 0126: Size: run-length Unicode tables, panics without formatting

status: in-progress
agent: bloat
branch: issue/0126-size-tables-panics
depends-on: 0123
guide: docs/guide/regexp.md / strings pages if they describe the tables (check)
touches: crates/djs-core/src/unicode/ (or wherever the tables live) and their generators, crates/djs-wasm/ (panic handler), panic/format call sites in djs-core (error paths), Cargo profile; coordinate with perf (0122) which works on the base image, natives and containers

## Goal
From bloat's report (docs/size-vs-quickjs.md): two speed-free cuts toward the D020 budget,
disjoint from perf's 0122 work:
1. Unicode tables are 107 KB raw vs QuickJS's 43 KB (run-length encoded): re-encode
   (\p property sets, normalization, case/ID) compactly; ~−13 KB gzip. Lookups must stay
   fast where hot (ID_Start/ID_Continue in the lexer, case mapping), measure.
2. Panic and formatting paths: 17.5 KB gzip measured with a nightly abort build. On stable:
   a panic handler that never formats, removing core::fmt from reachable code (panic
   messages, `format!` in error paths, Debug impls reached from wasm), panic location
   strings; measure what's reachable with twiggy.

## Acceptance criteria
- [ ] Both cuts landed with sizes measured via scripts/size; no workload slower than +1%.
- [ ] Panics remain diagnosable in native/dev builds (messages kept there; only the shipped
      wasm drops them), and a wasm trap still maps to a clear host error.
- [ ] Decisions recorded.

## Log
- Priority item (coordinator, from perf): the shipped module carried absolute paths of
  the build machine in panic locations (`/Users/<name>/.rustup/...` for inlined std code,
  `.../.cargo/registry/...` for libm). `scripts/wasm-rustflags.sh` remaps them
  (CARGO_ENCODED_RUSTFLAGS with `--remap-path-prefix`; cargo's trim-paths is unstable in
  1.99) for every shipped-module build: build-wasm.sh, the gate's size step, djs-bench's
  size/wasm groups (shared with perf: two `.env(...)` lines), variant.sh. budget.py now
  fails on `/Users/`, `/home/`, `.cargo/registry`, `.rustup/` in the module (D2331).
  Module 1,186,680 -> 1,186,312 raw (the paths get shorter; the panic work removes the
  rest). Merged on its own first.
- Panics (D2332, D2333): djs-wasm is no_std on wasm32 with its own panic handler (fixed
  message in release modules, the formatted one in debug modules; natively the std hook as
  before) and dlmalloc as the global allocator (std's own on that target). Alone: −3.4 KB
  gzip (std's panic machinery). Core panics still build `fmt::Arguments` and a `Location`
  at each call site, which LLVM can't drop on stable; `-Zlocation-detail=none` (with
  RUSTC_BOOTSTRAP=1, only in the shipped build, coordinator's call) removes the locations:
  506,114 -> 497,603 (−8.5 KB). Total 509,489 -> 497,603 gzip (−11.9 KB), raw 1,186,680 ->
  1,159,910. With DJS_STABLE_ONLY=1 the module builds on plain stable (506,114, tested).
  scripts/wasm-rustflags.sh became scripts/wasm-build-env.sh + scripts/with-wasm-env.sh.
  For comparison, nightly immediate-abort with build-std reaches 487,772: ~10 KB more
  that stable can't get (call-site `fmt::Arguments` of core panics).
- Perf (wmeasure, wasm instructions, n=3, base vs new): loop/calls/props/strings/json/
  higher-order/regexp equal or lower, arrays +0.3%, strbuild +0.2%, pi-like +0.6%
  (spread 0.2–0.6%): no workload beyond +1%.
- js/test sandbox trap test accepts the release module's fixed message too. Guide:
  docs/guide/host.md (trap paragraph) updated; docs/host.md too.

## Follow-ups

## Result
