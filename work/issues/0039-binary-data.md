# 0039: Binary data: ArrayBuffer, typed arrays, DataView, Uint8Array base64/hex

status: done
agent: infra
branch: issue/0039-binary-data
depends-on: 0038
touches: builtins/ (arraybuffer.ts, typedarray.ts, dataview.ts …), builtins/gen/, crates/djs-core/src/builtins/, crates/djs-core/src/heap/ (new cell kinds: byte buffers, additive), crates/djs-core/src/interp/ (only the integer-indexed exotic object element paths: get/set/has/define/delete/ownKeys for typed arrays, isolated), crates/djs-core/src/wire/ (decode bytes to a real Uint8Array: the 0018 follow-up), js/src/reader.ts (read typed arrays out: the 0021 follow-up), tests/test262.toml (promote binary data to active), ratchets

## Goal
Codemode moves binary data around (images as base64 and bytes; the wire format already
has a bytes tag that decodes to a placeholder). ARCH G1 wants bytes in and out cheaply.

## Acceptance criteria
- [x] ArrayBuffer (including `resizable`, `maxByteLength`, `resize`, `transfer`,
      `transferToFixedLength`, `detached`, `slice`, `isView`), with backing store in the
      arena (a byte cell; for big buffers, note the copying-GC cost and whether the
      large-object space from 0009 is now needed). `$262.detachArrayBuffer` works via
      a leaf (update djs-test262's `$262`).
- [x] TypedArray (%TypedArray% and all constructors including Float16Array, BigInt64
      ones only if BigInt exists, otherwise stubbed and logged) and all prototype
      methods, the integer-indexed exotic behavior (element access by canonical
      numeric string keys, out-of-bounds, detached, length-tracking views on resizable
      buffers), `from`/`of`, species.
- [x] DataView with all getters and setters (Float16 included), both endiannesses.
- [x] Uint8Array base64/hex: `Uint8Array.fromBase64`, `fromHex`,
      `prototype.toBase64`, `toHex`, `setFromBase64`, `setFromHex` (the
      `uint8array-base64` feature) with all options (alphabet, lastChunkHandling,
      omitPadding), as fast leaf code: codemode moves images as base64.
- [x] Wire and reader: wire bytes decode into a real Uint8Array (0018 follow-up), and the
      JS reader returns typed arrays as `Uint8Array` copies, or views where safe;
      decide and document in docs/wire.md (0021 follow-up). Benchmarks for a 1 MiB
      image both ways.
- [x] Promote built-ins/ArrayBuffer, DataView, TypedArray, TypedArrayConstructors and
      Uint8Array (and their features) from later to active; report the numbers before and
      after the move separately.
- [x] behavior.js vs qjs, difftest cases (base64 round trips of a binary blob,
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

### Part 2: the rest of %TypedArray%.prototype, DataView
- %TypedArray%.prototype complete: at, copyWithin, entries/keys/values/@@iterator (the
  Array Iterator; its next now validates a typed array: TypeError once detached, internal
  length), every, filter, find/findIndex/findLast/findLastIndex, forEach, includes,
  indexOf, join, lastIndexOf, map, reduce/reduceRight, reverse, some, sort (default order
  via a new `%SortNumbers` leaf: -0 before +0, NaN last), toLocaleString, toReversed,
  toSorted, toString (the same function as Array.prototype.toString), with. slice into the
  same buffer copies forwards per element (the spec's byte loop).
- ToNumber of an element value where the spec converts: `TASet` in builtins (Set(A, k,
  v) on typed arrays), DefineOwnProperty in object.ts (convert after the descriptor
  checks), Reflect.set in reflect.ts (when [[Set]] ends at the typed-array receiver).
  [[Set]] that reaches a typed array without that element is a no-op success
  (`Heap::typed_array_set_stops`, used by run.rs `set_property` and `ordinary_set`).
- Outside touches (minimal, logged): builtins/object.ts DefineOwnProperty, reflect.ts
  ReflectSet, iter.ts ArrayIteratorPrototypeNext, array.ts (`@builtin
  %TypedArray%.prototype.toString` on Array.prototype.toString), array.rs
  `sort_numbers`, and interp/run.rs `construct`: a builtin constructor no longer reads
  `newTarget.prototype` for a `this` it ignores. It used to, so a `prototype` getter ran
  (or threw "not supported yet") before the builtin's own GetPrototypeFromConstructor:
  ordering bugs in all builtin constructors (D1393).
- DataView: constructor (both detach/bounds checks around the prototype lookup),
  buffer/byteLength/byteOffset, toStringTag, get/set for all nine Number types incl.
  Float16, both endiannesses, length-tracking views; BigInt64/BigUint64 throw "not
  supported yet: BigInt typed arrays". Leaves DataViewCreate/Field/Get/Set. Messages
  follow qjs except an out-of-bounds (resized) view, which is a TypeError per spec (qjs:
  RangeError "out of bound").
- behavior.js: stand-ins for the DataView leaves and SortNumbers; comparisons for every
  new method. difftest builtins/dataview.js, builtins/typed-arrays.js extended.
- Final numbers (active 24,120): 18,297 pass (75.8%); before 0039 (old toml) 16,355 /
  22,142 (73.8%); later 252 / 13,449. ArrayBuffer 173/179, DataView 445/449, TypedArray
  841/849, TypedArrayConstructors 357/360, Uint8Array 66/68, ArrayIteratorPrototype
  18/19. Every remaining failure in these directories needs BigInt, RegExp, classes or
  generators (19 tests).
- GC cost: buffers are copied by every collection like any cell (memcpy speed, about 1
  ms per 10 MiB of live buffers on this machine). For codemode's images (a few MiB, short
  lived) this is fine; a large-object space (0009) pays off only once VMs hold tens of
  MiB of long-lived buffers. Not needed now; listed as a follow-up.

## Follow-ups
- BigInt64Array/BigUint64Array and DataView get/setBigInt64 when BigInt lands (the
  constructors and methods exist and throw "not supported yet").
- A large-object space for big ArrayBuffers (0009) if heaps start keeping tens of MiB of
  bytes alive across collections.
- immutable-arraybuffer (the proposal) stays later.
- djs's console display of typed arrays matches qjs only via Array.from (console shows
  NaN/-0 in arrays differently; not specific to 0039).
- The release wasm module is 1.2 MB (D1321 measured 564 KB): the embedded prelude grew.

## Result
Binary data is complete apart from BigInt: ArrayBuffer (resizable, transfer,
transferToFixedLength, detached, slice, isView, species), all twelve typed array
constructors (BigInt ones stubbed) with the integer-indexed exotic behavior in the heap,
the whole %TypedArray% and %TypedArray%.prototype, DataView with every Number getter and
setter, and Uint8Array base64/hex as leaf code. Wire bytes decode to a Uint8Array; the
JS reader returns typed arrays and ArrayBuffers as copies (docs/wire.md, a 1 MiB image
benchmark: 0.017 ms decode, 0.022 ms read in wasm). `$262.detachArrayBuffer` uses
`Heap::detach_buffer`. Decisions D1390 to D1393. test262 active 75.8% (18,297 / 24,120)
with binary data promoted; the binary-data directories pass at 97 to 99%, with the rest
blocked on BigInt, RegExp, classes and generators.
