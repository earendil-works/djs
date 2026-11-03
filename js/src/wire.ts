// The inbound wire format encoder: host JS value -> bytes for djs' `wire::decode`.
// Format specification: docs/wire.md (version 1).

import { LAYOUT } from "./layout.generated.ts";

export const VERSION = 1;

export const Tag = {
  UNDEFINED: 0x00,
  NULL: 0x01,
  FALSE: 0x02,
  TRUE: 0x03,
  INT32: 0x04,
  FLOAT64: 0x05,
  HOLE: 0x06,
  INT8: 0x07,
  STR_UTF8: 0x10,
  STR_UTF16: 0x11,
  ARRAY: 0x20,
  OBJECT: 0x21,
  BYTES: 0x30,
  HOST_REF: 0x3f,
} as const;

/** Strings up to this length are tried as ASCII with a plain loop before TextEncoder. */
const SHORT_STRING = 64;
/** Cycles are looked for once nesting is this deep (a cycle eventually gets there). */
const CYCLE_CHECK_DEPTH = 32;

const textEncoder = new TextEncoder();

/** A value that cannot be encoded; `path` says where (`$.a[3]`). */
class EncodeError extends TypeError {
  segments: (string | number)[] = [];
  reason: string;
  constructor(reason: string) {
    super(reason);
    this.reason = reason;
  }
}

function describe(v: unknown): string {
  if (typeof v === "function") return "a function";
  if (typeof v === "symbol") return "a symbol";
  if (typeof v === "bigint") return "a BigInt";
  if (typeof v === "object" && v !== null) {
    const name = (v as object).constructor?.name;
    return name ? name : "an object with a custom prototype";
  }
  return typeof v;
}

function formatPath(segments: (string | number)[]): string {
  let p = "$";
  for (const s of segments) {
    if (typeof s === "number") p += `[${s}]`;
    else if (/^[A-Za-z_$][\w$]*$/.test(s)) p += `.${s}`;
    else p += `[${JSON.stringify(s)}]`;
  }
  return p;
}

class Encoder {
  buf: Uint8Array;
  view: DataView;
  pos = 0;
  keys = new Map<string, number>();
  stack: object[] = [];
  depth = 0;

  constructor(buffer?: Uint8Array) {
    this.buf = buffer ?? new Uint8Array(4096);
    this.view = new DataView(this.buf.buffer, this.buf.byteOffset, this.buf.byteLength);
  }

  ensure(n: number): void {
    if (this.pos + n <= this.buf.length) return;
    const size = Math.max(this.buf.length * 2, this.pos + n, 64);
    const next = new Uint8Array(size);
    next.set(this.buf.subarray(0, this.pos));
    this.buf = next;
    this.view = new DataView(next.buffer);
  }

  byte(b: number): void {
    this.ensure(1);
    this.buf[this.pos++] = b;
  }

  /** LEB128; the caller has ensured 5 bytes of room. */
  varint(v: number): void {
    const buf = this.buf;
    let pos = this.pos;
    while (v >= 0x80) {
      buf[pos++] = (v & 0x7f) | 0x80;
      v >>>= 7;
    }
    buf[pos++] = v;
    this.pos = pos;
  }

  value(v: unknown): void {
    switch (typeof v) {
      case "string":
        this.string(v);
        return;
      case "number":
        this.number(v);
        return;
      case "boolean":
        this.byte(v ? Tag.TRUE : Tag.FALSE);
        return;
      case "undefined":
        this.byte(Tag.UNDEFINED);
        return;
      case "object":
        if (v === null) {
          this.byte(Tag.NULL);
        } else if (Array.isArray(v)) {
          this.array(v);
        } else if (v instanceof Uint8Array) {
          this.bytes(v);
        } else {
          const proto = Object.getPrototypeOf(v);
          if (proto !== Object.prototype && proto !== null) {
            throw new EncodeError(`cannot encode ${describe(v)}`);
          }
          this.object(v as Record<string, unknown>);
        }
        return;
      default:
        throw new EncodeError(`cannot encode ${describe(v)}`);
    }
  }

  number(v: number): void {
    this.ensure(9);
    const i = v | 0;
    if (i === v && (i !== 0 || 1 / v > 0)) {
      if (i >= -128 && i <= 127) {
        this.buf[this.pos] = Tag.INT8;
        this.buf[this.pos + 1] = i & 0xff;
        this.pos += 2;
      } else {
        this.buf[this.pos] = Tag.INT32;
        this.view.setInt32(this.pos + 1, i, true);
        this.pos += 5;
      }
    } else {
      this.buf[this.pos] = Tag.FLOAT64;
      this.view.setFloat64(this.pos + 1, v, true);
      this.pos += 9;
    }
  }

  string(s: string): void {
    const n = s.length;
    // tag, units, the reserved byte length, and at most 3 bytes per unit.
    this.ensure(11 + 3 * n);
    const buf = this.buf;
    const start = this.pos;
    buf[start] = Tag.STR_UTF8;
    this.pos = start + 1;
    this.varint(n);
    // Reserve a byte length varint wide enough for the worst case; it is written padded.
    const lenAt = this.pos;
    const lenWidth = n < 43 ? 1 : n < 5462 ? 2 : n < 699051 ? 3 : n < 89478486 ? 4 : 5;
    const data = lenAt + lenWidth;
    let written = -1;
    if (n <= SHORT_STRING) {
      // By hand: cheaper than a TextEncoder call for short strings.
      let p = data;
      let i = 0;
      for (; i < n; i++) {
        let c = s.charCodeAt(i);
        if (c < 0x80) {
          buf[p++] = c;
        } else if (c < 0x800) {
          buf[p++] = 0xc0 | (c >> 6);
          buf[p++] = 0x80 | (c & 0x3f);
        } else if ((c & 0xf800) !== 0xd800) {
          buf[p++] = 0xe0 | (c >> 12);
          buf[p++] = 0x80 | ((c >> 6) & 0x3f);
          buf[p++] = 0x80 | (c & 0x3f);
        } else {
          const d = i + 1 < n ? s.charCodeAt(i + 1) : 0;
          if (c >= 0xdc00 || (d & 0xfc00) !== 0xdc00) break; // lone surrogate
          c = 0x10000 + ((c - 0xd800) << 10) + (d - 0xdc00);
          i++;
          buf[p++] = 0xf0 | (c >> 18);
          buf[p++] = 0x80 | ((c >> 12) & 0x3f);
          buf[p++] = 0x80 | ((c >> 6) & 0x3f);
          buf[p++] = 0x80 | (c & 0x3f);
        }
      }
      if (i === n) {
        written = p - data;
      } else {
        this.pos = start;
        this.stringUtf16(s);
        return;
      }
    }
    if (written < 0) {
      if (!s.isWellFormed()) {
        this.pos = start;
        this.stringUtf16(s);
        return;
      }
      written = textEncoder.encodeInto(s, buf.subarray(data)).written;
    }
    // The padded varint: lenWidth bytes, continuation bits on all but the last.
    let v = written;
    for (let k = 0; k < lenWidth - 1; k++) {
      buf[lenAt + k] = (v & 0x7f) | 0x80;
      v >>>= 7;
    }
    buf[lenAt + lenWidth - 1] = v;
    this.pos = data + written;
  }

  stringUtf16(s: string): void {
    const n = s.length;
    this.ensure(6 + 2 * n);
    this.buf[this.pos++] = Tag.STR_UTF16;
    this.varint(n);
    const view = this.view;
    let pos = this.pos;
    for (let i = 0; i < n; i++) {
      view.setUint16(pos, s.charCodeAt(i), true);
      pos += 2;
    }
    this.pos = pos;
  }

  enter(v: object): void {
    if (this.depth >= CYCLE_CHECK_DEPTH) {
      for (let i = 0; i < this.depth; i++) {
        if (this.stack[i] === v) throw new EncodeError("cannot encode a cycle");
      }
    }
    this.stack[this.depth++] = v;
  }

  array(v: unknown[]): void {
    this.enter(v);
    const n = v.length;
    this.ensure(6);
    this.buf[this.pos++] = Tag.ARRAY;
    this.varint(n);
    let i = 0;
    try {
      for (; i < n; i++) {
        const x = v[i];
        if (x === undefined && !(i in v)) {
          this.byte(Tag.HOLE);
        } else {
          this.value(x);
        }
      }
    } catch (e) {
      if (e instanceof EncodeError) e.segments.unshift(i);
      throw e;
    }
    this.depth--;
  }

  object(v: Record<string, unknown>): void {
    this.enter(v);
    const keys = Object.keys(v);
    const n = keys.length;
    this.ensure(6);
    this.buf[this.pos++] = Tag.OBJECT;
    this.varint(n);
    const table = this.keys;
    let key = "";
    try {
      for (let i = 0; i < n; i++) {
        key = keys[i];
        const k = table.get(key);
        this.ensure(5);
        if (k === undefined) {
          table.set(key, table.size + 1);
          this.buf[this.pos++] = 0;
          this.string(key);
        } else {
          this.varint(k);
        }
        this.value(v[key]);
      }
    } catch (e) {
      if (e instanceof EncodeError) e.segments.unshift(key);
      throw e;
    }
    this.depth--;
  }

  bytes(v: Uint8Array): void {
    this.ensure(6 + v.length);
    this.buf[this.pos++] = Tag.BYTES;
    this.varint(v.length);
    this.buf.set(v, this.pos);
    this.pos += v.length;
  }
}

/**
 * Encodes `value` for djs' `wire::decode`. Writes into `buffer` if given and big enough,
 * otherwise into a new, larger buffer; returns the bytes written (a view of that buffer).
 *
 * Accepts undefined, null, booleans, numbers, strings, arrays (holes included), Uint8Array
 * and plain objects (prototype Object.prototype or null; own enumerable string keys).
 * Anything else, and cycles, throw a TypeError naming the path. `toJSON` is not called.
 */
export function encode(value: unknown, buffer?: Uint8Array): Uint8Array {
  const e = new Encoder(buffer);
  e.ensure(4);
  e.buf.set([0x44, 0x4a, 0x57, VERSION], 0);
  e.pos = 4;
  try {
    e.value(value);
  } catch (err) {
    if (err instanceof EncodeError) {
      throw new TypeError(`${err.reason} at ${formatPath(err.segments)}`);
    }
    throw err;
  }
  return e.buf.subarray(0, e.pos);
}

/** Host strings from this length on go into the heap with `stringInPlace` (D2551). */
export const IN_PLACE_MIN = 1024;

/** The exports `stringInPlace` uses (djs-wasm; optional: older modules lack them). */
export interface InPlaceExports {
  memory: WebAssembly.Memory;
  djs_out_ptr(): number;
  vm_arena_ptr(vm: number): number;
  vm_handle_bits(vm: number, h: number): void;
  vm_string_reserve?(vm: number, len: number): number;
}

/** Where a one-byte string cell's code units start. */
const STR8_DATA = LAYOUT.cells.Str8.tail!.offset;

/** The varint at `buf[at]`: its value and the offset after it. */
function readVarint(buf: Uint8Array, at: number): [number, number] {
  let v = 0;
  for (let shift = 0; ; shift += 7) {
    const b = buf[at++];
    v += (b & 0x7f) * 2 ** shift;
    if (b < 0x80) return [v, at];
  }
}

/**
 * A handle to string `s` (`wire`, its `encode`d bytes) copied from the wire bytes straight
 * into a reserved heap string when it is ASCII (D2551): no staging copy and no transcode,
 * and the staging buffer does not grow to the string's size. 0 when it is not ASCII
 * (the UTF-8 is longer than `s`), the module lacks the exports, or the reservation
 * failed: then stage `wire` as usual. Deciding from the encoding costs non-ASCII strings
 * nothing; encoding straight into the reservation (one copy fewer for ASCII) made them
 * pay a reservation, a second encoding and a second arena growth (D2551).
 */
export function stringInPlace(x: InPlaceExports, vm: number, s: string, wire: Uint8Array): number {
  if (!x.vm_string_reserve || wire[4] !== Tag.STR_UTF8) return 0;
  const n = s.length;
  const [units, lenAt] = readVarint(wire, 5);
  const [bytes, data] = readVarint(wire, lenAt);
  if (units !== n || bytes !== n) return 0;
  const h = x.vm_string_reserve(vm, n);
  if (h === 0) return 0;
  // The string's offset is its value's low word; views after the calls (memory may grow).
  x.vm_handle_bits(vm, h);
  const cell = new DataView(x.memory.buffer, x.djs_out_ptr(), 4).getUint32(0, true);
  const at = x.vm_arena_ptr(vm) + cell + STR8_DATA;
  new Uint8Array(x.memory.buffer, at, n).set(wire.subarray(data, data + n));
  return h;
}
