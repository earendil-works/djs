# 0021: Outbound direct heap reader (VM → host)

status: open
agent: -
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
- [ ] Layout export: a function on the Rust side that serializes `LAYOUTS`, the value tag
      encoding, `LAYOUT_HASH` and the relevant constants (well-known atom ids are not
      needed; the atom table cell layout is) into a compact JSON description. A checked-in
      generated file `js/src/layout.generated.ts` (or `.json`) with a freshness test in
      cargo (like bgen's `--check`).
- [ ] `js/src/reader.ts`: `readValue(memory: Uint8Array, valueBits: bigint | [lo, hi],
      opts) -> unknown` that decodes primitives, strings (flat one-byte and two-byte,
      ropes by walking them; ARCH says ropes are flattened first, so decide whether the
      reader walks ropes or requires flattened input, and document it), arrays (dense,
      holes as `undefined` or skipped: decide), plain objects (own enumerable string-keyed
      data properties in spec order, via the atom table to get key strings, with an atom
      key-string cache per read), and symbols/functions/others as a typed placeholder
      or error (decide). Cycles: detect them and throw, or represent them; decide.
- [ ] It verifies the layout hash (in the roots cell) before reading.
- [ ] Tests: a Rust test writes heap images (`Heap` bytes) for a corpus of values (reuse
      0018's cross-language corpus in reverse: decode wire → heap → image file). Node
      reads them with the reader and compares with the original values. Include strings
      with lone surrogates, Latin-1 above 0x7F, ropes and big strings.
- [ ] Benchmarks: reading a 1 MiB string (target: about memcpy speed), 10k similar
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

## Follow-ups

## Result
