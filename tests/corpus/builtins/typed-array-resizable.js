// Typed arrays over resizable buffers and [[Set]] with values that need ToNumber (0090).
{
  // [[Set]] through builtins (Array.prototype.fill, Reflect.set) converts the value first.
  const ta = new Uint8Array(4);
  ta[1] = { valueOf() { return 7; } };
  Array.prototype.fill.call(ta, { valueOf() { return 5; } }, 0, 2);
  console.log(ta.join(), Reflect.set(ta, 3, { valueOf: () => 9 }), ta[3]);
  const big = new BigInt64Array(2);
  Array.prototype.fill.call(big, { valueOf() { return 3n; } });
  console.log(big.join());
  try { Array.prototype.fill.call(new Uint8Array(1), Symbol()); } catch (e) { console.log(e.constructor.name); }
  try { Array.prototype.fill.call(new BigInt64Array(1), 1); } catch (e) { console.log(e.constructor.name); }
  // The conversion shrinks the buffer: the store is skipped.
  const rab = new ArrayBuffer(4, { maxByteLength: 8 });
  const f = new Uint8Array(rab, 0, 4);
  Array.prototype.fill.call(f, { valueOf() { rab.resize(2); return 3; } }, 1, 2);
  console.log(f.length, new Uint8Array(rab).join());
  const rab2 = new ArrayBuffer(4, { maxByteLength: 8 });
  const g = new Uint8Array(rab2);
  g[3] = { valueOf() { rab2.resize(2); return 3; } };
  console.log(g.length, g.join(), Object.keys(g).join());
}
{
  // subarray of an out-of-bounds view uses its [[ByteOffset]]: RangeError.
  const rab = new ArrayBuffer(4, { maxByteLength: 8 });
  const f = new Uint8Array(rab, 2, 2);
  const lt = new Uint8Array(rab, 2);
  rab.resize(1);
  for (const v of [f, lt]) {
    try { console.log(v.subarray(0).length); } catch (e) { console.log(e.constructor.name, v.length, v.byteOffset); }
  }
  rab.resize(4);
  console.log(f.subarray(1).join(), lt.subarray(-1).length);
}
{
  // Views of resizable buffers cannot be made non-extensible.
  const rab = new ArrayBuffer(4, { maxByteLength: 8 });
  for (const ta of [new Uint8Array(rab, 0, 0), new Uint8Array(rab, 4), new Uint8Array(new ArrayBuffer(0)), new Uint8Array(rab, 0, 2), new Uint8Array(rab.transferToFixedLength(), 0, 0)]) {
    try { Object.freeze(ta); console.log("froze", Object.isFrozen(ta)); } catch (e) { console.log(e.constructor.name, e.message, Object.isExtensible(ta)); }
    console.log(Reflect.preventExtensions(ta), Object.isExtensible(ta));
    try { Object.seal(ta); console.log("sealed"); } catch (e) { console.log(e.constructor.name); }
  }
}
