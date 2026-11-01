# 0018: Inbound wire format (host → VM)

status: in-progress
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
- [ ] `docs/wire.md`: the format spec. Tagged binary, little-endian. Primitives (undefined,
      null, bools, int32 and f64, -0), strings as UTF-8 written with
      `TextEncoder.encodeInto`, a UTF-16 string variant for strings with lone surrogates,
      arrays, objects with a **key table** (repeated keys interned once, referenced by
      index), `Uint8Array`/bytes (decoded into a placeholder until typed arrays exist;
      document this), and a reserved host-ref tag. Versioned header. Designed so the
      encoding can be swapped (ARCH).
- [ ] `js/` package (TypeScript, no dependencies, ES2022, node ≥ 20): `encode(value,
      buffer?) -> Uint8Array`, which grows the buffer as needed, detects cycles (an error)
      and handles the edge cases (`toJSON`? decide and document: probably not, since
      this is structured data, not JSON). Node tests with `node --test`.
- [ ] Rust decoder `wire::decode(heap, bytes) -> Result<Value, WireError>`: one pass, no
      recursion (an explicit stack), UTF-8 transcoded straight into one-byte/two-byte
      heap strings, keys interned through the key table into atoms once, objects and
      arrays built with 0012's bulk helpers. It validates everything (corrupt input is
      an error, never a panic or UB) and follows the GC contract (no collection during
      decode).
- [ ] A fuzz-ish test: random and corrupted buffers never panic.
- [ ] Cross-language test: node encodes a corpus of values to files (a checked-in
      script; outputs in work/scratch or generated in the test), Rust decodes them and
      compares with `json::parse` of the same values (where they are representable).
- [ ] Benchmarks (native release for the decoder, node for the encoder) against
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

## Follow-ups

## Result
