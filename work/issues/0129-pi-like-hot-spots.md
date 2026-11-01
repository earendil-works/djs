# 0129: pi-like hot spots after GC work (interpreter, native calls, JSON)

status: done
agent: perf
branch: issue/0129-pi-like-hot-spots
depends-on: 0116, 0120, 0127
guide: docs/guide/interpreter.md / builtins.md if mechanisms change
touches: crates/djs-core/src/vm/ (run_hot, call paths), crates/djs-core/src/builtins/ (JSON, strings), crates/djs-bgen/ (native call glue); heap owns heap/ and djs-wasm/src/allocator.rs (0128), bloat owns Unicode/fmt (0126)

## Goal
Back to speed, with the size ratchet in force (no growth without a recorded trade).
Re-profile pi-like in wasm on current main (GC is now old-generation, so the
picture changed) and attack the top non-GC costs: the hot interpreter loop on user code
(~6.5%), the native call path (~3.5%), JSON parse tokenizer/quoting/escapes (~8%),
`alloc_latin1_pieces` in slice().join() (2.3%). Also try the one-pass JSON string
scan+copy idea from 0116.

## Acceptance criteria
- [x] Fresh wasm profile of pi-like recorded.
- [x] Wins landed with wasm instruction counts on all workloads (no workload slower than
      +1%), size ratchet respected (or trades recorded).
- [x] Ratio to quickjs-wasi on pi-like re-measured (it was ~1.4–1.5x instructions).

## Log
### Fresh wasm profile (main 74353e58, node --cpu-prof self time, pi-like)
wmeasure n=3: pi-like 8071.7 Minstr djs vs 7234.5 qjs-wasi = **1.116x** (was ~1.4–1.5x);
json 0.58x, json-text 1.16x, strings 0.41x.

Top djs functions (self): run_hot 6.6%, json `Out::quote` 4.2%, json `Parser::next` 2.8%,
`alloc_latin1_pieces` 2.3%, `write_escaped_latin1` 2.1%, `n_slot` 2.0%, `find_latin1` 1.7%,
`n_call` 1.7%, `step_once` 1.5%, `str_view` 1.3%, `try_fast_call` 1.1%,
`set_element_fast` 1.0%, `native_ArrayPrototypeFilter` 1.0%. GC (mark_olds/visit/
for_each_slot/mark_compact/mark) about 9% together.

### JSON quoting and string scan (D2300)
- `quote`: on a word with a byte to escape, jump to its first such byte (lowest SWAR flag)
  instead of byte-by-byte; the SWAR mask moved to json/mod.rs (`escape_mask8`), shared.
- parse `scan_string`: after an escape, an out-of-line `plain_run` skips plain words
  (keys and short strings keep the 0116 byte loop; an inline word test in the loop cost
  parse-short +2%).
- wasm n=3: pi-like 8083.5 → 7755.1 (−4.1%), json-text −2.3%, others within ±0.4%
  (json +0.3%, json-indent +0.5%). Size 460.9 KB (−1.2 KB vs main).

### One-pass JSON string scan+copy (tried, dropped)
Decoding one-byte strings with escapes into a scratch buffer during the scan, then one
`alloc_latin1` (onepass.diff in scratch): native json-text +2.6%, pi-like +0.6% against
the D2300 scan alone. The second pass (`write_escaped_latin1`) already copies runs in
bulk; the scratch copy plus the cell copy is the same traffic plus a Vec.

### slice/join/split and CreateDataPropertyOrThrow (D2301)
- In wasm, pi-like's pieces against quickjs-wasi (minus a base script):
  - join 1.38x, split+filter 1.11x, parse 1.03x, quote 0.51x;
  - map/filter/sort/format 1.37x.
- slice: `plain_species` plus `Heap::new_array_from_dense` (one copy). Anything else
  declines to the builtin.
- join: `Heap::join_flat_latin1`. Natively the slice+join microbench went 453 → 189
  Minstr, and join is now 0.64x qjs-wasi in wasm.
- CDP into Arrays: `Heap::create_element_fast`, no prototype walk. split: direct piece
  copies.
- New corpus cases:
  - builtins/slice-join-bulk.js (slices, holes, species, ropes, two-byte strings, split
    edge cases);
  - builtins/array-create-data-property.js (proto setters not called; non-extensible,
    fixed-length and frozen results throw).

### Native call path (measured, not landed)
In wasm, per element against quickjs-wasi:
- `map` callback: 3087 vs 1052 instructions;
- `forEach`: 1933 vs 737;
- a bytecode loop calling `f`: 1731 vs 2394 (djs is faster there).

About a third of the native cost is the out-of-line native API (D1634: n_slot, n_arg,
n_set_slot, n_result, f64_of, num). Inlining it is a size trade:
- n_slot+n_arg: +4.6 KB gzip, for higher-order −10.5%, foreach −5.7%, pi-like −2.1%;
- num, f64_of and n_result: +2.6 KB for −3%;
- the whole set: +10.7 KB, for −17%, −9%, −2.6%.

The coordinator put this on the owner's list. 0107's `n_srcs` batching (one call for all arguments) had already
been tried (slower). Native djs is ~2x slower than native qjs on plain loops, but in
wasm it is 0.4–0.5x quickjs-wasi: wasm is the target, so native ratios to qjs say
little.

### Size
- Main 455,584 → 456,943 gzip (+1,359). Recorded as the trade D2302, with the ceiling
  raised to 457,967.
- Shaved before that: shared scan instances, slice declining to the builtin.
- A leaner join variant saved 232 B but cost pi-like 2.3%. Sharing `set_element_fast`'s
  append tail saved 140 B but cost pushpop/push-pop 0.4%; both dropped.

### Measurement note
wmeasure counts include V8's TurboFan tier-up of the module: ~525 M for djs and ~322 M
for quickjs-wasi on a near-empty script. Written up in TESTING.md and
scripts/perf/README.md.

Guide: no page describes these mechanisms; none changed. Also fixed: scripts/perf/
instr.py for the generational collector.

## Follow-ups
- Owner trade: inline the native API in wasm (numbers above). It is the remaining lever
  on callback-heavy code (higher-order 2.6x, foreach 1.36x quickjs-wasi).
- join of two-byte strings and ropes still goes through the pieces list; it could share
  `join_flat_latin1`'s single pass (widening copy) if a workload needs it.

## Result
- **pi-like in wasm: 7815 → 6825 Minstr (−12.7%), 0.94x quickjs-wasi** (it was ~1.1x
  at the start of 0129). Measured on main 1954f0dc, n=3.
- Other changes:
  - json-text −11.7%; higher-order −2.9%; strings and strbuild −2%;
  - every other workload within ±1%.
- The ratio counts V8's TurboFan compile of each module (~0.5 G for djs, ~0.3 G for
  quickjs-wasi). The engine-only pi-like ratio is ≈0.91x.

What landed:
- D2300: JSON quoting and scanning word-wise through escaped text.
- D2301: slice, join, split and CreateDataPropertyOrThrow on plain arrays in one step.
- D2302: the size trade, +1,359 B gzip, ceiling 457,967.

Tried and dropped:
- the one-pass JSON scan+copy;
- inlining the native API (+4.6 to +10.7 KB; it is on the owner's list).

The fresh profile is in the Log. GC is ~10% of pi-like; the top engine costs now are
run_hot (6.7%), JSON (~9%) and the native accessors.
