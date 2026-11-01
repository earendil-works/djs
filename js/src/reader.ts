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
const SYM_DESCRIPTION = cells.Symbol.fields.DESCRIPTION.offset;
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
const TAG_PREFIX = LAYOUT.value.tagPrefix;
const HI_HOLE = ((TAG_PREFIX << 16) | tags.HOLE) >>> 0;

const tagNames: string[] = [];
for (const [name, t] of Object.entries(tags)) tagNames[t] = name;
const classNames: string[] = [];
for (const [name, c] of Object.entries(LAYOUT.objectClasses)) classNames[c] = name;

/** Strings shorter than this are built with String.fromCharCode instead of a TextDecoder. */
const SHORT = 48;
/** String.fromCharCode.apply argument chunk. */
const CHUNK = 0x2000;

const utf16Decoder = new TextDecoder("utf-16le", { fatal: true, ignoreBOM: true });
// "latin1" is windows-1252 in the Encoding standard. It agrees with Latin-1 everywhere but
// 0x80..0x9F, so it is only used for data without bytes in that range.
const win1252Decoder = new TextDecoder("windows-1252");

/**
 * A VM value with no host equivalent: a symbol, a function or other non-plain object, a
 * BigInt, a host ref or an internal value. `ref` is the arena offset of its cell (0 if none).
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
  /**
   * Atom index -> key string, filled while reading. Reusing one array across reads of the
   * same VM is safe: atoms are never freed and their strings never change.
   */
  atomCache?: (string | undefined)[];
}

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
 * left out. Everything else becomes an {@link Opaque}. Each heap object is read once, so
 * shared references stay shared and cycles become cycles.
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
  const r = new Reader(memory, opts.atomCache ?? []);
  if (hi >>> 16 !== TAG_PREFIX) {
    const scratch = new DataView(new ArrayBuffer(8));
    scratch.setUint32(0, lo, true);
    scratch.setUint32(4, hi, true);
    return scratch.getFloat64(0, true);
  }
  return r.tagged(hi & 0xffff, lo >>> 0);
}

class Reader {
  mem: Uint8Array;
  view: DataView;
  u16: Uint16Array | null;
  atoms: (string | undefined)[];
  atomEntries: number;
  memo = new Map<number, unknown>();

  constructor(mem: Uint8Array, atoms: (string | undefined)[]) {
    this.mem = mem;
    this.view = new DataView(mem.buffer, mem.byteOffset, mem.byteLength);
    // Two-byte string data is 2-aligned within the arena; a view needs the base aligned too.
    this.u16 = mem.byteOffset % 2 === 0 ? new Uint16Array(mem.buffer, mem.byteOffset, mem.length >> 1) : null;
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

  slotIfString(off: number): string | undefined {
    const hi = this.view.getUint32(off + 4, true);
    if (hi !== ((TAG_PREFIX << 16) | tags.STRING) >>> 0) return undefined;
    return this.string(this.view.getUint32(off, true));
  }

  string(off: number): string {
    const kind = this.mem[off + HDR_KIND];
    if (kind === K_STR8) return this.latin1(off + STR8_DATA, this.view.getUint32(off + STR8_LEN, true));
    if (kind === K_STR16) return this.utf16(off + STR16_DATA, this.view.getUint32(off + STR16_LEN, true));
    if (kind === K_ROPE) return this.rope(off);
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
      } else if (this.view.getUint16(c + HDR_FLAGS, true) & ROPE_FLATTENED) {
        stack.push(this.view.getUint32(c + ROPE_LEFT, true));
      } else {
        stack.push(this.view.getUint32(c + ROPE_RIGHT, true), this.view.getUint32(c + ROPE_LEFT, true));
      }
    }
    return parts.join("");
  }

  latin1(at: number, n: number): string {
    const mem = this.mem;
    if (n < SHORT) {
      return String.fromCharCode.apply(null, mem.subarray(at, at + n) as unknown as number[]);
    }
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
    const u16 = this.u16;
    if (u16 === null) {
      const copy = new Uint16Array(n);
      for (let i = 0; i < n; i++) copy[i] = this.view.getUint16(at + 2 * i, true);
      return chunked(copy);
    }
    const units = u16.subarray(at >> 1, (at >> 1) + n);
    if (n < SHORT) return String.fromCharCode.apply(null, units as unknown as number[]);
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
    const seen = this.memo.get(off);
    if (seen !== undefined) return seen;
    const cls = this.view.getUint32(off + OBJ_CLASS, true);
    if (cls === C_ARRAY) return this.array(off);
    if (cls === C_ORDINARY) return this.plain(off);
    const o = new Opaque("object", off, classNames[cls] ?? `class ${cls}`);
    this.memo.set(off, o);
    return o;
  }

  plain(off: number): Record<string, unknown> {
    const view = this.view;
    const o: Record<string, unknown> = {};
    this.memo.set(off, o);
    const el = view.getUint32(off + OBJ_ELEMENTS, true);
    if (el !== 0) {
      const n = view.getUint32(el + VA_LEN, true);
      for (let i = 0; i < n; i++) {
        const at = el + VA_VALUES + 8 * i;
        if (view.getUint32(at + 4, true) !== HI_HOLE) o[i] = this.slot(at);
      }
    }
    const t = view.getUint32(off + OBJ_PROPS, true);
    if (t === 0) return o;
    const count = view.getUint32(t + TAB_COUNT, true);
    for (let i = 0; i < count; i++) {
      const e = t + TAB_ENTRIES + ENTRY_SIZE * i;
      const atom = view.getUint32(e + ENTRY_KEY, true);
      if (atom === 0) continue; // deleted
      const flags = view.getUint32(e + ENTRY_FLAGS, true);
      if (flags & P_ACCESSOR || !(flags & P_ENUMERABLE)) continue;
      const key = this.key(atom);
      if (key === undefined) continue;
      const v = this.slot(e + ENTRY_VALUE);
      // Assigning "__proto__" would set the prototype instead of defining a property.
      if (key === "__proto__") {
        Object.defineProperty(o, key, { value: v, writable: true, enumerable: true, configurable: true });
      } else {
        o[key] = v;
      }
    }
    return o;
  }

  array(off: number): unknown[] {
    const view = this.view;
    const a: unknown[] = [];
    this.memo.set(off, a);
    const length = this.slot(off + OBJ_SLOTS) as number;
    const el = view.getUint32(off + OBJ_ELEMENTS, true);
    if (el !== 0) {
      const n = view.getUint32(el + VA_LEN, true);
      for (let i = 0; i < n; i++) {
        const at = el + VA_VALUES + 8 * i;
        if (view.getUint32(at + 4, true) === HI_HOLE) {
          a.length = i + 1; // keep the hole
        } else {
          a.push(this.slot(at));
        }
      }
    }
    if (view.getUint16(off + HDR_FLAGS, true) & OBJ_INDEXED_PROPS) {
      // Elements beyond the dense part live in the property table.
      const t = view.getUint32(off + OBJ_PROPS, true);
      const count = t === 0 ? 0 : view.getUint32(t + TAB_COUNT, true);
      for (let i = 0; i < count; i++) {
        const e = t + TAB_ENTRIES + ENTRY_SIZE * i;
        const atom = view.getUint32(e + ENTRY_KEY, true);
        const flags = view.getUint32(e + ENTRY_FLAGS, true);
        if (atom === 0 || flags & P_ACCESSOR || !(flags & P_ENUMERABLE)) continue;
        const key = this.key(atom);
        if (typeof key === "number" || (typeof key === "string" && isArrayIndex(key))) {
          a[Number(key)] = this.slot(e + ENTRY_VALUE);
        }
      }
    }
    if (a.length < length) a.length = length;
    return a;
  }
}

/** Array indexes that are string atoms (2^31 .. 2^32 - 2). */
function isArrayIndex(s: string): boolean {
  const n = Number(s);
  return n >= 0 && n < 4294967295 && Number.isInteger(n) && String(n) === s;
}

function chunked(units: ArrayLike<number> & { subarray(a: number, b: number): ArrayLike<number> }): string {
  let s = "";
  for (let i = 0; i < units.length; i += CHUNK) {
    s += String.fromCharCode.apply(null, units.subarray(i, Math.min(i + CHUNK, units.length)) as unknown as number[]);
  }
  return s;
}
