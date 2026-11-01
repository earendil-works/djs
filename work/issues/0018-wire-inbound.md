# 0018: Inbound wire format (host → VM)

status: done
agent: num
branch: issue/0018-wire-inbound
depends-on: 0012
touches: crates/djs-core/src/wire/, crates/djs-core/src/heap/ (small additive helpers only), js/ (new package: the encoder only), docs/wire.md, crates/djs-core/src/lib.rs (mod lines only)

## Goal
ARCH.md "Data passing (G1)" → "Inbound, wire format": the host encodes a JS value into a
staging buffer in linear memory, and the VM materializes it in one native pass. It has
to beat the baseline (host `JSON.stringify` + VM `json::parse_utf8` from 0012) in
benchmarks to justify itself; if it doesn't, say so with the numbers (that is a valid
result).

## Acceptance criteria
- [x] `docs/wire.md`: the format spec. Tagged binary, little-endian. Primitives (undefined,
      null, bools, int32 and f64, -0), strings as UTF-8 written with
      `TextEncoder.encodeInto`, a UTF-16 string variant for strings with lone surrogates,
      arrays, objects with a **key table** (repeated keys interned once, referenced by
      index), `Uint8Array`/bytes (decoded into a placeholder until typed arrays exist;
      document this), and a reserved host-ref tag. Versioned header. Designed so the
      encoding can be swapped (ARCH).
- [x] `js/` package (TypeScript, no dependencies, ES2022, node ≥ 20): `encode(value,
      buffer?) -> Uint8Array`, which grows the buffer as needed, detects cycles (an error)
      and handles the edge cases (`toJSON`? decide and document: probably not, since
      this is structured data, not JSON). Node tests with `node --test`.
- [x] Rust decoder `wire::decode(heap, bytes) -> Result<Value, WireError>`: one pass, no
      recursion (an explicit stack), UTF-8 transcoded straight into one-byte/two-byte
      heap strings, keys interned through the key table into atoms once, objects and
      arrays built with 0012's bulk helpers. It validates everything (corrupt input is
      an error, never a panic or UB) and follows the GC contract (no collection during
      decode).
- [x] A fuzz-ish test: random and corrupted buffers never panic.
- [x] Cross-language test: node encodes a corpus of values to files (a checked-in
      script; outputs in work/scratch or generated in the test), Rust decodes them and
      compares with `json::parse` of the same values (where they are representable).
- [x] Benchmarks (native release for the decoder, node for the encoder) against
      `JSON.stringify` (node) + `json::parse_utf8` (djs), on: 10k similar objects, one 1
      MiB string, mixed codemode-like tool results, and a big array of numbers. Record the
      results in the Log and in docs/wire.md. A wasm measurement is not possible yet (no
      djs-wasm); note how to do it later.

## Notes
- Outbound (VM → host direct read) is a separate later issue; do not build it here, but
  keep the format choices compatible with a JS reader that walks the heap.
- Keep `js/` minimal: package.json, tsconfig, src/wire.ts, tests. No bundler.
  `node_modules/` is gitignored (add it if missing). If you need TypeScript to build,
  node's type stripping (`node --experimental-strip-types` or the default in node 25)
  may avoid needing a tsc install for tests.

## Log

2026-11-01 (num)
- Read 0012's `json::parse` (explicit stack, bulk constructors, string allocation at the
  final width) and the heap string/atom/object APIs. Everything needed already exists:
  `alloc_str_uninit` + `str_data_offset`, `atom_str` / `atom_utf16`,
  `new_object_from_entries`, `new_array_from_values`. **No heap changes were needed.**
- Format (docs/wire.md, D1180): `DJW` + version header, tags for undefined/null/bools/int8/
  int32/float64/hole, UTF-8 strings with their UTF-16 length up front (ASCII when bytes ==
  units) plus a UTF-16 form for lone surrogates, arrays, objects with an inline key table
  (first use carries the string, later uses an index), bytes, and a reserved host-ref tag.
  Varints may be zero-padded, so the encoder can reserve the byte length before `encodeInto`
  tells it. Bytes decode to an array of numbers until typed arrays exist.
- `js/` package: `package.json` (no dependencies), `tsconfig.json` (type-checks `src/` with
  tsc), `src/wire.ts` (`encode(value, buffer?)`), `test/wire.test.ts` (node --test, with a
  reference decoder for round trips; 12 tests), `test/gen-cross.ts`, `bench/wire.bench.ts`.
  Decisions: no `toJSON`; only plain or null-prototype objects, arrays, `Uint8Array` and
  primitives. Anything else, and cycles, throw a TypeError with a path such as
  `cannot encode Date at $.a[1].when`. The path costs nothing on success, because it is built
  while the error unwinds. Cycles are only looked for below depth 32 (a cycle always gets
  there), so shallow data pays nothing. Added `node_modules/` to .gitignore.
- Rust `wire::decode(heap, bytes, &DecodeOptions)`: one pass, explicit stack, containers
  built in one allocation when complete, keys interned once per buffer, strings allocated at
  their final width and filled from the input. It validates everything: counts against the
  remaining input before allocating, varints, UTF-8 and unit counts, key indexes, depth
  (5000, as JSON), trailing bytes. Corrupt input gives `WireError::Invalid { offset, reason }`.
- Tests: per-feature decoder tests with a Rust reference encoder; validation errors; the depth
  limit (built without recursion, since the test helpers recurse); random round trips; a
  fuzz test (12k mutated buffers in the gate, 2M in the ignored `fuzz_long`) that checks no
  panic and `Heap::verify` after every successful decode; the cross-language `wire_cross`
  test: 1533 node-encoded values, compared with the node-side dump, and 1055 of them also
  with `json::parse_utf8` of `JSON.stringify` output. The gate now runs the node tests and
  `wire_cross` (D1181). That is a small change to `scripts/gate.sh`, outside this issue's
  touches, as TESTING.md asks for new test suites.
- Benchmarks: the table is in docs/wire.md. The wire format wins end to end on every dataset:
  10k objects 5.6 vs 6.8 ms, tool results 1.28 vs 1.84, 500k numbers 3.9 vs 11.8, 1 MiB
  non-ASCII text 0.73 vs 2.51, 1 MiB string 0.05 vs 0.87. Buffers are 20-35% smaller. The
  encoder alone is 1.5x slower than `JSON.stringify` + `encodeInto` on many small objects
  (about 24 ns per value); the decoder makes up for it (2.97 vs 5.00 ms).
- Things that mattered: benchmark data built with template literals gives V8 cons strings,
  which made `charCodeAt` loops 2x slower. Real tool results come from `JSON.parse`, so the
  datasets go through it. A hand-written UTF-8 loop for strings of 64 units or less beats
  `TextEncoder.encodeInto` plus `isWellFormed` by about 2.5x. In the decoder, `char`-iterator
  transcoding was slow (non-ASCII text 2.0 ms); a manual transcoder with an 8-byte ASCII
  fast path brought it to 0.54. `from_utf8` on short strings was the top profile entry;
  an `is_ascii` check for the bytes == units case replaced it.
- Tried and not kept (each about 5% or less on 10k objects, not worth the complexity):
  reusing key indexes from the previous object at the same depth, `for...in` instead of
  `Object.keys` (needs a padded count and a polluted-prototype fallback), and a
  `constructor === Object` shortcut for the plain-object check.
- wasm size (opt-level z, LTO, stripped): heap alone 9.7 KB; heap + `wire::decode` 21.7 KB;
  heap + `json::parse_utf8` 33.1 KB.
- wasm timing is not possible yet (no djs-wasm). The spec says how to measure it later.

## Follow-ups
- Measure both pipelines in wasm once djs-wasm exists (method in docs/wire.md).
- Decode `BYTES` into a real `Uint8Array` once typed arrays exist; host refs (tag 0x3F).
- Heap: decoding objects is now dominated by allocation. Zero-filling of cells that are
  overwritten right away (strings), and arena growth on a fresh heap, show up in profiles.
  An uninitialized allocation for string data and an arena `reserve` would help `json::parse`
  too (heap issue).
- The encoder on rows of small objects is 1.5x slower than V8's `JSON.stringify`.
  Interning repeated string values (like keys) would also shrink buffers. It needs a format
  version bump and a measurement.
- The encoder recurses, so nesting is limited by the JS stack (about 10k levels in node), as
  with `JSON.stringify`. The decoder limit is 5000.

## Result
- Spec: `docs/wire.md` (format v1, encoder rules, validation, benchmarks).
- Host: `js/src/wire.ts`. `encode(value, buffer?) -> Uint8Array` writes into `buffer` when it
  is big enough (pass a view of wasm memory), otherwise into a grown copy, and returns the
  bytes written. TypeErrors name the path.
- VM: `djs_core::wire::decode(heap, bytes, &DecodeOptions { object_proto, array_proto,
  max_depth }) -> Result<Value, WireError>`. It never collects; check `gc_pending`
  afterwards.
- Tests: `node --test js/test/*.test.ts`, `cargo test -p djs-core wire`, and the gate's
  cross-language step. Benchmarks: `node js/bench/wire.bench.ts <dir>` and then
  `DJS_WIRE_BENCH=<dir> cargo test -p djs-core --release bench_wire -- --ignored
  --nocapture`.
- Known gaps: bytes become an array of numbers, host refs are rejected, there is no wasm
  measurement yet.
