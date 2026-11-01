# 0126: Size: run-length Unicode tables, panics without formatting

status: done
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
- [x] Both cuts landed with sizes measured via scripts/size; no workload slower than +1%.
- [x] Panics remain diagnosable in native/dev builds (messages kept there; only the shipped
      wasm drops them), and a wasm trap still maps to a clear host error.
- [x] Decisions recorded.

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
- For heap's 0128 (page allocator): the allocator setup moved to its own module,
  crates/djs-wasm/src/allocator.rs (`Base`: dlmalloc on wasm32, std's System natively for
  alloc-stats; the global allocator unless alloc-stats wraps it). Named `allocator`, not
  `alloc`, which is the alloc crate. Module unchanged (1,159,910 raw).
- Unicode (D2334, D2335): props.rs coded by structure (General_Category as a partition,
  binary properties as a base of GC leaves / an earlier property XOR a difference, emoji
  sequences by their structure as libunicode does): 57,820 -> 20,477 bytes; a scratch
  comparison of all 976 property lookups against the old tables is identical, test262
  property-escapes 613/613. Then the NFD/NFKD/NFKC quick-check tables replaced by exact
  derivations (asserted in the generator; a unit test over every code point). Module
  471,983 -> 465,290 gzip (−6.7 KB), raw 1,121,445 -> 1,082,157. Not done: the
  decomposition stream (11.6 KB gzip; a libunicode-style recoding might save 2–3 KB for
  real complexity) and the composition tables (deriving them needs a runtime cache).
- Debug details (D2336): internal errors and invariant panics show `Debug` details only
  natively and in debug builds (`crate::detail::detail`); wire/djs-wasm heap errors use
  Display. 465,294 -> 464,043 gzip (−1.25 KB). Full core::fmt removal is not possible on
  stable (core's panics build fmt::Arguments at call sites); replacing the ~100 `format!`
  sites of JS-visible messages would give ~2–3 KB more (follow-up).
- Perf (wmeasure, wasm instructions, base before the Unicode work vs after Unicode+Debug,
  n=3/5): every workload within ±0.6% (json re-measured at n=5: −0.4%). The lexer's ID
  tables, case tables and NFC tables are untouched.
- js/test sandbox trap test accepts the release module's fixed message too. Guide:
  docs/guide/host.md (trap paragraph) updated; docs/host.md too.

## Follow-ups
- core::fmt from JS-visible messages: a small non-formatting string builder for the ~100
  `format!` sites (interp/run.rs, date.rs, compiler, intrinsics): ~2–3 KB gzip, touches
  files several agents work in.
- The normalization decomposition stream (11.6 KB gzip): a libunicode-style recoding
  (per-type codings, CJK compatibility ideographs as a table) might save 2–3 KB gzip.
- The composition tables (3 KB gzip) could be derived from the decompositions, but need
  a runtime cache (no global state in djs-core) to stay fast for localeCompare.

## Result
Landed in five merges (D2331–D2336). The shipped module went 509,489 -> 464,043 bytes
gzip (−45.4 KB, of which perf's 0127 merged in between is part; this issue's own cuts,
each measured against its baseline: path remap −0.1, no_std panic handler −3.4, no panic
locations −8.5, Unicode tables −6.7, Debug details −1.25: −20 KB gzip), raw 1,186,680 ->
1,078,802. Build-machine paths are gone from the module and the gate checks for them.
Panic messages remain natively and in debug wasm builds; the shipped module reports a
fixed trap message. DJS_STABLE_ONLY=1 builds the module without the one unstable flag.
Known gaps: core::fmt stays reachable on stable (see D2336 and the follow-ups).
