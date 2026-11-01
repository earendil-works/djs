# 0026: Leaf performance sweep (arena inlining, opt-level, allocation, stringify)

status: done
agent: num
branch: issue/0026-leaf-perf-sweep
depends-on: 0023
touches: crates/djs-core/src/heap/ (arena accessors, allocation), crates/djs-core/src/json/, crates/djs-core/src/wire/, Cargo.toml ([profile.wasm] only), crates/djs-bench/ (new benchmarks)

## Goal
Collected performance follow-ups from 0012, 0018, 0021 and 0023 that are all about the
leaf layer (the arena and the data paths), measured both natively and in wasm.

## Acceptance criteria
- [x] `#[inline(always)]` (or restructuring) on the hot arena accessors
      (`Arena::read/write::<T>`, `layout::read_array`, …). Re-measure at opt-level z and
      s in wasm. If z gets within ~10% of s, propose switching back (with numbers;
      coordinator decision).
- [x] Allocation without zero-fill where the caller initializes every byte, and arena
      reserve costs (0018 follow-up: they dominate object decoding and JSON parse).
- [x] `json::stringify_fast` on number arrays (0021: 11 ms for 500k) and on object-heavy
      data (0012: ~50% of node). Profile and fix what is cheap.
- [x] Benchmarks: add the wire decode and the reader-side benchmarks to djs-bench (0019
      follow-up), and wasm variants via the size/engine tracks or a new `wasm` group
      that runs in node, if practical.
- [x] Record before/after in tests/scores/bench.tsv (`scripts/bench.sh --record` on main
      before and after).
- [x] No behavior changes: all corpora (num, json, regexp, wire, reader) still pass.

## Notes
- 0022 (heap agent) is working in interp/ and vm/ at the same time. Stay out of those.
- The machine is shared with other agents compiling, so measure carefully (D1190 notes
  on noise) and repeat runs.

## Log
- **Benchmarks first.**
  - djs-bench `wire` group: decode, parse_utf8 and stringify on the 0018/0021 boundary
    datasets, generated natively by `data.rs` (one value tree written as JSON and wire;
    a test checks that both agree).
  - djs-bench `wasm` group: builds djs-wasm, writes the datasets, runs
    `js/bench/wasm.bench.ts --tsv` in node (decode, parse_json, readValue,
    json_stringify, instantiate, vm_new, an interpreter loop). Skipped without node or in
    quick mode.
  - Runner additions: `time_with_setup` and `wants_group` (filters naming results inside
    a group).
  - Baseline recorded before any change.
- **Arena accessors** (D1260): `inline(always)` plus `first_chunk` loads and stores. At z,
  the interpreter loop went 132 → 83 ms and decode 15.8 → 7.6 ms; s barely moved (it
  already inlined them).
- **Allocation** (D1261): `alloc_init` (strings, GC copies) writes data once; a zeroed
  frontier makes small allocations a bump; `alloc_from_iter` converts UTF-16 straight into
  the cell. Tried and dropped: zeroing by static-block or doubling copies (fixes z's
  byte-loop `resize` but costs 15% natively); a buffered `[u8; N]` iterator path (no gain:
  the profile samples were first-touch page faults, not the iterator).
- **stringify_fast**:
  - two-byte strings are quoted in runs (1 MiB text: −54%, tool results: −33%);
  - short decimals skip Ryu (D1262; 500k numbers: −32% natively, −47% in wasm);
  - an inline copy for short pieces did nothing (LLVM makes it a memcpy again), so it
    stays a plain `extend`.
  - The rest of object stringify is spread over quote/emit/member, with no cheap target
    left.
- **z vs s** (D1260): z is still 1.4x (interpreter) to 2.8x (JSON on text) slower, far from
  10%. s stays.
- **Before/after**: recorded in tests/scores/bench.tsv. "Before" is main at 0394842
  (existing groups); for `wire/` and `wasm/` it is the first baseline. The sequential
  "after" run was noisy (unchanged host-only `readValue` moved ±30%), so the reliable
  deltas are interleaved A/B runs (`work/scratch/0026/ab.sh`; main's binary, and this
  branch's djs-bench built against main's djs-core).

  Native:
  - GC: −19% / −11%;
  - JSON parse: +7% MB/s;
  - wire decode: objects −7%, string −19%, tool results −8%;
  - stringify: objects −8%, text −54%, tool results −33%, numbers −32%;
  - `num/to_string(short decimal)`: −68%; random doubles +1%.

  Wasm (s):
  - decode: objects −15%, tool results −8%;
  - parse_json: objects −10%, tool results −5%;
  - json_stringify: text −52%, tool results −24%, numbers −47%;
  - no regressions beyond noise.
- **Corpora**: unit tests and in-tree corpora pass; the external num corpus (525k qjs
  cases) matches; wire and reader cross tests run in the gate. New tests: an arena frontier
  model, alloc_init / alloc_from_iter paths, the debug-only past-the-end assert,
  short_decimal against Ryu (2M values), and two-byte quoting against the independent
  quoter (checked by a mutation).
- **Coordinator extra**: after a failed run (`maxFuel`, or a host-side error escaping
  the loop), the Sandbox calls `vm_abort` and stays usable; two node tests; docs/host.md
  updated.
- **Outside touches** (minimal):
  - `crates/djs-core/src/num/format.rs` (the short-decimal fast path; my module from
    0004);
  - `js/src/sandbox.ts`, `js/test/sandbox.test.ts`, `js/bench/wasm.bench.ts` (`--tsv`, the
    interp row) and docs/host.md, all mine from 0023.
- **Found**: on main, djs-wasm `vm_new` is 143 µs (11 µs at 0023), because it compiles the
  JS prelude each time. Reported; 0032 (base snapshot) takes it. LineTable did not show
  up in any of the leaf profiles here.

## Follow-ups
- `vm_new` 143 µs (the prelude is compiled per VM): 0032, base snapshot.
- JSON parse: the UTF-8 lexer (`lex::<2>`) dominates `parse_utf8` and is 2.8x slower at
  z. A parse-side sweep (run-based string scanning like the stringify change) could pay
  off.
- Native decode/parse benchmarks include first-touch page faults of the fresh to-space
  after every GC (~20% of object decode). Keeping a spare semispace would remove them
  natively; wasm does not have this cost (dlmalloc reuses the memory).
- docs/host.md's boundary tables predate the stringify and decode gains; refresh them
  (the `wasm/` group now tracks the same numbers).

## Result
Leaf-layer speedups, measured natively and in wasm (D1260-D1262):
- inlined fixed-size arena accessors;
- a zeroed arena frontier, with data written once for strings, GC copies and UTF-16;
- run-based quoting of two-byte strings in `stringify_fast`;
- an exact short-decimal fast path in number formatting.

Largest gains: stringify of text, tool results and numbers −24% to −54% (native and
wasm), wasm decode −15%, GC −11% to −19%; no regressions.

opt-level z was re-measured and stays rejected: still 1.4x to 2.8x slower.

New benchmarks: `scripts/bench.sh wire/` (native boundary) and `scripts/bench.sh wasm/`
(djs-wasm in node: boundary, startup, interpreter loop), with before/after rows in
tests/scores/bench.tsv. The Sandbox now survives `maxFuel` (vm_abort).

Known gaps: the JSON lexer is the next parse cost; on main `vm_new` is 143 µs until 0032.
