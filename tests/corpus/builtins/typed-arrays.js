// Typed arrays and ArrayBuffer (0039).
const b = new ArrayBuffer(8);
const u = new Uint8Array(b);
u[0] = 255; u[1] = 256; u[2] = -1; u[3] = 3.7; u["4"] = 4; u[100] = 9; u[-1] = 9;
console.log(u.length, u.byteLength, u[0], u[1], u[2], u[3], u[4], u[100], u["-0"], Object.keys(u), b.byteLength);
const i16 = new Int16Array(b); console.log(i16[0], i16.length, i16.BYTES_PER_ELEMENT, Int16Array.BYTES_PER_ELEMENT);
const f = new Float64Array([1.5, -2, NaN]); console.log(f[0], f[1], f[2], f.length, Array.from(f));
const c = new Uint8ClampedArray([300, -5, 1.5, 2.5, 254.5]); console.log(Array.from(c));
const h = new Float16Array([1, 65504, 65520, 0.1, -0, 1e-8]); console.log(Array.from(h));
console.log(Object.prototype.toString.call(u), u[Symbol.toStringTag], Object.getPrototypeOf(Uint8Array).name, Object.getPrototypeOf(Uint8Array.prototype) === Object.getPrototypeOf(Int8Array.prototype));
const s = u.subarray(2, 5); s[0] = 7; console.log(u[2], s.length, s.byteOffset, s.buffer === b);
const sl = u.slice(1, 3); sl[0] = 1; console.log(u[1], Array.from(sl));
const t = new Uint8Array(4); t.set([1, 2], 1); t.set(new Uint8Array([9]), 3); console.log(Array.from(t), Array.from(new Uint8Array(4).fill(7, 1, 3)));
console.log(new Uint8Array([104, 105, 33]).toBase64(), new Uint8Array([251, 255]).toBase64({ alphabet: "base64url" }), new Uint8Array([1]).toBase64({ omitPadding: true }), new Uint8Array([0xab, 0x01]).toHex());
console.log(Array.from(Uint8Array.fromBase64("aGkh")), Array.from(Uint8Array.fromBase64(" aG k=", { lastChunkHandling: "loose" })), Array.from(Uint8Array.fromHex("ff00")));
const into = new Uint8Array(3); const r1 = into.setFromBase64("aGkhZm9v"); const r2 = new Uint8Array(4).setFromHex("0102"); console.log(r1.read, r1.written, Array.from(into), r2.read, r2.written);
const r = new ArrayBuffer(4, { maxByteLength: 16 }); const tr = new Uint8Array(r); r.resize(8); console.log(tr.length, r.byteLength, r.resizable, r.maxByteLength); r.resize(2); console.log(tr.length);
const x = new ArrayBuffer(4); const xv = new Uint8Array(x); xv[0] = 5; const y = x.transfer(8); console.log(x.detached, x.byteLength, xv.length, xv[0], y.byteLength, new Uint8Array(y)[0]);
console.log(ArrayBuffer.isView(u), ArrayBuffer.isView(b), b.slice(1, 3).byteLength, Uint8Array.of(1, 2).length, Uint8Array.from([1, 2], (v) => v * 10)[1]);
for (const f of [() => new ArrayBuffer(-1), () => ArrayBuffer(1), () => new Uint8Array(-1), () => new Uint16Array(new ArrayBuffer(3)), () => new Uint16Array(new ArrayBuffer(4), 1), () => Uint8Array.fromHex("1"), () => Uint8Array.fromBase64("A"), () => new Uint8Array(2).set([1, 2, 3]), () => Uint8Array.prototype.toBase64.call(new Int8Array(1)), () => Object.getPrototypeOf(Uint8Array)()]) {
  try { console.log(f()); } catch (e) { console.log(String(e)); }
}
const o = { valueOf() { return 42; } }; u[0] = o; console.log(u[0]); try { u[0] = Symbol(); } catch (e) { console.log(String(e)); }
