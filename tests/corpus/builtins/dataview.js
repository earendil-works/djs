// DataView (0039): every getter and setter, both endiannesses, bounds and detaching.
function t(f) {
  try {
    console.log(JSON.stringify(f()));
  } catch (e) {
    console.log(e.name + ": " + e.message);
  }
}
const names = ["Int8", "Uint8", "Int16", "Uint16", "Int32", "Uint32", "Float16", "Float32", "Float64"];
const bytes = new Uint8Array([1, 0x80, 0xff, 3, 0x3c, 5, 0x7f, 0xfe, 0, 0x40]);
const v = new DataView(bytes.buffer, 1);
for (const n of names) {
  const out = [];
  for (let i = 0; i + 1 < v.byteLength; i += 3) out.push(v["get" + n](0), v["get" + n](i, true), v["get" + n](i, false));
  console.log(n, out.map(String).join(" "));
}
for (const n of names) {
  const w = new DataView(new ArrayBuffer(10));
  w["set" + n](1, -1234.75);
  w["set" + n](5, 65.5, true);
  console.log(n, Array.from(new Uint8Array(w.buffer)).join(","));
}
t(() => new DataView(new ArrayBuffer(4)).getInt32(1));
t(() => new DataView(new ArrayBuffer(4), 5));
t(() => new DataView(new ArrayBuffer(4), 1, 4));
t(() => DataView(new ArrayBuffer(1)));
t(() => new DataView({}));
t(() => DataView.prototype.getInt8.call(new Uint8Array(1), 0));
t(() => new DataView(new ArrayBuffer(4)).getInt8(-1));
t(() => {
  const b = new ArrayBuffer(4);
  const d = new DataView(b);
  b.transfer();
  return d.getInt8(0);
});
t(() => {
  const b = new ArrayBuffer(4, { maxByteLength: 8 });
  const d = new DataView(b, 2);
  const lens = [d.byteLength];
  b.resize(8);
  lens.push(d.byteLength);
  d.setUint16(4, 0xabcd);
  lens.push(new Uint8Array(b)[6], new Uint8Array(b)[7]);
  return lens;
});
t(() => [Object.prototype.toString.call(new DataView(new ArrayBuffer(1))), DataView.length, DataView.prototype.setFloat64.length]);
t(() => {
  const d = new DataView(new ArrayBuffer(8));
  d.setFloat64(0, NaN);
  const a = [d.getUint8(0) >= 0x7f];
  d.setFloat16(0, 65520);
  a.push(d.getFloat16(0), d.getUint16(0));
  d.setFloat16(0, 5.960464477539063e-8);
  a.push(d.getFloat16(0));
  d.setUint32(0, { valueOf: () => 4294967297 });
  a.push(d.getUint32(0), d.getInt32(0, true));
  return a;
});
