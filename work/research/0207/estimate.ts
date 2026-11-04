// 0207: estimated savings of each idea on the measured dumps (README.md). Textual
// simulations where the idea is a rewrite of the text (exact raw and gzip deltas), and
// span counts where it needs the exporter (raw, and the gzip of the dump without those
// bytes as an upper bound).
//
//   node work/research/0207/estimate.ts SUITE=DIR ...

import { readFileSync, statSync } from "node:fs";
import { bodySpans, partition, envSpans, walk, gz, type Span } from "../../../scripts/dump-size/dump-size.ts";

type Est = { raw: number; gz: number };
const ideas = new Map<string, Est>();
const add = (k: string, raw: number, g: number) => {
  const e = ideas.get(k) ?? { raw: 0, gz: 0 };
  e.raw += raw;
  e.gz += g;
  ideas.set(k, e);
};

/** Raw and gzip saved by rewriting `t` to `u`. */
function delta(k: string, t: string, u: string, gt: number) {
  if (t === u) return;
  add(k, t.length - u.length, gt - gz(u));
}

/** Raw and gzip (upper bound) saved by dropping the spans `drop` from `t`. */
function dropSpans(k: string, t: string, drop: [number, number][], gt: number, extra = 0) {
  if (!drop.length) return;
  drop.sort((a, b) => a[0] - b[0]);
  let u = "";
  let at = 0;
  for (const [s, e] of drop) {
    if (s < at) continue;
    u += t.slice(at, s);
    at = e;
  }
  u += t.slice(at);
  add(k, t.length - u.length - extra, gt - gz(u));
}

const HELPER_MIN: Record<string, string> = {
  // The same functions, written compactly (what a minified helper block would cost).
  L: "L:(i,n)=>{let f=1;const r={next(){if(f){f=0;return{value:void 0,done:!1}}return Reflect.apply(n??i.next,i,[])},return(v){const m=i.return;return m==null?{done:!0,value:v}:Reflect.apply(m,i,[v])},[Symbol.iterator](){return r}};return r}",
  advance: "advance:(i,n)=>{const x=i.next;while(n--)Reflect.apply(x,i,[]);return i}",
};

/** Each continuation's text by the start of its head. */
function contBodies(t: string, spans: Span[]): Map<number, string> {
  const out = new Map<number, string>();
  let cur = -1;
  for (const [c, s, e] of spans) {
    if (c === "cont.head") {
      cur = s;
      out.set(cur, "");
    } else if (cur >= 0 && c.startsWith("cont.")) out.set(cur, out.get(cur)! + t.slice(s, e));
    else if (!c.startsWith("cont")) cur = -1;
  }
  return out;
}

let noEnv = false;

function estimate(t0: string) {
  let t = t0;
  if (noEnv) {
    const sp = envSpans(t0, partition(t0, bodySpans(t0, 0, t0.length)));
    t = sp.filter(([c]) => !c.startsWith("env")).map(([, s, e]) => t0.slice(s, e)).join("");
  }
  const gt = gz(t);
  const spans: Span[] = partition(t, bodySpans(t, 0, t.length));

  // E1: anonymous members written as `(0, fn)` instead of a name fixup afterwards.
  const nameFix = /^Object\.defineProperty\([^;\n]*, "name", \{ value: "" \}\);\n/gm;
  const n1 = (t.match(nameFix) ?? []).length;
  if (n1) {
    const u = t.replace(nameFix, "");
    add("E1 anonymous members as (0, f), no name fixups", t.length - u.length - 5 * n1, gt - gz(u) - 2 * n1);
  }

  // E2: records as one helper call, defaults left out (at "await#1", empty cmap/scaf).
  const rec = /^Object\.defineProperty\(([^,]+), "\$djs", \{ value: \{ f: (.*?), at: ("[^"]*"), segs: (\[[^\]]*\]), scaf: (\[.*?\]), cmap: (\[.*?\]) \} \}\);$/gm;
  let any = false;
  const u2 = t.replace(rec, (_m, k, f, at, segs, scaf, cmap) => {
    any = true;
    const args = [k, f, segs];
    const tail = [scaf === "[]" ? "" : scaf, cmap === "[]" ? "" : cmap, at === '"await#1"' ? "" : at];
    while (tail.length && tail[tail.length - 1] === "") tail.pop();
    return `$dump.o(${[...args, ...tail.map((x) => x || "0")].join(", ")});`;
  });
  if (any) {
    const helper = 'o: (k, f, segs, scaf, cmap, at) => Object.defineProperty(k, "$djs", { value: { f, at: at || "await#1", segs, scaf: scaf || [], cmap: cmap || [] } }),\n  ';
    const u = u2.replace("globalThis.$dump = { ", "globalThis.$dump = { " + helper);
    delta("E2 records as $dump.o(K, f, segs, …) with defaults", t, u, gt);
  }

  // E2b: records without their default fields (at "await#1", empty segs/scaf/cmap).
  delta(
    "E2b records without default fields",
    t,
    t.replace(/^(Object\.defineProperty\([^,]+, "\$djs", .*)$/gm, (l) => l.replace(/, at: "await#1"/, "").replace(/, (segs|scaf|cmap): \[\]/g, "")),
    gt,
  );

  // E12: frame variables a continuation declares and never reads again (their lines; the
  // data only they reach is not counted).
  let e12 = 0;
  for (const [s0, body] of contBodies(t, spans)) {
    void s0;
    for (const m of body.matchAll(/^(?:var|const|let) ([\w$]+) = [^\n]*\n/gm)) {
      const rest = body.slice(m.index! + m[0].length);
      if (!new RegExp(`(?<![\\w$.])${m[1].replace(/\$/g, "\\$")}(?![\\w$])`).test(rest)) e12 += m[0].length;
    }
  }
  if (e12) add("E12 dead frame variables' declarations (raw only)", e12, 0);

  // E3: written origins (Fa) without their dead prefix: the text before the earliest
  // offset any seg or scaffold of their record refers to (an upper bound: the pruned origin
  // still declares the prefix's bindings).
  // E4: written origins not written at all (what the stretches copied into K cover).
  const drop3: [number, number][] = [];
  let e4 = 0;
  for (const m of t.matchAll(/^\$dump\.(Fa\d+) = /gm)) {
    const name = m[1];
    const sp = spans.find(([c, s]) => c === "origin" && s === m.index);
    if (!sp) continue;
    const fa = t.slice(sp[1] + m[0].length, sp[2]);
    const r = new RegExp(`f: \\$dump\\.${name}, at: "[^"]*", segs: \\[([^\\]]*)\\], scaf: (\\[.*?\\]), cmap`).exec(t);
    if (!r) continue;
    const segs = r[1] ? r[1].split(",").map(Number) : [];
    const offs: number[] = [];
    for (let i = 0; i + 2 < segs.length; i += 3) {
      offs.push(segs[i + 1]);
      e4 += segs[i + 2];
    }
    for (const s of r[2].matchAll(/\["[\w-]+",(\d+),/g)) offs.push(Number(s[1]));
    const head = /^[^{]*?(=>\s*\{|\{)/.exec(fa)?.[0].length ?? 0;
    const min = offs.length ? Math.min(...offs) : fa.length;
    if (min > head + 20) drop3.push([sp[1] + m[0].length + head, sp[1] + m[0].length + Math.min(min, fa.length - 3)]);
  }
  dropSpans("E3 origins without their dead prefix (upper bound)", t, drop3, gt);
  if (e4) add("E4 origin stretches already in K (if written once)", e4, 0);

  // E5: continuations of the same function and site written once (a shared continuation
  // taking the frame's values): conts equal after their literals and $dump refs are
  // replaced, and their records.
  const conts = spans.filter(([c]) => c === "cont.code" || c === "cont.scaf");
  const byStart = new Map<number, string>();
  // Group a continuation's lines by the cont.head before them.
  let cur = -1;
  for (const [c, s, e] of spans) {
    if (c === "cont.head") {
      cur = s;
      byStart.set(cur, "");
    } else if (cur >= 0 && c.startsWith("cont.")) byStart.set(cur, byStart.get(cur)! + t.slice(s, e));
    else if (!c.startsWith("cont")) cur = -1;
  }
  void conts;
  const norm = new Map<string, number[]>();
  for (const [s, body] of byStart) {
    const k = body.replace(/"(?:[^"\\]|\\.)*"/g, '""').replace(/\$dump\.[a-z]+\d+(_\d+)?/g, "$x").replace(/\b\d+\b/g, "0");
    norm.set(k, [...(norm.get(k) ?? []), body.length]);
  }
  let e5 = 0;
  for (const [, lens] of norm) if (lens.length > 1) e5 += lens.slice(1).reduce((a, b) => a + b, 0);
  // Their records: equal after the K name and offsets.
  const recs = new Map<string, number[]>();
  for (const m of t.matchAll(/^Object\.defineProperty\(\S+, "\$djs", (.*)$/gm)) {
    const k = m[1].replace(/\d+/g, "0");
    recs.set(k, [...(recs.get(k) ?? []), m[0].length]);
  }
  for (const [, lens] of recs) if (lens.length > 1) e5 += lens.slice(1).reduce((a, b) => a + b, 0);
  if (e5) add("E5 one continuation per (function, site) (raw only)", e5, 0);

  // E6: helpers written compactly.
  let u6 = t;
  for (const [k, min] of Object.entries(HELPER_MIN)) {
    const re = new RegExp(`\\n  ${k}: \\(it, n(ext)?\\) => \\{.*\\}( \\}|,)`, "m");
    u6 = u6.replace(re, (m0) => `\n  ${min}${m0.endsWith(" }") ? " }" : ","}`);
  }
  delta("E6 helpers written compactly", t, u6, gt);

  // E7: `$dump.` as `$d.` (a shorter root name).
  delta("E7 $dump as $d", t, t.replaceAll("$dump.", "$d.").replace("globalThis.$dump =", "globalThis.$d =").replace("delete globalThis.$dump", "delete globalThis.$d"), gt);

  // E8: `$djs_` scaffolding names as `$j` (t0, R, L…).
  delta("E8 $djs_ names as $j", t, t.replaceAll("$djs_", "$j"), gt);

  // E9: the final statement and header shorter: `try { X } finally { delete globalThis.$dump, delete globalThis.$djs }`.
  delta(
    "E9 shorter final statement",
    t,
    t.replace(/^try \{ \((.*)\); \} finally \{ delete globalThis\.\$dump; delete globalThis\.\$djs; \}$/m, "try { $1 } finally { delete globalThis.$dump, delete globalThis.$djs }"),
    gt,
  );

  // E10: exact duplicate function texts (closures of one function written per instance).
  const fns = new Map<string, number>();
  let e10 = 0;
  for (const [c, s, e] of spans) {
    if (c !== "fn") continue;
    const k = t.slice(s, e).replace(/^\s*\$f\d+:\s*/, "").trim();
    if (k.length < 40) continue;
    if (fns.has(k)) e10 += e - s;
    else fns.set(k, 1);
  }
  if (e10) add("E10 duplicate function texts shared (raw only)", e10, 0);

  // E11: long string literals written more than once.
  const strs = new Map<string, number>();
  let e11 = 0;
  for (const m of t.matchAll(/"(?:[^"\\\n]|\\.){40,}"/g)) {
    if (strs.has(m[0])) e11 += m[0].length - 8;
    else strs.set(m[0], 1);
  }
  if (e11) add("E11 repeated long strings shared (raw only)", e11, 0);

  return { raw: t.length, gz: gt };
}

const kb = (n: number) => (n / 1024).toFixed(1);
noEnv = process.argv.includes("--no-env");
for (const spec of process.argv.slice(2).filter((a) => a.includes("="))) {
  ideas.clear();
  const [name, dir] = spec.split("=");
  const files = statSync(dir).isDirectory() ? walk(dir) : [dir];
  let raw = 0;
  let g = 0;
  for (const f of files) {
    const r = estimate(readFileSync(f, "utf8"));
    raw += r.raw;
    g += r.gz;
  }
  console.log(`\n### ${name}: ${files.length} dumps, ${kb(raw)} KB raw, ${kb(g)} KB gzip\n`);
  console.log("| idea | raw KB saved | raw % | gzip KB saved | gzip % |\n|---|---:|---:|---:|---:|");
  for (const [k, e] of [...ideas].sort((a, b) => b[1].raw - a[1].raw)) {
    console.log(`| ${k} | ${kb(e.raw)} | ${((100 * e.raw) / raw).toFixed(1)} | ${e.gz ? kb(e.gz) : "–"} | ${e.gz ? ((100 * e.gz) / g).toFixed(1) : "–"} |`);
  }
}
