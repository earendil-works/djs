// A host-side dumper (0167 prototype, docs/functions-as-code.md "Rust or TypeScript"):
// the algorithm of crates/djs-core/src/vm/dump.rs over the heap bytes, the way
// js/src/reader.ts reads values, so that the two can be compared on the same cases.
//
// What the layout table (layout.generated.ts) does not describe is copied here from the
// Rust sources and marked "(engine)": function flags, the intrinsics' numbering, object
// class slots, lazy function properties. A shipping version would need those exported.
// Gaps against the Rust dumper: user globals a function reads are found by scanning its
// source for identifiers (an over-approximation), errors lose their `stack`, builtins
// are named by the walk's first two passes (globals, realm roots, then [[Prototype]]
// links; no installed-intrinsic pass).

import { LAYOUT } from "../src/layout.generated.ts";
import { INTRINSIC, REALM_ROOTS } from "./realm.generated.ts";

export const HEADER = "// djs-dump 1";

const C = LAYOUT.cells;
const TAGP = LAYOUT.value.tagPrefix;
const T = LAYOUT.value.tags;
const HDR_KIND = LAYOUT.header.KIND;
const HDR_FLAGS = LAYOUT.header.FLAGS;
const HDR_SIZE = LAYOUT.header.SIZE;
const F_EXT = LAYOUT.flags.OBJ_EXTENSIBLE;
const F_IDX = LAYOUT.flags.OBJ_INDEXED_PROPS;
const F_LEN_RO = LAYOUT.flags.OBJ_LENGTH_READONLY;
const F_DICT = LAYOUT.flags.OBJ_DICTIONARY;
const F_LAZY_LENGTH = 256; // (engine) heap/cells.rs OBJ_LAZY_LENGTH
const F_LAZY_NAME = 512; // (engine) OBJ_LAZY_NAME
const SYM_PRIVATE = 1; // (engine) cells::flags::SYM_PRIVATE
const ROOT_FROZEN = 4; // (engine) ROOT_FROZEN_BUILTINS
const P_W = LAYOUT.propFlags.WRITABLE;
const P_E = LAYOUT.propFlags.ENUMERABLE;
const P_C = LAYOUT.propFlags.CONFIGURABLE;
const P_ACC = LAYOUT.propFlags.ACCESSOR;
const P_ALL = P_W | P_E | P_C;
const A_INT = LAYOUT.atom.intBit;
const A_SYM = LAYOUT.atom.symbolBit;
const A_INDEX = LAYOUT.atom.indexMask;
const CL = LAYOUT.objectClasses;
// (engine) bytecode/template.rs func_flags.
const K_SCRIPT = 0, K_BODY = 1, K_NORMAL = 2, K_ARROW = 3, K_METHOD = 4, K_GETTER = 5, K_SETTER = 6;
const K_CLASS = 7, K_DERIVED = 8, K_FIELD = 9;
const FF_ASYNC = 1 << 4, FF_GEN = 1 << 5, FF_HOME = 1 << 6, FF_PRIV = 1 << 8;
// (engine) heap/typed.rs Kind, regexp Flags, promise states.
const TA_NAMES = ["Int8Array", "Uint8Array", "Uint8ClampedArray", "Int16Array", "Uint16Array", "Int32Array",
  "Uint32Array", "Float16Array", "Float32Array", "Float64Array", "BigInt64Array", "BigUint64Array"];
const TA_SIZES = [1, 1, 1, 2, 2, 4, 4, 2, 4, 8, 8, 8];
const RE_FLAGS = "dgimsuvy";
const PROMISE_PENDING = 0, PROMISE_FULFILLED = 1;

/** A value: tagged ones as `tag * 2^32 + payload`, doubles boxed. */
class Num {
  readonly n: number;
  constructor(n: number) {
    this.n = n;
  }
}
type Val = number | Num;
const W = 4294967296;
const tagOf = (v: Val): number => (typeof v === "number" ? Math.floor(v / W) : 0);
const payOf = (v: Val): number => (v as number) % W;
const isRef = (v: Val): boolean => {
  const t = tagOf(v);
  return t === T.OBJECT || t === T.SYMBOL || t === T.INTERNAL;
};
const mk = (tag: number, p: number): Val => tag * W + p;

interface Prop {
  atom: number;
  /** The string key, or null for a symbol key. */
  name: string | null;
  /** A symbol key's cell. */
  sym: number;
  index: number;
  flags: number;
  value: Val;
  getter: Val;
  setter: Val;
}

const enc = new TextEncoder();
const dec = new TextDecoder();

/** Reads a heap image (arena offset 0 = the roots cell). */
class Heap {
  mem: Uint8Array;
  dv: DataView;
  atomEntries: number;
  atomBase: number;
  atomBaseCount: number;
  atoms: (string | number)[] = [];
  scripts = new Map<number, Uint8Array>();
  constructor(mem: Uint8Array) {
    this.mem = mem;
    this.dv = new DataView(mem.buffer, mem.byteOffset, mem.byteLength);
    const t = this.u32(C.Roots.fields.ATOMS.offset);
    this.atomEntries = this.u32(t + C.AtomTable.fields.ENTRIES.offset);
    this.atomBase = this.u32(t + C.AtomTable.fields.BASE_ENTRIES.offset);
    this.atomBaseCount = this.u32(t + C.AtomTable.fields.BASE_COUNT.offset);
  }
  u32(o: number): number {
    return this.dv.getUint32(o, true);
  }
  u16(o: number): number {
    return this.dv.getUint16(o, true);
  }
  val(o: number): Val {
    const hi = this.u32(o + 4);
    if (hi >>> 16 !== TAGP) return new Num(this.dv.getFloat64(o, true));
    return (hi & 0xffff) * W + this.u32(o);
  }
  root(field: keyof typeof C.Roots.fields): number {
    return this.u32(C.Roots.fields[field].offset);
  }
  intrinsic(i: number): Val {
    return this.val(this.root("REALM") + C.Realm.tail!.offset + 8 * i);
  }
  str(off: number): string {
    const k = this.mem[off + HDR_KIND];
    if (k === C.Str8.kind) return this.latin1(off + C.Str8.tail!.offset, this.u32(off + C.Str8.fields.LEN.offset));
    if (k === C.Str16.kind) return this.utf16(off + C.Str16.tail!.offset, this.u32(off + C.Str16.fields.LEN.offset));
    if (k === C.Slice.kind) {
      const parent = this.u32(off + C.Slice.fields.PARENT.offset);
      const at = parent + C.Str8.tail!.offset + this.u32(off + C.Slice.fields.OFF.offset);
      return this.latin1(at, this.u32(off + C.Slice.fields.LEN.offset));
    }
    // Ropes.
    const flags = this.u16(off + HDR_FLAGS);
    const left = this.u32(off + C.Rope.fields.LEFT.offset);
    if (flags & LAYOUT.flags.ROPE_APPEND) {
      const n = this.u32(off + C.Rope.fields.LEN.offset);
      return this.mem[left + HDR_KIND] === C.Str8.kind
        ? this.latin1(left + C.Str8.tail!.offset, n)
        : this.utf16(left + C.Str16.tail!.offset, n);
    }
    if (flags & LAYOUT.flags.ROPE_FLATTENED) return this.str(left);
    return this.str(left) + this.str(this.u32(off + C.Rope.fields.RIGHT.offset));
  }
  latin1(at: number, n: number): string {
    let s = "";
    for (let i = 0; i < n; i += 0x2000) {
      s += String.fromCharCode.apply(null, this.mem.subarray(at + i, at + Math.min(n, i + 0x2000)) as unknown as number[]);
    }
    return s;
  }
  utf16(at: number, n: number): string {
    let s = "";
    for (let i = 0; i < n; i++) s += String.fromCharCode(this.u16(at + 2 * i));
    return s;
  }
  /** An atom's string, or its symbol's cell. */
  atom(a: number): string | number {
    if (a & A_INT) return String((a & ~A_INT) >>> 0);
    const i = a & A_INDEX;
    let s = this.atoms[i];
    if (s === undefined) {
      const slot = i < this.atomBaseCount
        ? this.atomBase + C.ValueArray.tail!.offset + 8 * i
        : this.atomEntries + C.ValueArray.tail!.offset + 8 * (i - this.atomBaseCount);
      const v = this.val(slot);
      s = tagOf(v) === T.STRING ? this.str(payOf(v)) : payOf(v);
      this.atoms[i] = s;
    }
    return s;
  }
  cls(o: number): number {
    return this.u32(o + C.Object.fields.CLASS.offset);
  }
  proto(o: number): number {
    return this.u32(o + C.Object.fields.PROTO.offset);
  }
  oflags(o: number): number {
    return this.u16(o + HDR_FLAGS);
  }
  extensible(o: number): boolean {
    return (this.oflags(o) & F_EXT) !== 0;
  }
  slot(o: number, i: number): Val {
    return this.val(o + C.Object.tail!.offset + 8 * i);
  }
  elements(o: number): number {
    return this.u32(o + C.Object.fields.ELEMENTS.offset);
  }
  vaLen(a: number): number {
    return this.u32(a + C.ValueArray.fields.LEN.offset);
  }
  vaGet(a: number, i: number): Val {
    return this.val(a + C.ValueArray.tail!.offset + 8 * i);
  }
  /** Own properties in [[OwnPropertyKeys]] order (heap/object.rs ordinary_own_keys). */
  props(o: number): { props: Prop[]; private: boolean } {
    const idx: Prop[] = [], strs: Prop[] = [], syms: Prop[] = [];
    let priv = false;
    const cls = this.cls(o);
    const el = cls === CL.Function ? 0 : this.elements(o);
    const mkp = (atom: number, flags: number, v: Val): Prop => {
      const p: Prop = { atom, name: null, sym: 0, index: -1, flags, value: v, getter: mk(T.UNDEFINED, 0), setter: mk(T.UNDEFINED, 0) };
      if (flags & P_ACC) {
        const pair = payOf(v);
        p.getter = this.val(pair + C.AccessorPair.fields.GETTER.offset);
        p.setter = this.val(pair + C.AccessorPair.fields.SETTER.offset);
      }
      const k = this.atom(atom);
      if (typeof k === "number") p.sym = k;
      else p.name = k;
      if (atom & A_INT) p.index = (atom & ~A_INT) >>> 0;
      return p;
    };
    if (el !== 0 && cls !== CL.TypedArray) {
      const n = this.vaLen(el);
      for (let i = 0; i < n; i++) {
        const v = this.vaGet(el, i);
        if (tagOf(v) !== T.HOLE) idx.push(mkp((A_INT | i) >>> 0, P_ALL, v));
      }
    }
    const f = this.oflags(o);
    const t = this.u32(o + C.Object.fields.PROPS.offset);
    if (t !== 0) {
      const dict = (f & F_DICT) !== 0;
      const count = this.u32(t + (dict ? C.PropTable.fields.COUNT.offset : C.PropValues.fields.LEN.offset));
      const descs = dict ? 0 : this.u32(this.u32(t + C.PropValues.fields.SHAPE.offset) + C.Shape.fields.DESCS.offset) + C.U32Array.tail!.offset + 4;
      let sort = false;
      for (let i = 0; i < count; i++) {
        const e = dict ? t + C.PropTable.tail!.offset + C.PropTable.tail!.elemSize * i : descs + 8 * i;
        const atom = this.u32(e + (dict ? C.PropTable.tail!.fields.KEY.offset : 0));
        if (atom === 0) continue;
        const flags = this.u32(e + (dict ? C.PropTable.tail!.fields.FLAGS.offset : 4));
        const v = this.val(dict ? e + C.PropTable.tail!.fields.VALUE.offset : t + C.PropValues.tail!.offset + 8 * i);
        if (atom & A_SYM && !(atom & A_INT)) {
          const s = this.atom(atom) as number;
          if (this.u16(s + HDR_FLAGS) & SYM_PRIVATE) priv = true;
          else syms.push(mkp(atom, flags, v));
          continue;
        }
        const p = mkp(atom, flags, v);
        if (f & F_IDX && p.index < 0 && isIndex(p.name!)) p.index = Number(p.name);
        if (f & F_IDX && p.index >= 0) {
          idx.push(p);
          sort = true;
        } else strs.push(p);
      }
      if (sort) idx.sort((a, b) => a.index - b.index);
    }
    const out = idx;
    if (cls === CL.Array) {
      out.push({ atom: 0, name: "length", sym: 0, index: -1, flags: f & F_LEN_RO ? 0 : P_W, value: this.slot(o, 0), getter: 0, setter: 0 });
    }
    if (f & F_LAZY_LENGTH || f & F_LAZY_NAME) {
      const tpl = this.elements(o);
      if (f & F_LAZY_LENGTH) {
        out.push({ atom: 0, name: "length", sym: 0, index: -1, flags: P_C, value: mk(T.INT, this.u16(tpl + C.FuncTemplate.fields.LENGTH.offset)), getter: 0, setter: 0 });
      }
      if (f & F_LAZY_NAME) {
        const a = this.u32(tpl + C.FuncTemplate.fields.NAME.offset);
        out.push({ atom: 0, name: "name", sym: 0, index: -1, flags: P_C, value: new Num(NaN), getter: 0, setter: 0, lazyName: a === 0 ? "" : (this.atom(a) as string) } as Prop);
      }
    }
    return { props: out.concat(strs, syms), private: priv };
  }
  /** A function's source text (builtins/function.rs source_slice): UTF-8 offsets. */
  source(f: number): string | null {
    const t = this.elements(f);
    const flags = this.u16(t + C.FuncTemplate.fields.FLAGS.offset);
    if ((flags & 15) === K_SCRIPT || (flags & 15) === K_BODY) return null;
    const info = this.u32(t + C.FuncTemplate.fields.INFO.offset);
    if (info === 0) return null;
    const script = this.u32(info + C.FuncInfo.fields.SCRIPT.offset);
    if (script === 0) return null;
    let bytes = this.scripts.get(script);
    if (bytes === undefined) {
      const src = this.u32(script + C.Script.fields.SOURCE.offset);
      if (src === 0) return null;
      bytes = enc.encode(this.str(src));
      this.scripts.set(script, bytes);
    }
    return dec.decode(bytes.subarray(this.u32(info + C.FuncInfo.fields.SRC_START.offset), this.u32(info + C.FuncInfo.fields.SRC_END.offset)));
  }
  templateNames(t: number): string[] {
    const info = this.u32(t + C.FuncTemplate.fields.INFO.offset);
    const names = info === 0 ? 0 : this.u32(info + C.FuncInfo.fields.NAMES.offset);
    if (names === 0) return [];
    const n = this.u32(names + C.U32Array.fields.LEN.offset);
    const out: string[] = [];
    for (let i = 0; i < n; i++) {
      const a = this.atom(this.u32(names + C.U32Array.tail!.offset + 4 * i));
      out.push(typeof a === "string" ? a : "");
    }
    return out;
  }
}

/** A property's string value (lazy names are not stored). */
function propValue(h: Heap, p: Prop): Val | string {
  const lazy = (p as Prop & { lazyName?: string }).lazyName;
  return lazy !== undefined ? lazy : p.value;
}

function isIndex(s: string): boolean {
  const n = Number(s);
  return n >= 0 && n < 4294967295 && Number.isInteger(n) && String(n) === s;
}

const RESERVED = new Set(("break case catch class const continue debugger default delete do else enum export extends false " +
  "finally for function if import in instanceof new null return super switch this throw true try typeof var void while " +
  "with yield let static implements interface package private protected public await").split(" "));
const isIdent = (s: string): boolean => /^[A-Za-z_$][\w$]*$/.test(s) && !RESERVED.has(s);
const quote = (s: string): string =>
  JSON.stringify(s).replace(/[\u2028\u2029\x7f]/g, (c) => "\\u" + c.charCodeAt(0).toString(16).padStart(4, "0"));
const member = (k: string): string => (isIdent(k) ? `.${k}` : `[${quote(k)}]`);
const num = (n: number): string => (Object.is(n, -0) ? "-0" : String(n));

function anonymousForm(kind: number, src: string): boolean {
  if (kind === K_ARROW) return true;
  return kind === K_NORMAL && /^(async\s*)?function\s*\*?\s*\(/.test(src);
}
function declaredName(kind: number, src: string): string | null {
  if (kind !== K_NORMAL) return null;
  return /^(?:async\s*)?function\s+([\w$]+)/.exec(src)?.[1] ?? null;
}

// ---- builtin names (vm/registry.rs `names`, without the installed-intrinsic pass) ----

function segment(h: Heap, p: Prop): string | null {
  if (p.name === null) {
    const d = h.val(p.sym + C.Symbol.fields.DESCRIPTION.offset);
    if (tagOf(d) !== T.STRING) return null;
    const s = h.str(payOf(d));
    return s.startsWith("Symbol.") ? `[${s}]` : null;
  }
  return /^[A-Za-z_$][\w$]*$/.test(p.name) ? `.${p.name}` : `[${JSON.stringify(p.name)}]`;
}
const cellOf = (v: Val): number => (isRef(v) ? payOf(v) : 0);

function builtinNames(h: Heap): Map<number, string> {
  const seen = new Map<number, string>();
  const order: [number, string][] = [];
  const edges = (o: number, proto: boolean): [string, number, boolean][] => {
    const out: [string, number, boolean][] = [];
    for (const p of h.props(o).props) {
      const seg = segment(h, p);
      if (seg === null) continue;
      if (p.flags & P_ACC) {
        if (cellOf(p.getter)) out.push([`get ${seg}`, cellOf(p.getter), false]);
        if (cellOf(p.setter)) out.push([`set ${seg}`, cellOf(p.setter), false]);
      } else if (cellOf(p.value)) out.push([seg, cellOf(p.value), tagOf(p.value) === T.OBJECT]);
    }
    out.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
    if (proto && h.proto(o)) out.push([".[[Prototype]]", h.proto(o), true]);
    return out;
  };
  const child = (path: string, seg: string): string =>
    seg.startsWith("get ") || seg.startsWith("set ") ? `${seg.slice(0, 4)}${path}${seg.slice(4)}` : path + seg;
  const walk = (roots: [number, string][], proto: boolean): void => {
    const q: [number, string, boolean, boolean][] = roots.map(([c, p]) => [c, p, true, true]);
    for (let i = 0; i < q.length; i++) {
      const [c, path, expand, root] = q[i];
      if (!c) continue;
      if (!seen.has(c)) {
        seen.set(c, path);
        order.push([c, path]);
      } else if (!root) continue;
      if (!expand || h.mem[c + HDR_KIND] !== C.Object.kind) continue;
      for (const [seg, v, ex] of edges(c, proto)) q.push([v, child(path, seg), ex, false]);
    }
  };
  const g = payOf(h.intrinsic(INTRINSIC.Global));
  seen.set(g, "globalThis");
  order.push([g, "globalThis"]);
  walk(edges(g, false).map(([seg, v]) => {
    const p = child("globalThis", seg);
    return [v, p.startsWith("globalThis.") ? p.slice(11) : p];
  }), false);
  walk(REALM_ROOTS.map(([i, name]) => [cellOf(h.intrinsic(i)), `%${name}%`]), false);
  walk(order.slice(), true);
  return seen;
}

/** Builtin names per base image (they cost a walk of the base). */
const namesCache = new WeakMap<Uint8Array, Map<number, string>>();

// ---- the dumper ----

const LOADER_ROOTS: Record<string, string> = {
  IteratorPrototype: "Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()))",
  ArrayIteratorPrototype: "Object.getPrototypeOf([][Symbol.iterator]())",
  StringIteratorPrototype: "Object.getPrototypeOf(''[Symbol.iterator]())",
  MapIteratorPrototype: "Object.getPrototypeOf(new Map().entries())",
  SetIteratorPrototype: "Object.getPrototypeOf(new Set().values())",
  RegExpStringIteratorPrototype: "Object.getPrototypeOf(/./[Symbol.matchAll](''))",
  GeneratorFunction: "(function* () {}).constructor",
  GeneratorPrototype: "Object.getPrototypeOf(function* () {}).prototype",
  AsyncFunction: "(async function () {}).constructor",
  AsyncGeneratorFunction: "(async function* () {}).constructor",
  AsyncGeneratorPrototype: "Object.getPrototypeOf(async function* () {}).prototype",
  TypedArray: "Object.getPrototypeOf(Int8Array)",
  TypedArrayPrototype: "Object.getPrototypeOf(Int8Array.prototype)",
  ThrowTypeError: "Object.getOwnPropertyDescriptor(Function.prototype, 'caller').get",
};
const HELPERS: Record<string, string> = {
  bytes: "$dump.bytes = (s) => {\n  const b = atob(s), u = new Uint8Array(b.length);\n  for (let i = 0; i < b.length; i++) u[i] = b.charCodeAt(i);\n  return u.buffer;\n};\n",
  fn: "$dump.fn = (o) => {\n  const d = Object.getOwnPropertyDescriptor(o, Reflect.ownKeys(o)[0]);\n  return d.value || d.get || d.set;\n};\n",
  rejected: "$dump.rejected = (e) => {\n  const p = Promise.reject(e);\n  p.catch(() => {});\n  return p;\n};\n",
};
const LOADER = "$dump.builtin = (path) => {\n  const roots = {\n" +
  Object.entries(LOADER_ROOTS).map(([n, e]) => `    ${n}: () => ${e},\n`).join("") +
  "  };\n  const [, acc, root, rest] = /^(get |set )?(%\\w+%|[\\w$]+)(.*)$/.exec(path);\n" +
  "  let v = root[0] === \"%\" ? roots[root.slice(1, -1)]() : globalThis[root];\n" +
  "  const segs = rest.match(/\\.\\[\\[Prototype\\]\\]|\\.[\\w$]+|\\[\"(?:[^\"\\\\]|\\\\.)*\"\\]|\\[Symbol\\.\\w+\\]/g) || [];\n" +
  "  segs.forEach((s, i) => {\n    if (s === \".[[Prototype]]\") return void (v = Object.getPrototypeOf(v));\n" +
  "    const key = s[0] === \".\" ? s.slice(1) : s.startsWith(\"[Symbol.\") ? Symbol[s.slice(8, -1)] : JSON.parse(s.slice(1, -1));\n" +
  "    if (acc && i === segs.length - 1) {\n      const d = Object.getOwnPropertyDescriptor(v, key);\n      v = acc === \"get \" ? d.get : d.set;\n    } else v = v[key];\n  });\n  return v;\n};\n";

type Decl = "const" | "let" | "var";
const NEW = 0, BUSY = 1, DONE = 2;

interface Node {
  v: Val;
  path: string;
  refs: number;
  edges: number[];
  cyclic: boolean;
  root: number;
  state: number;
  loc: string;
  group: number;
}
interface Root {
  name: string;
  decl: Decl;
  value: Val;
  state: number;
}
interface Group {
  closures: number[];
  boxes: number[];
  state: number;
  loc: string;
}

export class DumpRefused extends Error {
  reasons: string[];
  constructor(reasons: string[]) {
    super(`cannot dump:\n  ${reasons.join("\n  ")}`);
    this.reasons = reasons;
  }
}

export interface DumpOptions {
  /** The base image the VM was made from. */
  base: Uint8Array;
  /** Globals the host defines (re-bound by name). */
  host?: string[];
  /** The bindings to dump (default: all of the VM's own). */
  names?: string[];
}

/** Dumps the VM whose arena is `arena` (Vm::dump's text, from the heap bytes). */
export function dumpHeap(arena: Uint8Array, opts: DumpOptions): string {
  return new Dumper(arena, opts).run(opts.names);
}

class Dumper {
  h: Heap;
  base: Heap;
  baseEnd: number;
  builtins: Map<number, string>;
  host = new Map<number, string>();
  hostNames: string[];
  globals = new Map<string, { value: Val; decl: Decl; root: number }>();
  roots: Root[] = [];
  nodes: Node[] = [];
  ids = new Map<number, number>();
  groups: Group[] = [];
  refusals: string[] = [];
  builtinPatches: [string, Prop, Prop | null][] = [];
  out: string[] = [];
  patches: string[] = [];
  finals: string[] = [];
  helpers = new Set<string>();
  ns = false;
  next = 0;
  depth = 0;
  visited = 0;
  global: number;
  idents = new Map<number, Set<string>>();

  constructor(arena: Uint8Array, opts: DumpOptions) {
    this.h = new Heap(arena);
    this.base = new Heap(opts.base);
    this.baseEnd = this.h.root("BASE_END");
    let names = namesCache.get(opts.base);
    if (names === undefined) {
      // Only the prefix is shared by every VM of the base (the global object, for one, is
      // in the per-VM tail).
      names = new Map([...builtinNames(this.base)].filter(([c]) => c < this.baseEnd));
      namesCache.set(opts.base, names);
    }
    this.builtins = names;
    this.hostNames = opts.host ?? [];
    this.global = payOf(this.h.intrinsic(INTRINSIC.Global));
    this.readGlobals();
    this.readPatches();
  }

  refuse(path: string, why: string): void {
    const l = `${path}: ${why}`;
    if (!this.refusals.includes(l)) this.refusals.push(l);
  }

  readGlobals(): void {
    const h = this.h, b = this.base;
    const lex = h.u32(h.root("REALM") + C.Realm.fields.GLOBAL_LEX.offset);
    for (const p of h.props(lex).props) {
      if (p.name !== null && !(p.flags & P_ACC)) this.binding(p.name, p.value, p.flags & P_W ? "let" : "const");
    }
    const bg = payOf(b.intrinsic(INTRINSIC.Global));
    const baseProps = new Map<string, Prop>();
    for (const p of b.props(bg).props) if (p.name !== null) baseProps.set(p.name, p);
    for (const p of h.props(this.global).props) {
      if (p.name === null) continue;
      const bp = baseProps.get(p.name);
      if (bp !== undefined && samePropBits(p, bp)) continue;
      if (p.flags & P_ACC) {
        if (!this.hostNames.includes(p.name)) this.refuse(p.name, "a global accessor");
      } else this.binding(p.name, p.value, "var");
    }
  }

  binding(name: string, v: Val, decl: Decl): void {
    if (this.hostNames.includes(name)) this.hostValues(v, name, 0);
    else if (name !== "$dump" && !this.globals.has(name)) this.globals.set(name, { value: v, decl, root: -1 });
  }

  hostValues(v: Val, expr: string, depth: number): void {
    if (tagOf(v) !== T.OBJECT) return;
    const o = payOf(v);
    if (o < this.baseEnd || this.host.has(o)) return;
    this.host.set(o, expr);
    if (depth >= 2 || this.h.cls(o) === CL.Proxy) return;
    for (const p of this.h.props(o).props) {
      if (p.name !== null && !(p.flags & P_ACC)) this.hostValues(p.value, expr + member(p.name), depth + 1);
    }
  }

  readPatches(): void {
    const h = this.h, b = this.base;
    if (h.root("FLAGS") & ROOT_FROZEN) return;
    const start = h.u32(HDR_SIZE);
    if (this.baseEnd > b.mem.length) return;
    let same = true;
    for (let i = start; i < this.baseEnd; i += 4) {
      if (h.u32(i) !== b.u32(i)) {
        same = false;
        break;
      }
    }
    if (same) return;
    for (const [cell, path] of this.builtins) {
      if (b.mem[cell + HDR_KIND] !== C.Object.kind) continue;
      if (h.proto(cell) !== b.proto(cell)) this.refuse(path, "a builtin whose prototype was changed");
      let e: string | undefined;
      const expr = (): string => (e ??= this.builtinExpr(path));
      if (h.extensible(cell) !== b.extensible(cell)) this.finals.push(`Object.preventExtensions(${expr()});`);
      const now = h.props(cell).props, then = b.props(cell).props;
      const key = (p: Prop): string => (p.name === null ? `@${b.atom(p.atom)}` : p.name);
      const nowBy = new Map(now.map((p) => [p.name ?? `@${p.sym}`, p]));
      for (const bp of then) {
        const p = nowBy.get(key(bp));
        if (p === undefined) this.builtinPatches.push([expr(), bp, null]);
        else if (!samePropBits(p, bp, h, b)) this.builtinPatches.push([expr(), p, p]);
      }
      const thenNames = new Set(then.map((p) => p.name ?? `@${p.sym}`));
      for (const p of now) if (!thenNames.has(p.name ?? `@${p.sym}`)) this.builtinPatches.push([expr(), p, p]);
    }
    for (const [expr, k, p] of this.builtinPatches) {
      if (p === null) continue;
      const path = expr + this.keyPath(k);
      if (p.flags & P_ACC) {
        this.note(p.getter, () => `get ${path}`);
        this.note(p.setter, () => `set ${path}`);
      } else this.note(p.value, () => path);
    }
  }

  builtinExpr(path: string): string {
    if (path.includes("%") || path.includes("~") || path.includes("[[") || path.startsWith("get ") || path.startsWith("set ")) {
      this.helpers.add("builtin");
      this.ns = true;
      return `$dump.builtin(${quote(path)})`;
    }
    return path;
  }

  run(names: string[] | undefined): string {
    if (names !== undefined && names.length > 0) {
      for (const n of names) {
        if (this.globals.has(n)) this.addGlobalRoot(n);
        else if (!this.hostNames.includes(n)) this.refuse(n, "no such global");
      }
    } else for (const n of [...this.globals.keys()]) this.addGlobalRoot(n);
    return this.finish();
  }

  addGlobalRoot(name: string): void {
    const g = this.globals.get(name)!;
    if (g.root >= 0) return;
    g.root = this.roots.length;
    this.pushRoot(name, g.decl, g.value);
    this.drain();
  }

  pushRoot(name: string, decl: Decl, value: Val): void {
    const i = this.roots.length;
    if (tagOf(value) === T.UNINITIALIZED) this.refuse(name, "a binding before its declaration ran (TDZ)");
    else {
      const n = this.note(value, () => name);
      if (n >= 0 && this.nodes[n].root < 0) this.nodes[n].root = i;
    }
    this.roots.push({ name, decl, value, state: NEW });
  }

  note(v: Val, path: () => string): number {
    if (!isRef(v)) return -1;
    const c = payOf(v);
    const p = this.builtins.get(c);
    if (p !== undefined) {
      const bare = p.replace(/^(get|set) /, "");
      if (p.includes("~") || bare.startsWith("%intrinsic:")) this.refuse(path(), `a builtin's internal value (${p})`);
      else if (bare.startsWith("%") && !(bare.slice(1, bare.indexOf("%", 1)) in LOADER_ROOTS)) {
        this.refuse(path(), `a builtin the loader cannot name (${p})`);
      }
      return -1;
    }
    if (this.host.has(c) || c === this.global) return -1;
    if (c < this.baseEnd) {
      this.refuse(path(), "a builtin without a name");
      return -1;
    }
    const n = this.ids.get(c);
    if (n !== undefined) {
      this.nodes[n].refs++;
      return n;
    }
    this.ids.set(c, this.nodes.length);
    this.nodes.push({ v, path: path(), refs: 1, edges: [], cyclic: false, root: -1, state: NEW, loc: "", group: -1 });
    return this.nodes.length - 1;
  }

  drain(): void {
    while (this.visited < this.nodes.length) this.visit(this.visited++);
  }

  edge(from: number, v: Val, path: () => string): number {
    const n = this.note(v, path);
    if (n >= 0) this.nodes[from].edges.push(n);
    return n;
  }

  keyPath(p: Prop): string {
    if (p.name !== null) return member(p.name);
    const b = this.builtins.get(p.sym);
    if (b !== undefined) return `[${b}]`;
    const d = this.h.val(p.sym + C.Symbol.fields.DESCRIPTION.offset);
    return `[Symbol(${tagOf(d) === T.STRING ? this.h.str(payOf(d)) : ""})]`;
  }

  symbolDescription(s: number): string {
    const d = this.h.val(s + C.Symbol.fields.DESCRIPTION.offset);
    return tagOf(d) === T.STRING ? this.h.str(payOf(d)) : "";
  }

  keyMember(p: Prop): string {
    return p.name !== null ? member(p.name) : `[${this.expr(mk(T.SYMBOL, p.sym), null, null)}]`;
  }

  keyExpr(p: Prop): string {
    return p.name !== null ? quote(p.name) : this.expr(mk(T.SYMBOL, p.sym), null, null);
  }

  keyEdge(n: number, p: Prop): void {
    if (p.name !== null) return;
    const base = this.nodes[n].path;
    const c = this.edge(n, mk(T.SYMBOL, p.sym), () => `${base}<key>`);
    if (c >= 0) this.nodes[c].refs++;
  }

  propEdges(n: number, o: number, skip: (p: Prop) => boolean): void {
    const { props, private: priv } = this.h.props(o);
    if (priv) this.refuse(this.nodes[n].path, "an object with private fields");
    for (const p of props) {
      if (p.name !== null && skip(p)) continue;
      this.keyEdge(n, p);
      const path = this.nodes[n].path + this.keyPath(p);
      if (p.flags & P_ACC) {
        this.edge(n, p.getter, () => `get ${path}`);
        this.edge(n, p.setter, () => `set ${path}`);
      } else if (!("lazyName" in p)) this.edge(n, p.value, () => path);
    }
    const pr = this.h.proto(o);
    if (pr !== 0 && !this.defaultProto(o, pr)) {
      const path = `${this.nodes[n].path}.[[Prototype]]`;
      this.edge(n, mk(T.OBJECT, pr), () => path);
    }
  }

  classProto(o: number): number {
    const h = this.h;
    const I = INTRINSIC;
    switch (h.cls(o)) {
      case CL.Ordinary: return I.ObjectProto;
      case CL.Array: return I.ArrayProto;
      case CL.Function: {
        const f = h.u16(h.elements(o) + C.FuncTemplate.fields.FLAGS.offset);
        return f & FF_ASYNC ? (f & FF_GEN ? I.AsyncGeneratorFunctionProto : I.AsyncFunctionProto) : f & FF_GEN ? I.GeneratorFunctionProto : I.FunctionProto;
      }
      case CL.Map: return I.MapProto;
      case CL.Set: return I.SetProto;
      case CL.Date: return I.DateProto;
      case CL.RegExp: return I.RegExpProto;
      case CL.Promise: return I.PromiseProto;
      case CL.Boolean: return I.BooleanProto;
      case CL.Number: return I.NumberProto;
      case CL.String: return I.StringProto;
      case CL.ArrayBuffer: return I.ArrayBufferProto;
      case CL.BoundFunction: return I.FunctionProto;
      default: return -1;
    }
  }

  defaultProto(o: number, p: number): boolean {
    const c = this.h.cls(o);
    if (c === CL.TypedArray || c === CL.DataView || c === CL.Error) return p !== 0 && p < this.baseEnd;
    const i = this.classProto(o);
    return i < 0 || payOf(this.h.intrinsic(i)) === p;
  }

  visit(n: number): void {
    const h = this.h;
    const v = this.nodes[n].v;
    const path = this.nodes[n].path;
    if (tagOf(v) === T.INTERNAL) {
      const inner = h.val(payOf(v) + C.BoxCell.fields.VALUE.offset);
      if (tagOf(inner) === T.UNINITIALIZED) return this.refuse(path, "a captured binding before its declaration ran (TDZ)");
      this.edge(n, inner, () => path);
      return;
    }
    if (tagOf(v) === T.SYMBOL) return;
    const o = payOf(v);
    switch (h.cls(o)) {
      case CL.Ordinary: return this.propEdges(n, o, () => false);
      case CL.Array: {
        const len = num0(h.slot(o, 0));
        const el = h.elements(o);
        const dense = el ? h.vaLen(el) : 0;
        for (let i = 0; i < len && i < dense; i++) this.edge(n, h.vaGet(el, i), () => `${path}[${i}]`);
        return this.propEdges(n, o, (p) => p.name === "length" || (p.index >= 0 && p.index < dense));
      }
      case CL.Function: return this.closure(n, o);
      case CL.Map:
      case CL.Set: {
        const map = h.cls(o) === CL.Map;
        entries(h, o).forEach(([k, val], i) => {
          this.edge(n, k, () => `${path}<entries>[${i}]`);
          if (map) this.edge(n, val, () => `${path}<entries>[${i}]`);
        });
        return this.propEdges(n, o, () => false);
      }
      case CL.Date:
      case CL.Boolean:
      case CL.Number: return this.propEdges(n, o, () => false);
      case CL.String: return this.propEdges(n, o, (p) => p.name === "length" || p.index >= 0);
      case CL.RegExp: return this.propEdges(n, o, (p) => p.name === "lastIndex");
      case CL.Error: return this.propEdges(n, o, (p) => p.name === "message" || p.name === "stack");
      case CL.Promise: {
        if (num0(h.slot(o, 0)) === PROMISE_PENDING) return this.refuse(path, "a pending promise");
        this.edge(n, h.slot(o, 1), () => `${path}<result>`);
        return this.propEdges(n, o, () => false);
      }
      case CL.BoundFunction: {
        this.edge(n, h.slot(o, 0), () => `${path}<target>`);
        this.edge(n, h.slot(o, 1), () => `${path}<this>`);
        boundArgs(h, o).forEach((a, i) => this.edge(n, a, () => `${path}<args>[${i}]`));
        return this.propEdges(n, o, (p) => p.name === "name" || p.name === "length");
      }
      case CL.ArrayBuffer:
        if (h.slot(o, 1) instanceof Num || tagOf(h.slot(o, 1)) === T.INT) this.refuse(path, "a resizable ArrayBuffer");
        return this.propEdges(n, o, () => false);
      case CL.TypedArray:
      case CL.DataView:
        this.edge(n, h.slot(o, 0), () => `${path}.buffer`);
        return this.propEdges(n, o, (p) => p.index >= 0);
      case CL.HostFunction: return this.refuse(path, "a host function the host does not re-bind by name");
      case CL.Generator: return this.refuse(path, "a suspended generator or async function");
      case CL.Proxy: return this.refuse(path, "a Proxy");
      case CL.WeakMap:
      case CL.WeakSet: return this.refuse(path, "a weak collection");
      default: return this.refuse(path, "an object of a kind dumps do not write");
    }
  }

  closure(n: number, o: number): void {
    const h = this.h;
    const path = this.nodes[n].path;
    const t = h.elements(o);
    const flags = h.u16(t + C.FuncTemplate.fields.FLAGS.offset);
    const kind = flags & 15;
    if (flags & FF_PRIV) return this.refuse(path, "an internal function of a builtin");
    if (kind === K_CLASS || kind === K_DERIVED || kind === K_FIELD) return this.refuse(path, "a class (classes are not dumped yet)");
    if (flags & FF_HOME) return this.refuse(path, "a method that uses super");
    const src = h.source(o);
    if (src === null) return this.refuse(path, "a function without source text");
    const names = h.templateNames(t);
    const locals = h.u16(t + C.FuncTemplate.fields.LOCALS.offset);
    const captures = h.u16(t + C.FuncTemplate.fields.CAPTURES.offset);
    for (let i = 0; i < captures; i++) {
      const name = names[locals + i] ?? "";
      const b = h.slot(o, i);
      if (tagOf(b) !== T.INTERNAL) continue;
      if (!isIdent(name)) {
        this.refuse(`${path}~${name}`, "a captured `this`, `arguments` or `new.target`");
        continue;
      }
      this.edge(n, b, () => `${path}~${name}`);
    }
    // User globals the source names (an over-approximation of what the code reads).
    for (const id of src.match(/[A-Za-z_$][\w$]*/g) ?? []) {
      const g = this.globals.get(id);
      if (g !== undefined && g.root < 0) {
        g.root = this.roots.length;
        this.pushRoot(id, g.decl, g.value);
      }
    }
    for (const p of h.props(o).props) {
      if (p.name !== null && this.defaultFnKey(o, p.name)) continue;
      this.keyEdge(n, p);
      const pp = path + this.keyPath(p);
      if (p.flags & P_ACC) {
        this.edge(n, p.getter, () => `get ${pp}`);
        this.edge(n, p.setter, () => `set ${pp}`);
      } else this.edge(n, p.value, () => pp);
    }
    if (!this.defaultProto(o, h.proto(o))) this.refuse(path, "a function with another prototype");
  }

  defaultFnKey(f: number, name: string): boolean {
    return name === "length" || name === "name" || (name === "prototype" && this.defaultFnPrototype(f));
  }

  defaultFnPrototype(f: number): boolean {
    const p = this.h.props(f).props.find((p) => p.name === "prototype");
    if (p === undefined) return true;
    if (p.flags & P_ACC || tagOf(p.value) !== T.OBJECT) return false;
    const keys = this.h.props(payOf(p.value)).props;
    return keys.length === 0 || (keys.length === 1 && keys[0].name === "constructor" && keys[0].value === mk(T.OBJECT, f));
  }

  analyze(): void {
    const n = this.nodes.length;
    const isBox = this.nodes.map((d) => tagOf(d.v) === T.INTERNAL);
    const parent = Array.from({ length: n }, (_, i) => i);
    const find = (x: number): number => {
      while (parent[x] !== x) x = parent[x] = parent[parent[x]];
      return x;
    };
    const grouped = new Array<boolean>(n).fill(false);
    this.nodes.forEach((d, i) => {
      if (isBox[i]) grouped[i] = true;
      else for (const e of d.edges) if (isBox[e]) {
        grouped[i] = true;
        parent[find(i)] = find(e);
      }
    });
    const groupOf = new Map<number, number>();
    for (let i = 0; i < n; i++) {
      if (!grouped[i]) continue;
      const r = find(i);
      let g = groupOf.get(r);
      if (g === undefined) {
        g = this.groups.length;
        groupOf.set(r, g);
        this.groups.push({ closures: [], boxes: [], state: NEW, loc: "" });
      }
      this.nodes[i].group = g;
      (isBox[i] ? this.groups[g].boxes : this.groups[g].closures).push(i);
    }
    // Tarjan, iterative.
    const index = new Array<number>(n).fill(-1), low = new Array<number>(n).fill(0), on = new Array<boolean>(n).fill(false);
    const stack: number[] = [];
    let next = 0;
    for (let s = 0; s < n; s++) {
      if (index[s] >= 0) continue;
      const work: [number, number][] = [[s, 0]];
      while (work.length > 0) {
        const top = work[work.length - 1];
        const v = top[0];
        if (top[1] === 0 && index[v] < 0) {
          index[v] = low[v] = next++;
          stack.push(v);
          on[v] = true;
        }
        const edges = this.nodes[v].edges;
        if (top[1] < edges.length) {
          const w = edges[top[1]++];
          if (index[w] < 0) work.push([w, 0]);
          else if (on[w]) low[v] = Math.min(low[v], index[w]);
          continue;
        }
        work.pop();
        if (work.length > 0) {
          const u = work[work.length - 1][0];
          low[u] = Math.min(low[u], low[v]);
        }
        if (low[v] === index[v]) {
          const comp: number[] = [];
          for (;;) {
            const w = stack.pop()!;
            on[w] = false;
            comp.push(w);
            if (w === v) break;
          }
          if (comp.length > 1 || edges.includes(v)) for (const w of comp) this.nodes[w].cyclic = true;
        }
      }
    }
  }

  finish(): string {
    this.drain();
    if (this.refusals.length > 0) throw new DumpRefused(this.refusals);
    this.analyze();
    for (let r = 0; r < this.roots.length; r++) this.emitRoot(r);
    const builtinLines: string[] = [];
    for (const [expr, k, p] of this.builtinPatches) {
      const target = expr + this.keyMember(k);
      if (p === null) builtinLines.push(`delete ${target};`);
      else if (!(p.flags & P_ACC) && p.flags === P_ALL) builtinLines.push(`${target} = ${this.expr(p.value, null, null)};`);
      else builtinLines.push(`Object.defineProperty(${expr}, ${this.keyExpr(k)}, ${this.descriptor(p)});`);
    }
    if (this.refusals.length > 0) throw new DumpRefused(this.refusals);
    const count = this.roots.length;
    let s = `${HEADER}: ${count} binding${count === 1 ? "" : "s"} (${this.roots.map((r) => r.name).join(", ")}).\n` +
      "// Run it as a Script to recreate them (djs REPL: .restore FILE). Builtins are\n// referenced by name, never written";
    if (this.hostNames.length > 0) s += `; the host's ${this.hostNames.join(", ")} are re-bound by name`;
    s += ".\n";
    if (this.ns) {
      s += "globalThis.$dump = {};\n";
      for (const [name, text] of Object.entries(HELPERS)) if (this.helpers.has(name)) s += text;
      if (this.helpers.has("builtin")) s += LOADER;
    }
    for (const l of this.out) s += l + "\n";
    if (builtinLines.length > 0) s += "// Builtins the session changed.\n" + builtinLines.map((l) => l + "\n").join("");
    if (this.patches.length > 0 || this.finals.length > 0) {
      s += "// Links that close cycles, and attributes.\n";
      for (const l of this.patches) s += l + "\n";
      for (const l of this.finals) s += l + "\n";
    }
    if (this.ns) s += "void delete globalThis.$dump;\n";
    return s;
  }

  fresh(prefix: string): string {
    this.ns = true;
    return `$dump.${prefix}${++this.next}`;
  }

  literalKey(p: Prop): string {
    if (p.name === null) return `[${this.expr(mk(T.SYMBOL, p.sym), null, null)}]`;
    if (p.name === "__proto__") return `[${quote(p.name)}]`;
    return isIdent(p.name) || p.index >= 0 ? p.name : quote(p.name);
  }

  descriptor(p: Prop): string {
    const f = p.flags;
    if (f & P_ACC) {
      const g = this.expr(p.getter, null, null);
      return `{ get: ${g}, set: ${this.expr(p.setter, null, null)}, enumerable: ${!!(f & P_E)}, configurable: ${!!(f & P_C)} }`;
    }
    return `{ value: ${this.expr(p.value, null, null)}, writable: ${!!(f & P_W)}, enumerable: ${!!(f & P_E)}, configurable: ${!!(f & P_C)} }`;
  }

  emitRoot(r: number): void {
    const root = this.roots[r];
    if (root.state !== NEW) return;
    root.state = BUSY;
    const n = this.nodeOf(root.value);
    let line: string;
    if (n >= 0 && this.nodes[n].root === r && this.nodes[n].state === NEW) {
      this.nodes[n].loc = root.name;
      const src = this.freeClosure(n) ? this.h.source(payOf(this.nodes[n].v)) : null;
      if (src !== null && declaredName(this.fnKind(n), src) === root.name) {
        this.nodes[n].state = DONE;
        this.fnPatches(n, root.name);
        line = src;
      } else line = `${root.decl} ${root.name} = ${this.render(n, root.name, root.name)};`;
    } else {
      const e = this.expr(root.value, null, root.name);
      if (e === root.name) {
        root.state = DONE;
        return;
      }
      line = `${root.decl} ${root.name} = ${e};`;
    }
    this.out.push(line);
    root.state = DONE;
  }

  nodeOf(v: Val): number {
    return isRef(v) ? this.ids.get(payOf(v)) ?? -1 : -1;
  }
  isFunction(n: number): boolean {
    const v = this.nodes[n].v;
    return tagOf(v) === T.OBJECT && this.h.cls(payOf(v)) === CL.Function;
  }
  freeClosure(n: number): boolean {
    return this.nodes[n].group < 0 && this.isFunction(n);
  }
  fnKind(n: number): number {
    return this.h.u16(this.h.elements(payOf(this.nodes[n].v)) + C.FuncTemplate.fields.FLAGS.offset) & 15;
  }
  fnSource(n: number): string {
    return this.h.source(payOf(this.nodes[n].v)) ?? "";
  }

  expr(v: Val, loc: string | null, ctx: string | null): string {
    if (v instanceof Num) return Number.isNaN(v.n) ? "NaN" : num(v.n);
    switch (tagOf(v)) {
      case T.UNDEFINED: return "undefined";
      case T.NULL: return "null";
      case T.BOOL: return payOf(v) !== 0 ? "true" : "false";
      case T.INT: return String(payOf(v) | 0);
      case T.STRING: return quote(this.h.str(payOf(v)));
      case T.BIGINT: return `${bigint(this.h, payOf(v))}n`;
      case T.HOLE: return "";
    }
    const c = payOf(v);
    const b = this.builtins.get(c);
    if (b !== undefined) return this.builtinExpr(b);
    const he = this.host.get(c);
    if (he !== undefined) return he;
    if (c === this.global) return "globalThis";
    const n = this.ids.get(c);
    if (n === undefined) return "undefined";
    const node = this.nodes[n];
    if (node.state === DONE) return node.loc;
    if (node.state === BUSY) return "";
    if (node.root >= 0 && this.roots[node.root].state === NEW) {
      this.emitRoot(node.root);
      return node.loc;
    }
    if (node.group >= 0) {
      this.emitGroup(node.group);
      return node.loc;
    }
    const inline = node.refs === 1 && !node.cyclic && this.depth < 64;
    if (inline && (loc !== null || this.simple(n, ctx))) {
      this.depth++;
      const e = this.render(n, loc, ctx);
      this.depth--;
      return e;
    }
    const name = this.fresh("n");
    const e = this.render(n, name, null);
    this.out.push(`${name} = ${e};`);
    return name;
  }

  simple(n: number, ctx: string | null): boolean {
    const h = this.h;
    const v = this.nodes[n].v;
    if (tagOf(v) !== T.OBJECT) return false;
    const o = payOf(v);
    if (!h.extensible(o)) return false;
    const cls = h.cls(o);
    const props = h.props(o).props;
    switch (cls) {
      case CL.Ordinary:
      case CL.Array:
        return props.every((p) => {
          if (p.flags & P_ACC) return false;
          if (!(p.flags === P_ALL || (cls === CL.Array && p.name === "length"))) return false;
          const c = this.nodeOf(p.value);
          if (c < 0) return true;
          const d = this.nodes[c];
          return d.state === DONE || (d.refs === 1 && !d.cyclic && this.simple(c, p.name));
        }) && h.proto(o) !== 0 && this.defaultProto(o, h.proto(o));
      case CL.Function:
        return this.fnNameMatches(n, ctx) && props.every((p) => p.name !== null && this.defaultFnKey(o, p.name));
      case CL.ArrayBuffer:
      case CL.Date:
      case CL.RegExp:
      case CL.TypedArray:
      case CL.DataView:
      case CL.Boolean:
      case CL.Number:
        return props.every((p) => p.index >= 0 || (p.name === "lastIndex" && p.value === mk(T.INT, 0))) &&
          this.defaultProto(o, h.proto(o));
      default:
        return false;
    }
  }

  fnName(n: number): string {
    const p = this.h.props(payOf(this.nodes[n].v)).props.find((p) => p.name === "name");
    if (p === undefined) return "";
    const v = propValue(this.h, p);
    return typeof v === "string" ? v : tagOf(v) === T.STRING ? this.h.str(payOf(v)) : "";
  }

  fnNameMatches(n: number, ctx: string | null): boolean {
    return !anonymousForm(this.fnKind(n), this.fnSource(n)) || this.fnName(n) === (ctx ?? "");
  }

  fnPatches(n: number, ctx: string | null): void {
    const o = payOf(this.nodes[n].v);
    const loc = this.nodes[n].loc;
    if (!this.fnNameMatches(n, ctx)) this.patches.push(`Object.defineProperty(${loc}, "name", { value: ${quote(this.fnName(n))} });`);
    for (const p of this.h.props(o).props) {
      if (p.name !== null && this.defaultFnKey(o, p.name)) continue;
      this.propPatch(loc, p);
    }
    if (!this.h.extensible(o)) this.finals.push(`Object.preventExtensions(${loc});`);
  }

  propPatch(loc: string, p: Prop): void {
    if (!(p.flags & P_ACC) && p.flags === P_ALL) {
      const target = loc + this.keyMember(p);
      this.patches.push(`${target} = ${this.expr(p.value, target, null)};`);
    } else this.patches.push(`Object.defineProperty(${loc}, ${this.keyExpr(p)}, ${this.descriptor(p)});`);
  }

  emitGroup(g: number): void {
    const group = this.groups[g];
    if (group.state !== NEW) return;
    group.state = BUSY;
    const loc = group.loc = this.fresh("s");
    const h = this.h;
    const { closures, boxes } = group;
    const boxValue = (b: number): Val => h.val(payOf(this.nodes[b].v) + C.BoxCell.fields.VALUE.offset);
    const boxNames: string[] = [];
    const taken = new Set<string>();
    for (const b of boxes) {
      const p = this.nodes[b].path;
      const name = p.slice(p.lastIndexOf("~") + 1);
      if (taken.has(name)) this.refuse(p, "two captured variables of the same name in one scope group");
      taken.add(name);
      boxNames.push(name);
    }
    const home = new Map<number, number>();
    boxes.forEach((b, bi) => {
      const c = this.nodeOf(boxValue(b));
      if (c >= 0 && closures.includes(c) && !home.has(c)) home.set(c, bi);
    });
    const keys = new Map<number, string>();
    for (const c of closures) {
      let key: string;
      const bi = home.get(c);
      if (bi !== undefined) key = boxNames[bi];
      else {
        const f = this.fnName(c);
        key = isIdent(f) && !taken.has(f) ? f : `$f${c}`;
      }
      taken.add(key);
      this.nodes[c].loc = `${loc}.${key}`;
      this.nodes[c].state = BUSY;
      keys.set(c, key);
    }
    for (const b of boxes) this.nodes[b].state = BUSY;
    let text = `// ${[...keys.values()].join(", ")} sharing ${boxNames.join(", ")}\n${loc} = (() => {\n`;
    const sets: [number, Val][] = [];
    boxes.forEach((b, bi) => {
      const name = boxNames[bi];
      const inner = boxValue(b);
      const c = this.nodeOf(inner);
      let line: string;
      if (c < 0 && this.expr(inner, null, name) === name) return; // the host value of the same name
      if (c >= 0 && home.get(c) === bi) {
        const src = this.fnSource(c);
        line = declaredName(this.fnKind(c), src) === name ? src : `let ${name} = ${this.fnExpr(c, src)};`;
      } else if (c >= 0 && this.nodes[c].state === BUSY) {
        sets.push([bi, inner]);
        line = `let ${name};`;
      } else line = `let ${name} = ${this.expr(inner, null, name)};`;
      for (const l of line.split("\n")) text += `  ${l}\n`;
    });
    const ret: string[] = [];
    for (const c of closures) {
      const key = keys.get(c)!;
      if (home.has(c)) ret.push(key);
      else {
        const src = this.fnSource(c);
        ret.push(this.methodText(c, key, K_METHOD) ? src : `${key}: ${this.fnExpr(c, src)}`);
      }
    }
    if (sets.length > 0) ret.push(`$set(v) { ({ ${sets.map(([bi]) => boxNames[bi]).join(", ")} } = v); }`);
    text += `  return { ${ret.join(", ")} };\n})();`;
    this.out.push(text);
    for (const b of boxes) this.nodes[b].state = DONE;
    for (const c of closures) {
      this.nodes[c].state = DONE;
      this.fnPatches(c, keys.get(c)!);
    }
    group.state = DONE;
    if (sets.length > 0) {
      this.patches.push(`${loc}.$set({ ${sets.map(([bi, v]) => `${boxNames[bi]}: ${this.exprOrLoc(v)}`).join(", ")} });`);
    }
  }

  fnExpr(n: number, src: string): string {
    const k = this.fnKind(n);
    if (k === K_METHOD || k === K_GETTER || k === K_SETTER) {
      this.helpers.add("fn");
      this.ns = true;
      return `$dump.fn({ ${src} })`;
    }
    return src;
  }

  render(n: number, loc: string | null, ctx: string | null): string {
    const node = this.nodes[n];
    node.state = BUSY;
    node.loc = loc ?? "";
    const h = this.h;
    let e: string;
    if (tagOf(node.v) === T.SYMBOL) {
      const s = payOf(node.v);
      const reg = this.registered(s);
      const d = h.val(s + C.Symbol.fields.DESCRIPTION.offset);
      e = reg !== null ? `Symbol.for(${quote(reg)})` : tagOf(d) === T.STRING ? `Symbol(${quote(h.str(payOf(d)))})` : "Symbol()";
    } else e = this.renderObject(n, payOf(node.v), node.loc, ctx);
    node.state = DONE;
    return e;
  }

  registered(s: number): string | null {
    const reg = payOf(this.h.intrinsic(INTRINSIC.SymbolRegistry));
    for (const p of this.h.props(reg).props) if (p.value === mk(T.SYMBOL, s)) return p.name;
    return null;
  }

  renderObject(n: number, o: number, loc: string, ctx: string | null): string {
    const h = this.h;
    const cls = h.cls(o);
    let e: string;
    switch (cls) {
      case CL.Ordinary: return this.literal(o, loc);
      case CL.Array: return this.array(o, loc);
      case CL.Function: {
        const r = this.fnExpr(n, this.fnSource(n));
        this.fnPatches(n, ctx);
        return r;
      }
      case CL.Map:
      case CL.Set: e = this.collection(o, loc); break;
      case CL.Date: {
        const t = h.slot(o, 0);
        e = `new Date(${t instanceof Num ? (Number.isNaN(t.n) ? "NaN" : num(t.n)) : payOf(t) | 0})`;
        break;
      }
      case CL.Boolean:
      case CL.Number:
      case CL.String: e = `Object(${this.expr(h.slot(o, 0), null, null)})`; break;
      case CL.RegExp: {
        const src = h.slot(o, 1);
        const bits = num0(h.slot(o, 2));
        let f = "";
        for (let i = 0; i < 8; i++) if (bits & (1 << i)) f += RE_FLAGS[i];
        const s = tagOf(src) === T.STRING ? h.str(payOf(src)) : "";
        e = s !== "" && !/[/\n\r]/.test(s) ? `/${s}/${f}` : `new RegExp(${quote(s)}, ${quote(f)})`;
        break;
      }
      case CL.Error: {
        const p = h.proto(o);
        const ctor = (this.builtins.get(p) ?? "").replace(/\.prototype$/, "");
        const msg = h.props(o).props.find((q) => q.name === "message");
        e = `new ${isIdent(ctor) ? ctor : "Error"}(${msg ? this.expr(msg.value, null, null) : ""})`;
        if (!(p !== 0 && p < this.baseEnd)) this.patches.push(`Object.setPrototypeOf(${loc}, ${this.expr(p ? mk(T.OBJECT, p) : mk(T.NULL, 0), null, null)});`);
        break;
      }
      case CL.Promise: {
        const r = this.expr(h.slot(o, 1), null, null);
        if (num0(h.slot(o, 0)) === PROMISE_FULFILLED) e = `Promise.resolve(${r})`;
        else {
          this.helpers.add("rejected");
          this.ns = true;
          e = `$dump.rejected(${r})`;
        }
        break;
      }
      case CL.BoundFunction: {
        const target = this.expr(h.slot(o, 0), null, null);
        const args = [this.expr(h.slot(o, 1), null, null), ...boundArgs(h, o).map((a) => this.expr(a, null, null))];
        if (target === "" || args.includes("")) this.refuse(this.nodes[n].path, "a bound function on a cycle");
        e = `${target}.bind(${args.join(", ")})`;
        break;
      }
      case CL.ArrayBuffer: {
        this.helpers.add("bytes");
        this.ns = true;
        e = `$dump.bytes("${base64(bufferBytes(h, o))}")`;
        break;
      }
      case CL.TypedArray:
      case CL.DataView: return this.view(n, o, loc);
      default: e = "";
    }
    this.extra(o, loc);
    return e;
  }

  extra(o: number, loc: string): void {
    const h = this.h;
    const cls = h.cls(o);
    const skip = (p: Prop): boolean => {
      switch (cls) {
        case CL.RegExp: return p.name === "lastIndex";
        case CL.Error: return p.name === "message" || p.name === "stack";
        case CL.String: return p.name === "length" || p.index >= 0;
        case CL.BoundFunction: return p.name === "name" || p.name === "length";
        case CL.TypedArray: return p.index >= 0;
        default: return false;
      }
    };
    this.otherProps(o, loc, skip);
    if (cls === CL.RegExp) {
      const li = h.props(o).props.find((p) => p.name === "lastIndex");
      if (li && li.value !== mk(T.INT, 0)) this.patches.push(`${loc}.lastIndex = ${this.expr(li.value, null, null)};`);
    }
    if (cls !== CL.Error && cls !== CL.TypedArray && cls !== CL.DataView && !this.defaultProto(o, h.proto(o))) {
      const p = h.proto(o);
      this.patches.push(`Object.setPrototypeOf(${loc}, ${this.exprOrLoc(p ? mk(T.OBJECT, p) : mk(T.NULL, 0))});`);
    }
    if (!h.extensible(o)) this.finals.push(this.freezeLine(o, loc));
  }

  array(o: number, loc: string): string {
    const h = this.h;
    const len = num0(h.slot(o, 0));
    const el = h.elements(o);
    const dense = el ? h.vaLen(el) : 0;
    const items: string[] = [];
    for (let i = 0; i < len; i++) {
      const v = i < dense ? h.vaGet(el, i) : mk(T.HOLE, 0);
      const at = `${loc}[${i}]`;
      const e = this.expr(v, at, null);
      if (e === "" && tagOf(v) !== T.HOLE) {
        this.patches.push(`${at} = ${this.exprOrLoc(v)};`);
        items.push("undefined");
      } else items.push(e);
    }
    if (len > 0 && (len > dense || tagOf(h.vaGet(el, len - 1)) === T.HOLE)) items.push("");
    this.otherProps(o, loc, (p) => p.name === "length" || (p.index >= 0 && p.index < dense));
    if (!h.extensible(o)) this.finals.push(this.freezeLine(o, loc));
    return `[${items.join(", ")}]`;
  }

  collection(o: number, loc: string): string {
    const isMap = this.h.cls(o) === CL.Map;
    const es = entries(this.h, o);
    const items: string[] = [];
    let later = false;
    for (const [k, v] of es) {
      const ke = this.expr(k, null, null);
      if (isMap) {
        const ve = this.expr(v, null, null);
        later ||= ke === "" || ve === "";
        items.push(`[${ke}, ${ve}]`);
      } else {
        later ||= ke === "";
        items.push(ke);
      }
    }
    const ctor = isMap ? "new Map(" : "new Set(";
    if (later) {
      for (const [k, v] of es) {
        this.patches.push(isMap ? `${loc}.set(${this.exprOrLoc(k)}, ${this.exprOrLoc(v)});` : `${loc}.add(${this.exprOrLoc(k)});`);
      }
      return ctor + ")";
    }
    return items.length === 0 ? ctor + ")" : `${ctor}[${items.join(", ")}])`;
  }

  view(n: number, o: number, loc: string): string {
    const h = this.h;
    const ta = h.cls(o) === CL.TypedArray;
    const b = h.slot(o, 0);
    const kind = ta ? num0(h.slot(o, 1)) : 0;
    const [name, size, off, len] = ta
      ? [TA_NAMES[kind], TA_SIZES[kind], num0(h.slot(o, 2)), numOr(h.slot(o, 3), -1)]
      : ["DataView", 1, num0(h.slot(o, 1)), numOr(h.slot(o, 2), -1)];
    const bytes = bufferBytes(h, payOf(b));
    const whole = off === 0 && len >= 0 && bytes.length === len * size;
    const bn = this.nodeOf(b);
    let e: string;
    if (whole && ta && kind < 10 && len <= 64 && bn >= 0 && this.nodes[bn].refs === 1 && this.nodes[bn].state === NEW &&
      this.simple(bn, null) && this.simple(n, null)) {
      this.nodes[bn].state = DONE;
      this.nodes[bn].loc = `${loc}.buffer`;
      const Ctor = (globalThis as unknown as Record<string, new (b: ArrayBuffer) => ArrayLike<number>>)[name];
      const items = Array.from(new Ctor(bytes.slice().buffer), (x) => num(x));
      return `new ${name}([${items.join(", ")}])`;
    }
    const buf = this.expr(b, null, null);
    e = whole ? `new ${name}(${buf})` : len < 0 ? `new ${name}(${buf}, ${off})` : `new ${name}(${buf}, ${off}, ${len})`;
    this.extra(o, loc);
    return e;
  }

  exprOrLoc(v: Val): string {
    const e = this.expr(v, null, null);
    if (e !== "") return e;
    const n = this.nodeOf(v);
    return n >= 0 ? this.nodes[n].loc : "";
  }

  freezeLevel(o: number): number {
    const props = this.h.props(o).props;
    if (props.every((p) => !(p.flags & P_C) && (p.flags & P_ACC || !(p.flags & P_W)))) return 2;
    if (props.every((p) => !(p.flags & P_C))) return 1;
    return 0;
  }

  freezeLine(o: number, loc: string): string {
    return `${["Object.preventExtensions", "Object.seal", "Object.freeze"][this.freezeLevel(o)]}(${loc});`;
  }

  otherProps(o: number, loc: string, skip: (p: Prop) => boolean): void {
    for (const p of this.h.props(o).props) {
      if (p.name !== null && skip(p)) continue;
      this.propPatch(loc, p);
    }
  }

  literal(o: number, loc: string): string {
    const h = this.h;
    const items: string[] = [];
    const pr = h.proto(o);
    if (!this.defaultProto(o, pr)) {
      if (pr === 0) items.push("__proto__: null");
      else {
        const e = this.expr(mk(T.OBJECT, pr), null, null);
        if (e === "") this.patches.push(`Object.setPrototypeOf(${loc}, ${this.exprOrLoc(mk(T.OBJECT, pr))});`);
        else items.push(`__proto__: ${e}`);
      }
    }
    const freeze = h.extensible(o) ? 0 : this.freezeLevel(o);
    for (const p of h.props(o).props) {
      const key = this.literalKey(p);
      if (!(p.flags & P_ACC)) {
        // A function under a symbol key is named `[description]`.
        const name = p.name ?? `[${this.symbolDescription(p.sym)}]`;
        const at = loc + this.keyMember(p);
        const c = this.nodeOf(p.value);
        if (p.flags === P_ALL && c >= 0 && this.inlineMethod(c, name, K_METHOD)) {
          this.nodes[c].loc = at;
          this.nodes[c].state = DONE;
          this.fnPatches(c, name);
          items.push(this.fnSource(c));
          continue;
        }
        const e = this.expr(p.value, at, name);
        if (e === "") {
          this.patches.push(`${at} = ${this.exprOrLoc(p.value)};`);
          items.push(`${key}: undefined`);
        } else items.push(e === key ? e : `${key}: ${e}`);
        const implied = freeze === 2 ? p.flags === P_E : freeze === 1 ? p.flags === (P_W | P_E) : false;
        if (p.flags !== P_ALL && !implied) {
          this.patches.push(`Object.defineProperty(${loc}, ${this.keyExpr(p)}, { writable: ${!!(p.flags & P_W)}, enumerable: ${!!(p.flags & P_E)}, configurable: ${!!(p.flags & P_C)} });`);
        }
      } else {
        const name = p.name ?? "";
        const gc = this.nodeOf(p.getter), sc = this.nodeOf(p.setter);
        const g = gc >= 0 && this.inlineMethod(gc, name, K_GETTER) ? gc : -1;
        const s = sc >= 0 && this.inlineMethod(sc, name, K_SETTER) ? sc : -1;
        const fits = p.flags & P_E && p.flags & P_C && (tagOf(p.getter) === T.UNDEFINED || g >= 0) && (tagOf(p.setter) === T.UNDEFINED || s >= 0);
        if (fits) {
          const ke = this.keyExpr(p);
          for (const c of [g, s]) {
            if (c < 0) continue;
            this.nodes[c].loc = `Object.getOwnPropertyDescriptor(${loc}, ${ke}).${c === g ? "get" : "set"}`;
            this.nodes[c].state = DONE;
            items.push(this.fnSource(c));
          }
        } else {
          items.push(`${key}: undefined`);
          this.patches.push(`Object.defineProperty(${loc}, ${this.keyExpr(p)}, ${this.descriptor(p)});`);
        }
      }
    }
    if (!h.extensible(o)) this.finals.push(this.freezeLine(o, loc));
    return items.length === 0 ? "{}" : `{ ${items.join(", ")} }`;
  }

  inlineMethod(c: number, key: string, kind: number): boolean {
    const d = this.nodes[c];
    return this.freeClosure(c) && d.refs === 1 && d.state === NEW && d.root < 0 && this.methodText(c, key, kind);
  }

  methodText(c: number, key: string, kind: number): boolean {
    if (!this.isFunction(c) || this.fnKind(c) !== kind || !isIdent(key)) return false;
    let s = this.fnSource(c);
    if (kind === K_GETTER || kind === K_SETTER) {
      if (!s.startsWith(kind === K_GETTER ? "get" : "set")) return false;
      s = s.slice(3);
    } else if (/^async\s/.test(s)) s = s.slice(5);
    s = s.trimStart();
    if (s.startsWith("*")) s = s.slice(1).trimStart();
    return s.startsWith(key) && s.slice(key.length).trimStart().startsWith("(");
  }
}

function samePropBits(p: Prop, bp: Prop, h?: Heap, b?: Heap): boolean {
  if (p.flags !== bp.flags) return false;
  const same = (x: Val, y: Val): boolean => {
    if (x instanceof Num || y instanceof Num) return x instanceof Num && y instanceof Num && Object.is(x.n, y.n);
    if (x === y) return true;
    return h !== undefined && b !== undefined && tagOf(x) === T.STRING && tagOf(y) === T.STRING && h.str(payOf(x)) === b.str(payOf(y));
  };
  const lp = (p as Prop & { lazyName?: string }).lazyName, lbp = (bp as Prop & { lazyName?: string }).lazyName;
  if (lp !== undefined || lbp !== undefined) return lp === lbp;
  return p.flags & P_ACC ? same(p.getter, bp.getter) && same(p.setter, bp.setter) : same(p.value, bp.value);
}

const num0 = (v: Val): number => numOr(v, 0);
function numOr(v: Val, d: number): number {
  if (v instanceof Num) return v.n;
  return tagOf(v) === T.INT ? payOf(v) | 0 : d;
}

function entries(h: Heap, o: number): [Val, Val][] {
  const e = payOf(h.slot(o, 0));
  const n = (h.vaLen(e) - 2) / 2;
  const out: [Val, Val][] = [];
  for (let i = 0; i < n; i++) {
    const k = h.vaGet(e, 2 + 2 * i);
    if (tagOf(k) !== T.HOLE) out.push([k, h.vaGet(e, 3 + 2 * i)]);
  }
  return out;
}

function boundArgs(h: Heap, o: number): Val[] {
  const a = h.slot(o, 2);
  if (tagOf(a) !== T.OBJECT) return [];
  const arr = payOf(a);
  const el = h.elements(arr);
  const len = num0(h.slot(arr, 0));
  return Array.from({ length: len }, (_, i) => (el ? h.vaGet(el, i) : mk(T.UNDEFINED, 0)));
}

function bufferBytes(h: Heap, buf: number): Uint8Array {
  const s = h.slot(buf, 0);
  if (tagOf(s) === T.UNDEFINED) return new Uint8Array(0);
  const cell = payOf(s);
  const n = h.u32(cell + C.Bytes.fields.LEN.offset);
  return h.mem.subarray(cell + C.Bytes.tail!.offset, cell + C.Bytes.tail!.offset + n);
}

function base64(bytes: Uint8Array): string {
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x2000) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x2000) as unknown as number[]);
  return btoa(s);
}

function bigint(h: Heap, off: number): bigint {
  const len = h.u32(off + C.BigInt.fields.LEN.offset);
  let v = 0n;
  for (let i = len - 1; i >= 0; i--) v = (v << 32n) | BigInt(h.u32(off + C.BigInt.tail!.offset + 4 * i));
  return h.u16(off + HDR_FLAGS) & LAYOUT.flags.BIGINT_NEG ? -v : v;
}
