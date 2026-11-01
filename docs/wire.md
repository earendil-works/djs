# Inbound wire format (host → VM)

ARCH.md, "Data passing (G1)": the host encodes a JS value into a byte buffer
(`js/src/wire.ts`, `encode`), and the VM materializes it in one native pass
(`djs_core::wire::decode`). This document is the format specification, version 1.

Design goals, in order: a fast JS encoder (one pass, typed-array writes, `TextEncoder`
for long strings), a fast one-pass decoder that allocates every heap cell at its final size,
and a compact encoding. The format is an implementation detail behind `encode`/`decode`; the
version byte lets it change.

## Layout

All multi-byte numbers are little-endian. A buffer is a 4-byte header followed by exactly
one value; anything after that value is an error.

```
header  = 'D' 'J' 'W' version      ; 0x44 0x4A 0x57, version = 0x01
value   = tag payload
```

`varint` is unsigned LEB128 of at most 5 bytes whose value fits in a `u32`. Non-canonical
(zero-padded) encodings are allowed, so that an encoder can reserve space for a length it
learns later: `5` may be written as `0x85 0x00`.

| Tag    | Value                | Payload                                                    |
| ------ | -------------------- | ---------------------------------------------------------- |
| `0x00` | `undefined`          |                                                            |
| `0x01` | `null`               |                                                            |
| `0x02` | `false`              |                                                            |
| `0x03` | `true`               |                                                            |
| `0x04` | int32 number         | `i32`                                                      |
| `0x05` | float64 number       | `f64` (any value: `-0`, NaN, infinities)                   |
| `0x06` | array hole           | (only as an array element)                                 |
| `0x07` | int8 number          | `i8`                                                       |
| `0x10` | string, UTF-8        | `varint units`, `varint bytes`, `bytes` of UTF-8           |
| `0x11` | string, UTF-16       | `varint units`, `units` × `u16`                            |
| `0x20` | array                | `varint count`, `count` × value                            |
| `0x21` | object               | `varint count`, `count` × (key, value)                     |
| `0x30` | bytes (`Uint8Array`) | `varint length`, `length` bytes                            |
| `0x3F` | host ref (reserved)  | `varint id`                                                |

Other tags are invalid.

### Numbers

An encoder writes int8 for integers in -128..127, int32 for other integers that fit in
an `i32`, and float64 for everything else, including `-0`. A decoder accepts any of the
three for any number. int8 and int32 decode to the VM's int32 value form.

### Strings

`units` is the length in UTF-16 code units (the JS `length`), so the decoder can allocate
the heap string before it reads the data. A UTF-8 string must be valid UTF-8 (no
surrogate code points) and decode to exactly `units` code units; this needs
`bytes / 3 <= units <= bytes`. If `bytes == units` the string is ASCII and is copied as is.
Otherwise the decoder scans for a lead byte `>= 0xC4` (a code point above U+00FF) to choose
a one-byte or two-byte heap string, and transcodes into it.

The UTF-16 form carries any code unit sequence. An encoder uses it only for strings that
are not well-formed (lone surrogates), which UTF-8 cannot represent; `TextEncoder` would
replace them with U+FFFD. The decoder stores it one-byte if every unit is at most 0xFF.

### Objects and the key table

Each property is a key followed by its value. A key is a `varint k`:

- `k = 0`: a new key follows, as a string value (tag `0x10` or `0x11` and its payload). It is
  appended to the key table.
- `k >= 1`: entry `k - 1` of the key table.

The key table starts empty and grows across the whole buffer, so a key that occurs in 10,000
objects is transmitted, decoded and interned once. Keys become property keys exactly like
`CreateDataProperty` with that string (`"0"` is the index 0). A decoder treats a repeated key
within one object like JS does: the first position, the last value. Objects get the realm's
`Object.prototype`, arrays `Array.prototype`.

### Arrays

`count` elements, each a value or a hole (`0x06`). Extra non-index properties of a JS array
are not encoded (as with JSON).

### Bytes

A `Uint8Array` (only that type; other typed arrays and `ArrayBuffer`s are not encodable).
Until the VM has typed arrays, the decoder turns bytes into an ordinary array of numbers
(0..255), which has the same `[i]` and `length` behavior but costs 8 bytes of heap per byte.
This changes to a real `Uint8Array` when typed arrays exist.

### Host refs

Reserved for opaque handles to host objects (ARCH "Host refs (later)"). A decoder rejects
it until host refs exist.

## What the encoder accepts

`encode` takes structured data, not arbitrary JS values, and does not call `toJSON`
(this is data for the VM, not JSON):

- `undefined`, `null`, booleans, numbers, strings;
- arrays (`Array.isArray`), encoded with holes as holes;
- `Uint8Array`;
- plain objects: prototype `Object.prototype` or `null`. Their own enumerable string keys
  are encoded in `Object.keys` order (which is also the VM's property order). Getters are
  called. Symbol keys are ignored.

Anything else (functions, symbols, BigInt, class instances, `Date`, `Map`, `Set`, `RegExp`,
`Error`, other typed arrays, ...) is a `TypeError` that names the path, e.g.
`cannot encode Date at $.items[3].when`. A cycle is a `TypeError` too. Shared (non-cyclic)
references are encoded once per occurrence; the VM gets independent copies. Nesting is limited
by the JS stack of the encoder (it recurses) and by the decoder's depth limit (5000, as
`JSON.parse` in djs).

## Validation

The decoder checks everything: the header, every tag and length against the remaining
input, varint sizes, UTF-8 validity and unit counts, key indexes, the depth limit and
trailing bytes. Counts are checked against the remaining input before anything is
allocated (each array element takes at least one byte, each property at least two), so a
small corrupt buffer cannot make the decoder allocate a lot. Corrupt input is an error
with an offset; it never panics. Heap exhaustion is reported separately. The decoder
allocates but never collects (the GC contract); everything it builds is reachable from the
result only.

## Benchmarks

Host side: node 25 (V8), `js/bench/wire.bench.ts`: `encode` against `JSON.stringify` and
against `JSON.stringify` + `TextEncoder.encodeInto`. The latter is the real baseline, because a
wasm host has to copy the JSON text into linear memory as UTF-8. VM side: native release
build (Apple M-series), `bench_wire` in `crates/djs-core/src/wire/tests.rs`: `wire::decode`
against `json::parse_utf8` on the same data. Both decode into a fresh heap each time. Times
are medians in milliseconds. The datasets go through `JSON.parse` first, like real tool
results, which gives V8 flat strings. String-building code would give cons strings, which slow
both encoders down.

| dataset | encode | stringify + encodeInto | decode | parse_utf8 | wire total | JSON total | wire KB | JSON KB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10k similar objects (9 fields) | 2.66 | 1.79 | 2.97 | 5.00 | **5.6** | 6.8 | 1222 | 1850 |
| one 1 MiB ASCII string | 0.02 | 0.36 | 0.03 | 0.51 | **0.05** | 0.87 | 1024 | 1024 |
| 1 MiB source text with some non-ASCII | 0.19 | 1.12 | 0.54 | 1.39 | **0.73** | 2.51 | 1062 | 1079 |
| tool results (2000 hits + 200 KB file) | 0.65 | 0.65 | 0.63 | 1.19 | **1.28** | 1.84 | 496 | 614 |
| 500k numbers (half floats) | 2.84 | 5.18 | 1.10 | 6.59 | **3.9** | 11.8 | 3418 | 4335 |

The wire format wins end to end on every dataset: 1.2x for many small objects, 1.4x for
mixed tool results, 3x for numbers and non-ASCII text, and 17x for one big string, which is
two memcpys instead of an escape-scanning serializer plus a parser. The one loss is the
encoder on many small objects: V8's native `JSON.stringify` is 1.5x faster than the JS
encoder there (about 24 ns per value). The decoder more than makes up for it. Most of the
decoder's remaining time on objects is heap allocation, which both paths share. The
buffers are 20-35% smaller than JSON for structured data.

In wasm, the decode columns will be slower than native. This can be measured once djs-wasm
exists: export a `decode` and a `parse_json` entry point, have the host `encode` into a view
of linear memory (`encode(value, new Uint8Array(memory.buffer, ptr, cap))`; if the
result is not in that buffer, it grew and must be copied in), and time both pipelines from
node with the same datasets (`node js/bench/wire.bench.ts <dir>` writes them).

## Code and tests

- Encoder: `js/src/wire.ts` (`encode(value, buffer?)`), tests `node --test js/test/*.test.ts`
  (node >= 22.6 runs the TypeScript directly).
- Decoder: `crates/djs-core/src/wire/` (`wire::decode(heap, bytes, &DecodeOptions)`), with unit
  tests, a fuzz test (`fuzz_long` is the longer ignored run) and `wire_cross`: node writes a corpus
  (`node js/test/gen-cross.ts <dir>`) and the Rust test decodes it and compares it with the node
  side's dump and with `json::parse_utf8` of the same value. The gate runs the node tests and
  `wire_cross`.
