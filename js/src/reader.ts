// The outbound direct reader: decodes VM values straight out of the heap arena
// (VM -> host, no serialization in the VM). Everything it knows about the heap comes from
// layout.generated.ts, which djs-core exports from its layout table (D1052).
// Format notes: docs/wire.md, "Outbound: direct heap reads".

import { LAYOUT } from "./layout.generated.ts";

const cells = LAYOUT.cells;
const tags = LAYOUT.value.tags;

// Offsets and constants, looked up once from the layout description.
const ROOTS_MAGIC = cells.Roots.fields.MAGIC.offset;
const ROOTS_VERSION = cells.Roots.fields.VERSION.offset;
const ROOTS_ATOMS = cells.Roots.fields.ATOMS.offset;
const HDR_KIND = LAYOUT.header.KIND;
const HDR_FLAGS = LAYOUT.header.FLAGS;
const K_STR8 = cells.Str8.kind;
const K_STR16 = cells.Str16.kind;
const K_ROPE = cells.Rope.kind;
const STR8_LEN = cells.Str8.fields.LEN.offset;
const STR8_DATA = cells.Str8.tail.offset;
const STR16_LEN = cells.Str16.fields.LEN.offset;
const STR16_DATA = cells.Str16.tail.offset;
const ROPE_LEFT = cells.Rope.fields.LEFT.offset;
const ROPE_RIGHT = cells.Rope.fields.RIGHT.offset;
const ROPE_FLATTENED = LAYOUT.flags.ROPE_FLATTENED;
const ROPE_APPEND = LAYOUT.flags.ROPE_APPEND;
const ROPE_LEN = cells.Rope.fields.LEN.offset;
const SYM_DESCRIPTION = cells.Symbol.fields.DESCRIPTION.offset;
const BIG_LEN = cells.BigInt.fields.LEN.offset;
const BIG_LIMBS = cells.BigInt.tail.offset;
const BIGINT_NEG = LAYOUT.flags.BIGINT_NEG;
const OBJ_CLASS = cells.Object.fields.CLASS.offset;
const OBJ_PROPS = cells.Object.fields.PROPS.offset;
const OBJ_ELEMENTS = cells.Object.fields.ELEMENTS.offset;
const OBJ_SLOTS = cells.Object.tail.offset;
const OBJ_INDEXED_PROPS = LAYOUT.flags.OBJ_INDEXED_PROPS;
const TAB_COUNT = cells.PropTable.fields.COUNT.offset;
const TAB_ENTRIES = cells.PropTable.tail.offset;
const ENTRY_SIZE = cells.PropTable.tail.elemSize;
const ENTRY_KEY = cells.PropTable.tail.fields.KEY.offset;
const ENTRY_FLAGS = cells.PropTable.tail.fields.FLAGS.offset;
const ENTRY_VALUE = cells.PropTable.tail.fields.VALUE.offset;
const OBJ_DICTIONARY = LAYOUT.flags.OBJ_DICTIONARY;
const PV_SHAPE = cells.PropValues.fields.SHAPE.offset;
const PV_LEN = cells.PropValues.fields.LEN.offset;
const PV_VALUES = cells.PropValues.tail.offset;
const SHAPE_DESCS = cells.Shape.fields.DESCS.offset;
const U32_DATA = cells.U32Array.tail.offset;
const VA_LEN = cells.ValueArray.fields.LEN.offset;
const VA_VALUES = cells.ValueArray.tail.offset;
const ATOMS_ENTRIES = cells.AtomTable.fields.ENTRIES.offset;
const P_ENUMERABLE = LAYOUT.propFlags.ENUMERABLE;
const P_ACCESSOR = LAYOUT.propFlags.ACCESSOR;
const A_INT = LAYOUT.atom.intBit;
const A_SYMBOL = LAYOUT.atom.symbolBit;
const A_INDEX = LAYOUT.atom.indexMask;
const C_ORDINARY = LAYOUT.objectClasses.Ordinary;
const C_ARRAY = LAYOUT.objectClasses.Array;
const C_ARRAY_BUFFER = LAYOUT.objectClasses.ArrayBuffer;
const C_TYPED_ARRAY = LAYOUT.objectClasses.TypedArray;
const BYTES_LEN = cells.Bytes.fields.LEN.offset;
const BYTES_DATA = cells.Bytes.tail.offset;
// Typed array kinds (heap/typed.rs `Kind`) as host constructors. Float16Array needs a
// recent engine; without it the 16-bit patterns are read as a Uint16Array.
type TypedArrayCtor = new (buffer: ArrayBuffer) => ArrayBufferView;
const F16 = (globalThis as { Float16Array?: TypedArrayCtor }).Float16Array ?? Uint16Array;
const TA_CTORS: [TypedArrayCtor, number][] = [
  [Int8Array, 1], [Uint8Array, 1], [Uint8ClampedArray, 1], [Int16Array, 2], [Uint16Array, 2],
  [Int32Array, 4], [Uint32Array, 4], [F16, 2], [Float32Array, 4], [Float64Array, 8],
  [BigInt64Array, 8], [BigUint64Array, 8],
];
const TAG_PREFIX = LAYOUT.value.tagPrefix;
const HI_HOLE = ((TAG_PREFIX << 16) | tags.HOLE) >>> 0;
const HI_INT = ((TAG_PREFIX << 16) | tags.INT) >>> 0;
const HI_UNDEFINED = ((TAG_PREFIX << 16) | tags.UNDEFINED) >>> 0;

const tagNames: string[] = [];
for (const [name, t] of Object.entries(tags)) tagNames[t] = name;
const classNames: string[] = [];
for (const [name, c] of Object.entries(LAYOUT.objectClasses)) classNames[c] = name;

/** Strings up to this length are built character by character. */
const TINY = 4;
/** Without Buffer, strings shorter than this use String.fromCharCode, not a TextDecoder. */
const SHORT = 48;
/** String.fromCharCode.apply argument chunk. */
const CHUNK = 0x2000;

// Node's Buffer decodes both representations exactly at memcpy speed: `latin1Slice` is
// ISO-8859-1 (unlike TextDecoder's "latin1") and `ucs2Slice` keeps lone surrogates.
interface NodeBuffer {
  latin1Slice(start: number, end: number): string;
  ucs2Slice(start: number, end: number): string;
}
interface NodeBufferCtor {
  from(buffer: ArrayBufferLike, byteOffset: number, length: number): NodeBuffer;
}
const NodeBuffer = (globalThis as { Buffer?: NodeBufferCtor }).Buffer;
const hasNodeBuffer =
  NodeBuffer !== undefined &&
  typeof (NodeBuffer.from(new ArrayBuffer(2), 0, 2) as Partial<NodeBuffer>).latin1Slice === "function";

const utf16Decoder = new TextDecoder("utf-16le", { fatal: true, ignoreBOM: true });
// "latin1" is windows-1252 in the Encoding standard. It agrees with Latin-1 everywhere but
// 0x80..0x9F, so it is only used for data without bytes in that range.
const win1252Decoder = new TextDecoder("windows-1252");

/**
 * A VM value with no host equivalent: a symbol, a function or other non-plain object, a
 * host ref or an internal value (BigInts are read as native bigints). `ref` is the arena offset of its cell (0 if none).
 */
export class Opaque {
  readonly type: string;
  readonly ref: number;
  /** The object class (`Function`, `Error`, ...) for objects. */
  readonly className?: string;
  /** A symbol's description. */
  readonly description?: string;
  constructor(type: string, ref: number, className?: string, description?: string) {
    this.type = type;
    this.ref = ref;
    this.className = className;
    this.description = description;
  }
}

export interface ReadOptions {
  /** Use only standard APIs (TextDecoder, String.fromCharCode), not node's Buffer. */
  portable?: boolean;
  /**
   * The most values (containers and elements) to produce. Shared references are read
   * once per occurrence, so a heap with sharing at every level could otherwise expand
   * exponentially. Default: four times the arena's value slots, plus a million.
   */
  maxValues?: number;
  /**
   * Atom index -> key string, filled while reading. Reusing one array across reads of the
   * same VM is safe: atoms are never freed and their strings never change.
   */
  atomCache?: (string | undefined)[];
}

/** A value that cannot be read as a tree: `path` says where (`$.a[3]`). */
class ReadError extends TypeError {
  segments: (string | number)[] = [];
  reason: string;
  constructor(reason: string) {
    super(reason);
    this.reason = reason;
  }
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

/** Cycles are looked for once nesting is this deep (a cycle eventually gets there). */
const CYCLE_CHECK_DEPTH = 32;

/** Thrown when the memory does not hold a heap of this build's layout. */
export class LayoutMismatch extends Error {}

/**
 * Checks that `memory` holds a djs heap with the layout this reader was generated for
 * (the magic and `LAYOUT_HASH` in the roots cell).
 */
export function checkHeap(memory: Uint8Array): void {
  if (memory.length < cells.Roots.minSize) throw new LayoutMismatch("not a djs heap: too small");
  const view = new DataView(memory.buffer, memory.byteOffset, memory.byteLength);
  if (view.getUint32(ROOTS_MAGIC, true) !== LAYOUT.magic) {
    throw new LayoutMismatch("not a djs heap: bad magic");
  }
  const version = view.getUint32(ROOTS_VERSION, true);
  if (version !== LAYOUT.layoutHash) {
    throw new LayoutMismatch(
      `djs heap layout ${version.toString(16)} does not match the reader's ${LAYOUT.layoutHash.toString(16)}`,
    );
  }
}

/**
 * Decodes the VM value `value` (its 64 bits, or `[lo, hi]` as two u32) from the heap arena
 * `memory` (offset 0 of the view is the arena start, i.e. the roots cell).
 *
 * Primitives map to themselves; strings (flat or ropes) to strings; arrays to arrays (holes
 * stay holes); ordinary objects to plain objects with their own enumerable string-keyed data
 * properties in property order. Accessors, non-enumerable and symbol-keyed properties are
 * left out. Everything else becomes an {@link Opaque}. The result is a tree, as with JSON:
 * a shared object is read once per reference, and a cycle is a TypeError naming its path.
 */
export function readValue(
  memory: Uint8Array,
  value: bigint | readonly [number, number],
  opts: ReadOptions = {},
): unknown {
  checkHeap(memory);
  let lo: number, hi: number;
  if (typeof value === "bigint") {
    lo = Number(value & 0xffffffffn);
    hi = Number((value >> 32n) & 0xffffffffn);
  } else {
    [lo, hi] = value;
  }
  const budget = opts.maxValues ?? 4 * (memory.length >>> 3) + 1_000_000;
  const r = new Reader(memory, opts.atomCache ?? [], opts.portable === true, budget);
  if (hi >>> 16 !== TAG_PREFIX) {
    const scratch = new DataView(new ArrayBuffer(8));
    scratch.setUint32(0, lo, true);
    scratch.setUint32(4, hi, true);
    return scratch.getFloat64(0, true);
  }
  try {
    return r.tagged(hi & 0xffff, lo >>> 0);
  } catch (e) {
    if (e instanceof ReadError) throw new TypeError(`${e.reason} at ${formatPath(e.segments)}`);
    throw e;
  }
}

class Reader {
  mem: Uint8Array;
  view: DataView;
  u16: Uint16Array | null;
  /** u32 and f64 views over the arena when its base is 8-aligned (for element loops). */
  w32: Uint32Array | null;
  f64: Float64Array | null;
  /** A Buffer over the arena, when running on node. */
  buf: NodeBuffer | null;
  atoms: (string | undefined)[];
  atomEntries: number;
  /** Offsets of the containers being read (cycle detection). */
  stack: number[] = [];
  depth = 0;
  /** Values left in the budget. */
  budget: number;

  constructor(mem: Uint8Array, atoms: (string | undefined)[], portable: boolean, budget: number) {
    this.budget = budget;
    this.mem = mem;
    this.view = new DataView(mem.buffer, mem.byteOffset, mem.byteLength);
    // Two-byte string data is 2-aligned within the arena; a view needs the base aligned too.
    this.u16 = mem.byteOffset % 2 === 0 ? new Uint16Array(mem.buffer, mem.byteOffset, mem.length >> 1) : null;
    const aligned = mem.byteOffset % 8 === 0;
    this.w32 = aligned ? new Uint32Array(mem.buffer, mem.byteOffset, mem.length >>> 2) : null;
    this.f64 = aligned ? new Float64Array(mem.buffer, mem.byteOffset, mem.length >>> 3) : null;
    this.buf = hasNodeBuffer && !portable ? NodeBuffer!.from(mem.buffer, mem.byteOffset, mem.byteLength) : null;
    this.atoms = atoms;
    const table = this.view.getUint32(ROOTS_ATOMS, true);
    this.atomEntries = table === 0 ? 0 : this.view.getUint32(table + ATOMS_ENTRIES, true);
  }

  /** The value stored at arena offset `off`. */
  slot(off: number): unknown {
    const hi = this.view.getUint32(off + 4, true);
    if (hi >>> 16 !== TAG_PREFIX) return this.view.getFloat64(off, true);
    return this.tagged(hi & 0xffff, this.view.getUint32(off, true));
  }

  tagged(tag: number, payload: number): unknown {
    switch (tag) {
      case tags.UNDEFINED:
      case tags.HOLE:
        return undefined;
      case tags.NULL:
        return null;
      case tags.BOOL:
        return payload !== 0;
      case tags.INT:
        return payload | 0;
      case tags.STRING:
        return this.string(payload);
      case tags.OBJECT:
        return this.object(payload);
      case tags.BIGINT:
        return this.bigint(payload);
      case tags.SYMBOL: {
        const d = this.slotIfString(payload + SYM_DESCRIPTION);
        return new Opaque("symbol", payload, undefined, d);
      }
      default: {
        const name = tagNames[tag];
        if (name === undefined || tag < LAYOUT.value.firstRef) {
          throw new Error(`djs heap: unexpected value tag ${tag}`);
        }
        return new Opaque(name.toLowerCase(), payload);
      }
    }
  }

  /** A BigInt cell as a native bigint (0046: u32 limbs, little-endian, sign flag). */
  bigint(off: number): bigint {
    const len = this.view.getUint32(off + BIG_LEN, true);
    let v = 0n;
    for (let i = len - 1; i >= 0; i--) {
      v = (v << 32n) | BigInt(this.view.getUint32(off + BIG_LIMBS + 4 * i, true));
    }
    return this.view.getUint16(off + HDR_FLAGS, true) & BIGINT_NEG ? -v : v;
  }

  slotIfString(off: number): string | undefined {
    const hi = this.view.getUint32(off + 4, true);
    if (hi !== ((TAG_PREFIX << 16) | tags.STRING) >>> 0) return undefined;
    return this.string(this.view.getUint32(off, true));
  }

  string(off: number): string {
    const kind = this.mem[off + HDR_KIND];
    if (kind === K_STR8) return this.latin1(off + STR8_DATA, this.view.getUint32(off + STR8_LEN, true));
    if (kind === K_STR16) return this.utf16(off + STR16_DATA, this.view.getUint32(off + STR16_LEN, true));
    if (kind === K_ROPE) {
      if (this.view.getUint16(off + HDR_FLAGS, true) & ROPE_APPEND) return this.appendRope(off);
      return this.rope(off);
    }
    throw new Error(`djs heap: cell ${off} is not a string`);
  }

  /** Walks a rope's leaves left to right (iteratively; ropes can be deep). */
  rope(off: number): string {
    const parts: string[] = [];
    const stack = [off];
    while (stack.length > 0) {
      const c = stack.pop()!;
      if (this.mem[c + HDR_KIND] !== K_ROPE) {
        parts.push(this.string(c));
      } else if (this.view.getUint16(c + HDR_FLAGS, true) & ROPE_APPEND) {
        parts.push(this.appendRope(c));
      } else if (this.view.getUint16(c + HDR_FLAGS, true) & ROPE_FLATTENED) {
        stack.push(this.view.getUint32(c + ROPE_LEFT, true));
      } else {
        stack.push(this.view.getUint32(c + ROPE_RIGHT, true), this.view.getUint32(c + ROPE_LEFT, true));
      }
    }
    return parts.join("");
  }

  /** An append rope (D1610): the first LEN code units of its buffer (LEFT). */
  appendRope(off: number): string {
    const buf = this.view.getUint32(off + ROPE_LEFT, true);
    const n = this.view.getUint32(off + ROPE_LEN, true);
    if (this.mem[buf + HDR_KIND] === K_STR8) return this.latin1(buf + STR8_DATA, n);
    return this.utf16(buf + STR16_DATA, n);
  }

  latin1(at: number, n: number): string {
    const mem = this.mem;
    if (n <= TINY) {
      let s = "";
      for (let i = 0; i < n; i++) s += String.fromCharCode(mem[at + i]);
      return s;
    }
    if (this.buf !== null) return this.buf.latin1Slice(at, at + n);
    if (n < SHORT) return fromCodes(mem, at, n);
    const bytes = mem.subarray(at, at + n);
    // Without bytes in 0x80..0x9F, windows-1252 is Latin-1.
    let c1 = false;
    for (let i = 0; i < n; i++) {
      if ((bytes[i] & 0xe0) === 0x80) {
        c1 = true;
        break;
      }
    }
    if (!c1) return win1252Decoder.decode(bytes);
    return chunked(bytes);
  }

  utf16(at: number, n: number): string {
    if (this.buf !== null && n > TINY) return this.buf.ucs2Slice(at, at + 2 * n);
    const u16 = this.u16;
    if (u16 === null) {
      const copy = new Uint16Array(n);
      for (let i = 0; i < n; i++) copy[i] = this.view.getUint16(at + 2 * i, true);
      return chunked(copy);
    }
    if (n < SHORT) return fromCodes(u16, at >> 1, n);
    const units = u16.subarray(at >> 1, (at >> 1) + n);
    try {
      return utf16Decoder.decode(units);
    } catch {
      // Lone surrogates: the fatal decoder refuses them, fromCharCode keeps them.
      return chunked(units);
    }
  }

  /** A property key, `undefined` for symbols. */
  key(atom: number): string | number | undefined {
    if (atom & A_INT) return (atom & ~A_INT) >>> 0;
    if (atom & A_SYMBOL) return undefined;
    const index = atom & A_INDEX;
    let s = this.atoms[index];
    if (s === undefined) {
      const slot = this.atomEntries + VA_VALUES + 8 * index;
      s = this.string(this.view.getUint32(slot, true));
      this.atoms[index] = s;
    }
    return s;
  }

  object(off: number): unknown {
    const cls = this.view.getUint32(off + OBJ_CLASS, true);
    if (cls === C_TYPED_ARRAY) return this.typedArray(off);
    if (cls === C_ARRAY_BUFFER) return this.arrayBuffer(off);
    if (cls !== C_ARRAY && cls !== C_ORDINARY) {
      return new Opaque("object", off, classNames[cls] ?? `class ${cls}`);
    }
    // A cycle reaches any depth, so only deep nesting needs the (linear) check.
    const depth = this.depth;
    if (depth >= CYCLE_CHECK_DEPTH) {
      const stack = this.stack;
      for (let i = 0; i < depth; i++) {
        if (stack[i] === off) throw new ReadError("cyclic value");
      }
    }
    this.stack[depth] = off;
    this.depth = depth + 1;
    const v = cls === C_ARRAY ? this.array(off) : this.plain(off);
    this.depth = depth;
    return v;
  }

  /**
   * The bytes of an ArrayBuffer object: the arena offset of its Bytes cell and its
   * length, or null once detached.
   */
  bufferBytes(buffer: number): [number, number] | null {
    const slot = buffer + OBJ_SLOTS;
    if (this.view.getUint32(slot + 4, true) === HI_UNDEFINED) return null;
    const cell = this.view.getUint32(slot, true);
    return [cell + BYTES_DATA, this.view.getUint32(cell + BYTES_LEN, true)];
  }

  /** An ArrayBuffer: a copy of its bytes (docs/wire.md, "Typed arrays"). */
  arrayBuffer(off: number): ArrayBuffer {
    const bytes = this.bufferBytes(off);
    if (bytes === null) return new ArrayBuffer(0);
    this.spend(1);
    return this.mem.slice(bytes[0], bytes[0] + bytes[1]).buffer;
  }

  /**
   * A typed array: a copy of its elements as the host's typed array of the same kind
   * (detached or out of bounds: empty). Copies, not views: the arena moves with every
   * collection and may be replaced when wasm memory grows.
   */
  typedArray(off: number): ArrayBufferView {
    const slots = off + OBJ_SLOTS;
    const kind = this.slot(slots + 8) as number;
    const [Ctor, size] = TA_CTORS[kind] ?? TA_CTORS[1];
    const byteOffset = this.slot(slots + 16) as number;
    const length = this.slot(slots + 24) as number;
    const bytes = this.bufferBytes(this.view.getUint32(slots, true));
    let n = 0;
    if (bytes !== null && byteOffset <= bytes[1]) {
      n = length >= 0 ? length : Math.floor((bytes[1] - byteOffset) / size);
      if (byteOffset + n * size > bytes[1]) n = 0;
    }
    this.spend(1);
    const start = bytes === null ? 0 : bytes[0] + byteOffset;
    return new Ctor(this.mem.slice(start, start + n * size).buffer);
  }

  spend(n: number): void {
    this.budget -= n;
    if (this.budget < 0) throw new RangeError("djs heap read: value too large (maxValues)");
  }

  plain(off: number): Record<string, unknown> {
    const o: Record<string, unknown> = {};
    let key: string | number | undefined = undefined;
    try {
      const view = this.view;
      const el = view.getUint32(off + OBJ_ELEMENTS, true);
      if (el !== 0) {
        const n = view.getUint32(el + VA_LEN, true);
        this.spend(n);
        for (let i = 0; i < n; i++) {
          const at = el + VA_VALUES + 8 * i;
          key = i;
          if (view.getUint32(at + 4, true) !== HI_HOLE) o[i] = this.slot(at);
        }
      }
      const t = view.getUint32(off + OBJ_PROPS, true);
      if (t === 0) return o;
      // Dictionary mode: a table of (key, flags, value) entries. Shape mode: a values
      // block whose shape's descriptors hold (key, flags) pairs (docs/objects.md 7).
      const dict = (view.getUint16(off + HDR_FLAGS, true) & OBJ_DICTIONARY) !== 0;
      const count = view.getUint32(t + (dict ? TAB_COUNT : PV_LEN), true);
      const descs = dict ? 0 : view.getUint32(view.getUint32(t + PV_SHAPE, true) + SHAPE_DESCS, true) + U32_DATA + 4;
      this.spend(count + 1);
      for (let i = 0; i < count; i++) {
        const e = dict ? t + TAB_ENTRIES + ENTRY_SIZE * i : descs + 8 * i;
        const atom = view.getUint32(e + (dict ? ENTRY_KEY : 0), true);
        if (atom === 0) continue; // deleted
        const flags = view.getUint32(e + (dict ? ENTRY_FLAGS : 4), true);
        if (flags & P_ACCESSOR || !(flags & P_ENUMERABLE)) continue;
        key = this.key(atom);
        if (key === undefined) continue;
        const v = this.slot(dict ? e + ENTRY_VALUE : t + PV_VALUES + 8 * i);
        // Assigning "__proto__" would set the prototype instead of defining a property.
        if (key === "__proto__") {
          Object.defineProperty(o, key, { value: v, writable: true, enumerable: true, configurable: true });
        } else {
          o[key] = v;
        }
      }
    } catch (e) {
      if (e instanceof ReadError && key !== undefined) e.segments.unshift(key);
      throw e;
    }
    return o;
  }

  array(off: number): unknown[] {
    const a: unknown[] = [];
    let i = 0;
    try {
      const view = this.view;
      const length = this.slot(off + OBJ_SLOTS) as number;
      const el = view.getUint32(off + OBJ_ELEMENTS, true);
      if (el !== 0) {
        const n = view.getUint32(el + VA_LEN, true);
        this.spend(n + 1);
        const w32 = this.w32;
        const f64 = this.f64;
        for (; i < n; i++) {
          const at = el + VA_VALUES + 8 * i;
          if (w32 !== null) {
            // Numbers (the common element) straight from typed views.
            const hi = w32[(at >> 2) + 1];
            if (hi >>> 16 !== TAG_PREFIX) {
              a.push(f64![at >> 3]);
              continue;
            }
            if (hi === HI_INT) {
              a.push(w32[at >> 2] | 0);
              continue;
            }
          }
          if (view.getUint32(at + 4, true) === HI_HOLE) {
            a.length = i + 1; // keep the hole
          } else {
            a.push(this.slot(at));
          }
        }
      }
      const hf = view.getUint16(off + HDR_FLAGS, true);
      if (hf & OBJ_INDEXED_PROPS && hf & OBJ_DICTIONARY) {
        // Elements beyond the dense part live in the property table (index keys always
        // make an object a dictionary).
        const t = view.getUint32(off + OBJ_PROPS, true);
        const count = t === 0 ? 0 : view.getUint32(t + TAB_COUNT, true);
        for (let k = 0; k < count; k++) {
          const e = t + TAB_ENTRIES + ENTRY_SIZE * k;
          const atom = view.getUint32(e + ENTRY_KEY, true);
          const flags = view.getUint32(e + ENTRY_FLAGS, true);
          if (atom === 0 || flags & P_ACCESSOR || !(flags & P_ENUMERABLE)) continue;
          const key = this.key(atom);
          if (typeof key === "number" || (typeof key === "string" && isArrayIndex(key))) {
            i = Number(key);
            this.spend(1);
            a[i] = this.slot(e + ENTRY_VALUE);
          }
        }
      }
      if (a.length < length) a.length = length;
    } catch (e) {
      if (e instanceof ReadError) e.segments.unshift(i);
      throw e;
    }
    return a;
  }
}

/** Array indexes that are string atoms (2^31 .. 2^32 - 2). */
function isArrayIndex(s: string): boolean {
  const n = Number(s);
  return n >= 0 && n < 4294967295 && Number.isInteger(n) && String(n) === s;
}

const codes: number[] = [];

/** `n` code units from `at`, via a reused plain array (faster than applying a subarray). */
function fromCodes(units: ArrayLike<number>, at: number, n: number): string {
  codes.length = n;
  for (let i = 0; i < n; i++) codes[i] = units[at + i];
  return String.fromCharCode.apply(null, codes);
}

function chunked(units: ArrayLike<number> & { subarray(a: number, b: number): ArrayLike<number> }): string {
  let s = "";
  for (let i = 0; i < units.length; i += CHUNK) {
    s += String.fromCharCode.apply(null, units.subarray(i, Math.min(i + CHUNK, units.length)) as unknown as number[]);
  }
  return s;
}
