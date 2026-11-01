# 0039: Binary data: ArrayBuffer, typed arrays, DataView, Uint8Array base64/hex

status: in-progress
agent: infra
branch: issue/0039-binary-data
depends-on: 0038
touches: builtins/ (arraybuffer.ts, typedarray.ts, dataview.ts …), builtins/gen/, crates/djs-core/src/builtins/, crates/djs-core/src/heap/ (new cell kinds: byte buffers, additive), crates/djs-core/src/interp/ (only the integer-indexed exotic object element paths: get/set/has/define/delete/ownKeys for typed arrays, isolated), crates/djs-core/src/wire/ (decode bytes to a real Uint8Array: the 0018 follow-up), js/src/reader.ts (read typed arrays out: the 0021 follow-up), tests/test262.toml (promote binary data to active), ratchets

## Goal
Codemode moves binary data around (images as base64 and bytes; the wire format already
has a bytes tag that decodes to a placeholder). ARCH G1 wants bytes in and out cheaply.

## Acceptance criteria
- [ ] ArrayBuffer (including `resizable`, `maxByteLength`, `resize`, `transfer`,
      `transferToFixedLength`, `detached`, `slice`, `isView`), with backing store in the
      arena (a byte cell; for big buffers, note the copying-GC cost and whether the
      large-object space from 0009 is now needed). `$262.detachArrayBuffer` works via
      a leaf (update djs-test262's `$262`).
- [ ] TypedArray (%TypedArray% and all constructors including Float16Array, BigInt64
      ones only if BigInt exists, otherwise stubbed and logged) and all prototype
      methods, the integer-indexed exotic behavior (element access by canonical
      numeric string keys, out-of-bounds, detached, length-tracking views on resizable
      buffers), `from`/`of`, species.
- [ ] DataView with all getters and setters (Float16 included), both endiannesses.
- [ ] Uint8Array base64/hex: `Uint8Array.fromBase64`, `fromHex`,
      `prototype.toBase64`, `toHex`, `setFromBase64`, `setFromHex` (the
      `uint8array-base64` feature) with all options (alphabet, lastChunkHandling,
      omitPadding), as fast leaf code: codemode moves images as base64.
- [ ] Wire and reader: wire bytes decode into a real Uint8Array (0018 follow-up), and the
      JS reader returns typed arrays as `Uint8Array` copies, or views where safe;
      decide and document in docs/wire.md (0021 follow-up). Benchmarks for a 1 MiB
      image both ways.
- [ ] Promote built-ins/ArrayBuffer, DataView, TypedArray, TypedArrayConstructors and
      Uint8Array (and their features) from later to active; report the numbers before and
      after the move separately.
- [ ] behavior.js vs qjs, difftest cases (base64 round trips of a binary blob,
      DataView parsing), e2e (three modes), and the ratchets.

## Notes
- Land in parts: ArrayBuffer + Uint8Array basics + base64 first (codemode value), then
  the full TypedArray set, then DataView, then resizable.
- heap (0037) is in interp/ and vm/. Keep interp changes to the element paths,
  isolated, and logged.

## Log

### Part 1: ArrayBuffer, typed arrays (core), base64/hex, wire and reader
- test262 before (main 510fec3): active 16,355 / 22,142 (73.8%), later 128 / 15,427.
  The move to active alone (main's engine, this toml): active 16,355 / 24,120 (67.8%),
  later 128 / 13,449: +1,978 active tests, none passing yet.
- ArrayBuffer (resizable, transfer, transferToFixedLength, slice, isView, detached) and
  typed arrays of all twelve kinds (BigInt ones throw "not supported yet"), with the
  integer-indexed exotic behavior in heap/typed.rs (D1390). Outside touches: heap/object.rs
  hooks (isolated `if TypedArray` branches), interp/run.rs `primitive_operands` (ToNumber
  before an element write), djs-test262 (`$262.detachArrayBuffer`).
- %TypedArray% as a realm constructor, a realm constructors table (D1391); core methods:
  from, of, species, buffer/byteLength/byteOffset/length/toStringTag, set, subarray, slice,
  fill. The rest of the prototype comes in part 2.
- Uint8Array base64/hex in Rust (D1392); wire bytes → Uint8Array; the reader returns typed
  arrays and buffers as copies (docs/wire.md); a 1 MiB image benchmark (bytes: decode
  0.017 ms, read 0.022 ms in wasm).
- behavior.js: ArrayBuffer, typed array constructors and core methods, base64/hex vs qjs
  (behavior.rs keeps `% ` remainders now that builtins use them). difftest
  builtins/base64.js (every lastChunkHandling edge case, setFromBase64 partial writes),
  builtins/typed-arrays.js; e2e typed_arrays.js (three modes).
- After part 1: active 17,276 / 24,120 (71.6%); ArrayBuffer 161/179, TypedArray 323/849,
  TypedArrayConstructors 340/360, Uint8Array 66/68, DataView 0/449. difftest 102 / 185.
- Surprise: the release wasm module is 1.2 MB (D1321 said 564 KB): the prelude has grown
  a lot since (it is compiled into the embedded base); worth a look by the coordinator.

## Follow-ups

## Result
