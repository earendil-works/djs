// Compares the Rust dumper (Sandbox.dump, djs-wasm feature `dump`) with the TypeScript
// prototype (js/proto/dump.ts) on the cases of crates/djs-core/src/vm/dump_tests.rs and a
// large heap (0167). Usage:
//   DJS_WASM=target/.../djs_wasm.wasm node js/proto/compare.ts [--big N]

import { loadModule, Sandbox } from "../src/sandbox.ts";
import { baseReference, type BaseExports } from "../src/snapshot.ts";
import { dumpHeap, DumpRefused, HEADER } from "./dump.ts";

interface Case {
  name: string;
  setup: string;
  names?: string[];
  /** A body returning what both VMs must agree on. */
  check?: string;
  /** Refusals that must be listed. */
  refused?: string[];
}

const CASES: Case[] = [
  {
    name: "shared closures",
    setup: "function makeCounter() { let n = 0; return { inc: () => ++n, get: () => n }; }\nconst c = makeCounter(); c.inc(); c.inc();",
    names: ["c"],
    check: "c.inc(); return [c.get(), c.inc(), c.get(), c.inc.name, c.get.name].join()",
  },
  {
    name: "globals functions read",
    setup: "let calls = 0;\nfunction fmt(x) { calls++; return '<' + x + '>'; }\nconst lib = { fmt, twice(x) { return fmt(x) + fmt(x); }, tag: (s) => lib.fmt(s) };\nlib.twice(1);",
    names: ["lib"],
    check: "return [lib.twice('a'), lib.tag('b'), calls, lib.fmt === fmt, lib.tag.name].join()",
  },
  {
    name: "data identity and kinds",
    setup: "const a = { x: 1 };\nconst data = { a, b: a, list: [1, , 3], m: new Map([[a, 'A'], ['k', [a]]]), s: new Set([1, 'two']),\n" +
      "  d: new Date(86400000), r: /a+b/gi, z: -0, n: NaN, u: undefined, big: 2n ** 70n, str: 'q\"\\\\\\n\\u2028' };\n" +
      "data.self = data;\nObject.defineProperty(data, 'hidden', { value: 42, enumerable: false });\nconst frozen = Object.freeze({ k: [1] });",
    check: "return [data.b === data.a, data.self === data, data.m.get(data.a), data.m.get('k')[0] === a,\n" +
      "  [...data.s].join('|'), data.d.toISOString(), String(data.r), data.r.flags, Object.is(data.z, -0),\n" +
      "  Number.isNaN(data.n), 'u' in data, data.u, String(data.big), JSON.stringify(data.str), 1 in data.list,\n" +
      "  data.list.length, data.hidden, Object.keys(data).join(), Object.isFrozen(frozen), frozen.k[0]].join()",
  },
  {
    name: "cycles through closures",
    setup: "function mk() { const state = { items: [] };\n const api = { add(x) { state.items.push(x); return api; }, all: () => state.items };\n" +
      " state.owner = api; return api; }\nconst store = mk(); store.add(1).add(2);",
    check: "store.add(3); return [store.all().join(), store.add(4) === store, store.all().length].join()",
  },
  {
    name: "binary buffers",
    setup: "const bytes = new Uint8Array([1, 2, 3, 250]);\nconst view = new DataView(bytes.buffer, 1, 2);\nconst floats = new Float64Array([1.5, -2]);\n" +
      "const sub = bytes.subarray(2);\nconst mk = () => { const buf = new Uint8Array(4); let i = 0; return { push(b) { buf[i++] = b; return buf.join(); } }; };\n" +
      "const w = mk(); w.push(7);",
    check: "sub[0] = 9; return [bytes.join(), view.getUint16(0), floats.join(), w.push(8), view.buffer === bytes.buffer].join()",
  },
  {
    name: "builtins by name",
    setup: "const m = Math.max; const iterProto = Object.getPrototypeOf([][Symbol.iterator]());\n" +
      "const sizeGetter = Object.getOwnPropertyDescriptor(Map.prototype, 'size').get;\n" +
      "const tag = Symbol('tag'); const reg = Symbol.for('app');\nconst o = { [tag]: 1, [reg]: 2, [Symbol.iterator]: function* () { yield 1; } };",
    check: "return [m === Math.max, iterProto === Object.getPrototypeOf([].values()),\n" +
      "  sizeGetter.call(new Map([[1, 1]])), o[tag], o[Symbol.for('app')], [...o].join(), tag.description].join()",
  },
  {
    name: "patched builtins",
    setup: "Array.prototype.sum = function () { return this.reduce((a, b) => a + b, 0); };\ndelete Math.hypot; const x = 1;",
    check: "return [[1, 2, 3].sum(), typeof Math.hypot, x].join()",
  },
  {
    name: "symbol keys with attributes",
    setup: "const o = { visible: 1 };\nObject.defineProperty(o, Symbol('hidden'), { value: 2, enumerable: false });",
    check: "const ks = Object.getOwnPropertySymbols(o);\nreturn [ks.length, ks[0].description, o[ks[0]], o.propertyIsEnumerable(ks[0])].join()",
  },
  {
    name: "refusals",
    setup: "const p = new Promise(() => {});\nfunction* g() { yield 1; } const it = g(); it.next();\n" +
      "const px = { inner: new Proxy({}, {}) };\nclass A {} const a = new A();",
    refused: ["p: a pending promise", "it: a suspended generator", "px.inner: a Proxy", "A: a class"],
  },
];

/** A heap of `n` records, some shared, with closures over them and a buffer. */
function bigSetup(n: number): string {
  return `
const users = [];
for (let i = 0; i < ${n}; i++) users.push({ id: i, name: 'user' + i, tags: ['a', 'b' + (i % 7)], score: i * 1.5, meta: { created: new Date(i * 1000), active: i % 3 === 0 } });
const byId = new Map(users.map((u) => [u.id, u]));
const team = { lead: users[0], members: users.slice(0, 100) };
users[0].team = team;
function makeCounter(start) { let n = start; return { inc: () => ++n, get: () => n }; }
const counters = [];
for (let i = 0; i < ${Math.max(1, n / 10) | 0}; i++) counters.push(makeCounter(i));
const blob = new Uint8Array(${n * 4}).map((_, i) => i & 255);
const helpers = { total() { let s = 0; for (const u of users) s += u.score; return s; }, find: (id) => byId.get(id) };`;
}
const BIG_CHECK = "return [users.length, byId.get(5).name, team.lead.team === team, counters[3].get(), blob[1000], helpers.total(), helpers.find(7) === users[7]].join()";

const module = await loadModule();
const sandbox = (): Sandbox => new Sandbox({ module });
const baseOf = (s: Sandbox): Uint8Array => baseReference(module, (s as unknown as { x: BaseExports }).x).bytes;

function load(s: Sandbox, code: string): void {
  s.load(code.startsWith(HEADER) ? code : `${HEADER}\n${code}`);
}

function tsDump(s: Sandbox, names?: string[]): string {
  return dumpHeap(s.heapView(), { base: baseOf(s), host: ["console"], names });
}

function attempt(f: () => string): { text?: string; error?: string } {
  try {
    return { text: f() };
  } catch (e) {
    return { error: e instanceof DumpRefused ? e.message : String((e as Error).message ?? e) };
  }
}

function verify(text: string | undefined, check: string, want: unknown): string {
  if (text === undefined) return "refused";
  try {
    const b = sandbox();
    load(b, text);
    const got = b.run(check);
    return got === want ? "ok" : `differs: ${String(got)} vs ${String(want)}`;
  } catch (e) {
    return `load failed: ${(e as Error).message}`;
  }
}

const args = process.argv.slice(2);
let ok = true;
for (const c of CASES) {
  const a = sandbox();
  load(a, c.setup);
  const rust = attempt(() => a.dump(c.names ?? []));
  const ts = attempt(() => tsDump(a, c.names));
  if (c.refused) {
    const r = c.refused.map((w) => [rust.error?.includes(w) ?? false, ts.error?.includes(w) ?? false]);
    const line = (i: number): string => (r.every((x) => x[i]) ? "ok" : "missing refusals");
    console.log(`${c.name.padEnd(28)} rust: ${line(0)}  ts: ${line(1)}`);
    ok &&= r.every((x) => x[0]);
    continue;
  }
  const want = a.run(c.check!);
  const rv = verify(rust.text, c.check!, want);
  const tv = verify(ts.text, c.check!, want);
  console.log(`${c.name.padEnd(28)} rust: ${rv}  ts: ${tv}${rust.text === ts.text ? "  (same text)" : ""}`);
  if (tv !== "ok" && args.includes("--show")) console.log(ts.text ?? ts.error);
  ok &&= rv === "ok";
}

const bigAt = args.indexOf("--big");
if (bigAt >= 0) {
  const n = Number(args[bigAt + 1] ?? 10000);
  const a = sandbox();
  load(a, bigSetup(n));
  const want = a.run(BIG_CHECK);
  const time = (f: () => string): [number, string] => {
    f(); // warm up (TS: the builtin names of the base are computed once)
    const t0 = performance.now();
    const reps = 3;
    let s = "";
    for (let i = 0; i < reps; i++) s = f();
    return [(performance.now() - t0) / reps, s];
  };
  const [rt, rtext] = time(() => a.dump());
  const [tt, ttext] = time(() => tsDump(a));
  const t0 = performance.now();
  builtinNamesCold(a);
  const cold = performance.now() - t0;
  console.log(`big (${n} records): rust ${rt.toFixed(1)} ms, ${rtext.length} bytes: ${verify(rtext, BIG_CHECK, want)}`);
  console.log(`big (${n} records): ts   ${tt.toFixed(1)} ms, ${ttext.length} bytes: ${verify(ttext, BIG_CHECK, want)} (builtin names, once per base: ${cold.toFixed(1)} ms)`);
}

function builtinNamesCold(a: Sandbox): void {
  // An empty VM, and a fresh copy of the base to defeat the per-base cache.
  const e = sandbox();
  dumpHeap(e.heapView(), { base: baseOf(a).slice(), host: ["console"] });
}

process.exitCode = ok ? 0 : 1;
