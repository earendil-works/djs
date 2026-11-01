# 0039: Binary data: ArrayBuffer, typed arrays, DataView, Uint8Array base64/hex

status: open
agent: -
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

## Follow-ups

## Result
