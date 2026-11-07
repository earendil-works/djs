# 0244: TextEncoder and TextDecoder (UTF-8) (B1)

status: open
agent: -
branch: issue/0244-text-encoding
depends-on: -
touches: builtins/ (a new encoding.ts), builtins/intrinsics.d.ts, crates/djs-core/src/interp/run/intrinsics.rs (leaf intrinsics), crates/djs-core/src/interp/realm.rs (ABSENT_GLOBALS for slim), tests/builtins/registry*.tsv/txt, tests/corpus (cases), tests/scores/size.tsv (only if D3172 allows)
guide: docs/guide/language.md (builtins list)

## Goal

D3172. `TextEncoder` (`encode`, `encodeInto`, `encoding`) and `TextDecoder` (`utf-8` only: the
Encoding standard's labels for UTF-8, `fatal`, `ignoreBOM`, `decode(input, { stream })` with
state kept across calls, `encoding`/`fatal`/`ignoreBOM` getters; input any BufferSource:
ArrayBuffer, typed arrays, DataView), per the WHATWG Encoding standard (WebIDL conversions
and error types as browsers/node: RangeError for unsupported labels, TypeError for fatal decode
errors with node's/the spec's message where one exists). Lone surrogates encode as U+FFFD.
Over leaf intrinsics that reuse the engine's UTF-8 transcoders (wire decode, `str_to_string`,
the base64/hex code of Uint8Array) where possible, fuel-charged by bytes.

Full build only (`// @slim-omit`, globals in `realm::ABSENT_GLOBALS`, registry-slim). Measure the
shipped module's size: if it costs more than 1.5 KB gzip, stop and report (D3172 says to cut it
from the shipped module like a slim family then); otherwise raise `ceiling_gzip` by exactly the
measured amount with a size decision (as D3164 does), approved: coordinator.

## Acceptance criteria
- [ ] both classes with the full option surface above; `toString` tags, `length`s, property
      attributes as WebIDL classes in browsers (check against node)
- [ ] tests: corpus cases where qjs can be the oracle are not possible (qjs has no
      TextEncoder): use e2e/unit tests with expectations checked against node, and mark them so
- [ ] registry and slim updates; size measured and recorded
- [ ] performance: encoding/decoding 64 KiB ASCII and non-ASCII is memcpy-like / one pass
      (measure; native release)

## Log

## Follow-ups

## Result
