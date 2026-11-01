# 0021: Outbound direct heap reader (VM → host)

status: done
agent: num
branch: issue/0021-outbound-reader
depends-on: 0018
touches: js/ (new reader module), crates/djs-core/src/heap/layout.rs (additive: layout export), docs/wire.md (outbound section), tests for both

## Goal
ARCH.md "Data passing (G1)" → "Outbound, direct read": the host reads sandbox values
straight out of linear memory with a JS decoder that understands the heap layout, so the
VM does no serialization at all. Strings are decoded in place (`TextDecoder('utf-16le')`
for two-byte strings, chunked `String.fromCharCode` for Latin-1, never
`TextDecoder('latin1')`, which is windows-1252). Objects and arrays are walked directly.
The reader is generated from (or driven by) the same layout description as the Rust side
(D1052), so the two never drift apart.

## Acceptance criteria
- [x] Layout export: a function on the Rust side that serializes `LAYOUTS`, the value tag
      encoding, `LAYOUT_HASH` and the relevant constants (well-known atom ids are not
      needed; the atom table cell layout is) into a compact JSON description. A checked-in
      generated file `js/src/layout.generated.ts` (or `.json`) with a freshness test in
      cargo (like bgen's `--check`).
- [x] `js/src/reader.ts`: `readValue(memory: Uint8Array, valueBits: bigint | [lo, hi],
      opts) -> unknown` that decodes primitives, strings (flat one-byte and two-byte,
      ropes by walking them; ARCH says ropes are flattened first, so decide whether the
      reader walks ropes or requires flattened input, and document it), arrays (dense,
      holes as `undefined` or skipped: decide), plain objects (own enumerable string-keyed
      data properties in spec order, via the atom table to get key strings, with an atom
      key-string cache per read), and symbols/functions/others as a typed placeholder
      or error (decide). Cycles: detect them and throw, or represent them; decide.
- [x] It verifies the layout hash (in the roots cell) before reading.
- [x] Tests: a Rust test writes heap images (`Heap` bytes) for a corpus of values (reuse
      0018's cross-language corpus in reverse: decode wire → heap → image file). Node
      reads them with the reader and compares with the original values. Include strings
      with lone surrogates, Latin-1 above 0x7F, ropes and big strings.
- [x] Benchmarks: reading a 1 MiB string (target: about memcpy speed), 10k similar
      objects, compared with the alternative (VM-side `json::stringify_fast` +
      `TextDecoder` + `JSON.parse` in node). Record the results in the Log and in
      docs/wire.md.

## Notes
- No djs-wasm exists yet, so tests go through heap image files read into a Uint8Array.
  That is the same bytes the reader will see in linear memory (the arena is
  position-independent). Note how the reader gets the arena base offset later.
- 0015 (interpreter) is adding cell kinds right now. The reader should be driven by the
  exported layout, so new kinds need no hand-written changes unless the reader must
  understand them.

## Log

2026-11-01 (num)
- Layout export (D1210): `heap::layout::layout_json()`, additive in `layout.rs`, builds the
  description from `LAYOUTS` plus the header, magic, `LAYOUT_HASH`, value tags, atom bits,
  flags, object classes and property flag bits. `js/src/layout.generated.ts` is checked in, and
  `layout_generated_ts_is_current` fails when it is stale (`DJS_UPDATE_LAYOUT=1` rewrites it).
  `every_valid_tag_is_exported` catches new value tags. Outside `layout.rs` I made four private
  constants `pub(crate)` (`atom::INT_BIT`, `SYM_BIT`, `INDEX_MASK`, `object::STORE_ACCESSOR`)
  so the export reads them instead of copying them.
- `js/src/reader.ts`: `readValue(arena, bits | [lo, hi], opts)`, `checkHeap`, `Opaque`,
  `LayoutMismatch`. All offsets come from the generated layout. Decisions (D1211, table in
  docs/wire.md):
  - ropes are walked iteratively, so the VM need not flatten;
  - array holes stay holes;
  - own enumerable string-keyed data properties in property order, `__proto__` as an own
    property, accessors left out;
  - symbols, other classes, BigInt and host refs become `Opaque`;
  - the result is a tree: a cycle is a TypeError with its path, and a value budget stops
    exponential sharing.
- Cycles: the first version kept sharing with a memo. Profiling showed the memo cost 30% of
  a 10k-object read, and replacing the `Map` with an Int32Array hash table did not help: the
  cost is storing every fresh object into a long-lived table. pi serializes results to JSON,
  so tree semantics lose nothing.
- Strings: node's `Buffer.latin1Slice` / `ucs2Slice` are exact (true Latin-1, lone surrogates
  kept) and run at memcpy speed: 0.025 ms per MiB for any content. `TextDecoder` takes 0.9 ms
  for non-ASCII windows-1252 and 0.6 ms for UTF-16, and `fromCharCode` chunks take 2.2 ms. For
  short strings `latin1Slice` costs a constant ~27 ns, against 50-120 ns for
  `fromCharCode.apply(subarray)`. Strings of 4 units or fewer are built by a loop. There is a
  portable path (`{ portable: true }`, or no `Buffer`) using only standard APIs, and both paths
  are tested.
- Number elements are read through Uint32Array/Float64Array views when the arena view is
  8-aligned: 3.5 -> 2.85 ms for 500k numbers.
- Tests: `reader_images` (Rust, ignored, run by the gate) decodes 0018's cross-language corpus
  into one heap and writes its image, plus a second image of hand-built cases: ropes (plain,
  two-byte with a lone surrogate, 2000 levels deep, flattened), 1 MiB ASCII / Latin-1 / Latin-1
  with C1 bytes / two-byte / lone surrogate strings, a symbol, an object with every property
  kind (accessor, non-enumerable, symbol key, deleted, `__proto__`, out-of-order index keys),
  sharing, two cycles, a 2^40 sharing blow-up, a sparse array, and primitives in both number
  forms. `js/test/reader-cross.ts` checks all 1533 corpus values against node's original dumps
  (also with the portable path and at aligned and unaligned base offsets), the hand-built
  cases, and the layout check.
- Benchmarks: the table is in docs/wire.md. A 1 MiB string reads in 0.01-0.04 ms (memcpy of
  1 MiB: 0.02 ms). 10k objects: 3.0 ms vs 5.6 ms for `stringify_fast` + JSON read +
  `JSON.parse`; tool results 0.46 vs 2.0; 500k numbers 2.85 vs 16.8.

- Rebased onto 0015. Its cell kinds 17-19 (Interp, Realm, HostCall) and object classes
  (Function, HostFunction, Error) came through the export unchanged. The drift test flagged
  its new value tag 13 (`INTERNAL`), which I added to the exported tag list (internal
  values read as `Opaque { type: "internal" }`). Added an Error-class object to the
  hand-built cases.

## Follow-ups
- djs-wasm: export the arena pointer and length (and value bits of results and host-call
  arguments) so the host can build the view; the view must be re-derived after every VM call.
- Opaque objects: decide what pi shows for functions and errors (for example read an Error's
  `message`), once 0015's classes exist in practice.
- If sharing matters to a consumer later (for example passing graphs to host functions), an
  option with a memo is easy to add back. It cost 30% on reads.
- `stringify_fast` on 500k numbers takes 11 ms (number formatting through the generic path);
  irrelevant for this reader, but visible in the JSON baseline.

## Result
- `heap::layout::layout_json()` and `js/src/layout.generated.ts` (kept fresh by a cargo
  test). Regenerate with `DJS_UPDATE_LAYOUT=1 cargo test -p djs-core layout_generated`.
- `js/src/reader.ts`: `readValue(arena: Uint8Array, value: bigint | [lo, hi], opts?)`, where
  `arena` is a view starting at the arena's offset 0. Options: `atomCache`, `portable`,
  `maxValues`. Also `checkHeap`, `Opaque`, `LayoutMismatch`. Semantics in docs/wire.md
  "Outbound".
- Tests in the gate: the layout freshness and drift tests (cargo), and the reader cross test
  (`reader_images` + `js/test/reader-cross.ts`). Benchmarks: `reader_bench_images` +
  `js/bench/reader.bench.ts`.
- Known gaps: no wasm measurement yet (no djs-wasm); sharing is not preserved, by design.
