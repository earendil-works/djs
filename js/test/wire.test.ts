import { test } from "node:test";
import assert from "node:assert/strict";
import { encode, Tag, VERSION } from "../src/wire.ts";

/** A reference decoder (docs/wire.md), for round-trip tests. */
function decode(bytes: Uint8Array): unknown {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let pos = 0;
  const keys: string[] = [];
  const varint = (): number => {
    let v = 0;
    for (let i = 0; i < 5; i++) {
      const b = bytes[pos++];
      v += (b & 0x7f) * 2 ** (7 * i);
      if (b < 0x80) return v;
    }
    throw new Error("varint too long");
  };
  const str = (tag: number): string => {
    const units = varint();
    if (tag === Tag.STR_UTF16) {
      let s = "";
      for (let i = 0; i < units; i++) s += String.fromCharCode(view.getUint16(pos + 2 * i, true));
      pos += 2 * units;
      return s;
    }
    assert.equal(tag, Tag.STR_UTF8);
    const n = varint();
    const s = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes.subarray(pos, pos + n));
    pos += n;
    assert.equal(s.length, units);
    return s;
  };
  const value = (): unknown => {
    const t = bytes[pos++];
    switch (t) {
      case Tag.UNDEFINED: return undefined;
      case Tag.NULL: return null;
      case Tag.FALSE: return false;
      case Tag.TRUE: return true;
      case Tag.INT8: return view.getInt8(pos++);
      case Tag.INT32: pos += 4; return view.getInt32(pos - 4, true);
      case Tag.FLOAT64: pos += 8; return view.getFloat64(pos - 8, true);
      case Tag.STR_UTF8: case Tag.STR_UTF16: return str(t);
      case Tag.ARRAY: {
        const n = varint();
        const a: unknown[] = new Array(n);
        for (let i = 0; i < n; i++) {
          if (bytes[pos] === Tag.HOLE) { pos++; continue; }
          a[i] = value();
        }
        return a;
      }
      case Tag.OBJECT: {
        const n = varint();
        const o: Record<string, unknown> = {};
        for (let i = 0; i < n; i++) {
          const k = varint();
          let key: string;
          if (k === 0) { key = str(bytes[pos++]); keys.push(key); } else key = keys[k - 1];
          o[key] = value();
        }
        return o;
      }
      case Tag.BYTES: {
        const n = varint();
        pos += n;
        return bytes.slice(pos - n, pos);
      }
      default: throw new Error(`bad tag ${t} at ${pos - 1}`);
    }
  };
  assert.deepEqual([...bytes.subarray(0, 4)], [0x44, 0x4a, 0x57, VERSION]);
  pos = 4;
  const v = value();
  assert.equal(pos, bytes.length, "trailing bytes");
  return v;
}

const roundTrip = (v: unknown) => assert.deepStrictEqual(decode(encode(v)), v);

test("primitives", () => {
  for (const v of [undefined, null, true, false, 0, -0, 1, -1, 127, -128, 128, -129, 2 ** 31 - 1,
    -(2 ** 31), 2 ** 31, 1.5, NaN, Infinity, -Infinity, Number.MAX_VALUE, Number.MIN_VALUE]) {
    roundTrip(v);
  }
  assert.deepEqual([...encode(-0)], [0x44, 0x4a, 0x57, 1, Tag.FLOAT64, 0, 0, 0, 0, 0, 0, 0, 0x80]);
  assert.deepEqual([...encode(5)], [0x44, 0x4a, 0x57, 1, Tag.INT8, 5]);
  assert.deepEqual([...encode(-200)].slice(4), [Tag.INT32, 0x38, 0xff, 0xff, 0xff]);
});

test("strings", () => {
  for (const s of ["", "a", "hello", "é", "日本語", "😀", "a😀b", "x".repeat(42), "x".repeat(43),
    "é".repeat(100), "\u0000", "\uFEFF", "a".repeat(70000), "ü".repeat(6000)]) {
    roundTrip(s);
  }
  // ASCII: units == bytes, padded length varint for the reserved width.
  assert.deepEqual([...encode("ab")].slice(4), [Tag.STR_UTF8, 2, 2, 0x61, 0x62]);
  const long = [...encode("a".repeat(50))].slice(4, 8);
  assert.deepEqual(long, [Tag.STR_UTF8, 50, 50 | 0x80, 0]);
  // Lone surrogates take the UTF-16 form.
  roundTrip("\uD800");
  roundTrip("a\uDC00b");
  assert.deepEqual([...encode("\uD800")].slice(4), [Tag.STR_UTF16, 1, 0x00, 0xd8]);
});

test("containers and the key table", () => {
  roundTrip([]);
  roundTrip({});
  roundTrip([1, "two", [3, [4]], { five: 5 }]);
  roundTrip({ a: 1, b: { a: 2, c: [{ a: 3 }] }, "": 0, "0": "zero", "😀": "emoji" });
  const rows = Array.from({ length: 100 }, (_, i) => ({ id: i, name: `n${i}`, tags: ["x"] }));
  roundTrip(rows);
  // Each key is sent once: the second object references the table.
  const bytes = encode([{ k: 1 }, { k: 2 }]);
  assert.deepEqual([...bytes].slice(4), [Tag.ARRAY, 2, Tag.OBJECT, 1, 0, Tag.STR_UTF8, 1, 1, 0x6b,
    Tag.INT8, 1, Tag.OBJECT, 1, 1, Tag.INT8, 2]);
  assert.deepStrictEqual(decode(encode(Object.assign(Object.create(null), { x: 1 }))), { x: 1 });
});

test("holes and bytes", () => {
  // eslint-disable-next-line no-sparse-arrays
  const sparse = [1, , 3];
  const out = decode(encode(sparse)) as unknown[];
  assert.equal(out.length, 3);
  assert.ok(!(1 in out));
  roundTrip(new Uint8Array([0, 1, 255]));
  const buf = Buffer.from("hi");
  assert.deepEqual([...(decode(encode(buf)) as Uint8Array)], [0x68, 0x69]);
});

test("unsupported values name the path", () => {
  assert.throws(() => encode({ a: [1, { when: new Date(0) }] }), {
    name: "TypeError",
    message: "cannot encode Date at $.a[1].when",
  });
  assert.throws(() => encode([() => 1]), { message: "cannot encode a function at $[0]" });
  assert.throws(() => encode({ "a b": Symbol() }), { message: 'cannot encode a symbol at $["a b"]' });
  assert.throws(() => encode(1n), { message: "cannot encode a BigInt at $" });
  assert.throws(() => encode(new Map()), { message: "cannot encode Map at $" });
  assert.throws(() => encode({ x: new Uint16Array(1) }), { message: "cannot encode Uint16Array at $.x" });
  class Point { x = 1; }
  assert.throws(() => encode(new Point()), { message: "cannot encode Point at $" });
});

test("toJSON is not called", () => {
  roundTrip({ toJSON: 1 });
  const o = { a: 1, toJSON() { return "no"; } };
  assert.throws(() => encode(o), { message: "cannot encode a function at $.toJSON" });
});

test("cycles", () => {
  const a: unknown[] = [];
  a.push(a);
  assert.throws(() => encode(a), (e: Error) => e instanceof TypeError && e.message.startsWith("cannot encode a cycle at $[0][0]"));
  const o: Record<string, unknown> = { x: {} };
  (o.x as Record<string, unknown>).y = o;
  assert.throws(() => encode(o), /cannot encode a cycle at \$\.x\.y/);
  // Shared but acyclic references are fine (and copied).
  const shared = { s: 1 };
  roundTrip({ a: shared, b: shared, c: [shared, shared] });
  // Deep but acyclic.
  let deep: unknown = 0;
  for (let i = 0; i < 1000; i++) deep = [deep];
  roundTrip(deep);
});

test("buffers grow and can be reused", () => {
  const small = new Uint8Array(8);
  const out = encode("x".repeat(1000), small);
  assert.notEqual(out.buffer, small.buffer);
  const big = new Uint8Array(1 << 16);
  const out2 = encode({ a: [1, 2, 3] }, big);
  assert.equal(out2.buffer, big.buffer);
  roundTrip({ a: "y".repeat(100000), b: Array.from({ length: 5000 }, (_, i) => i * 1.5) });
  // An offset view works too.
  const region = new Uint8Array(new ArrayBuffer(4096), 100, 1000);
  assert.deepStrictEqual(decode(encode([1.25, "z"], region)), [1.25, "z"]);
});

test("random values round-trip", () => {
  let seed = 12345;
  const rnd = (n: number) => {
    seed = (seed * 1103515245 + 12345) >>> 0;
    return seed % n;
  };
  const strs = ["", "a", "key", "é", "😀", "\uD83D", "日本", "x".repeat(80)];
  const gen = (d: number): unknown => {
    switch (rnd(d > 3 ? 6 : 9)) {
      case 0: return rnd(2) === 0 ? null : undefined;
      case 1: return rnd(2) === 0;
      case 2: return [0, -0, 1.5, 300, -5, 2 ** 40, NaN][rnd(7)];
      case 3: case 4: return strs[rnd(strs.length)] + strs[rnd(strs.length)];
      case 5: return rnd(1000) - 500;
      case 6: return Array.from({ length: rnd(5) }, () => gen(d + 1));
      case 7: {
        const o: Record<string, unknown> = {};
        for (let i = rnd(5); i > 0; i--) o[strs[rnd(strs.length)] + rnd(3)] = gen(d + 1);
        return o;
      }
      default: return new Uint8Array([rnd(256), rnd(256)]);
    }
  };
  for (let i = 0; i < 2000; i++) roundTrip(gen(0));
});
