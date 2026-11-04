// dump-size (0207): where the bytes of plain dumps (format 3, D2830) go. Reads dumps
// written by the producers below, splits each one into categories by its structure (a
// small scanner of the exporter's output: top-level statements, scope factories, their
// members, deferred groups) and by the provenance of its lines, and reports raw and gzip
// bytes per category: totals, the marginal gzip cost (gzip of the dump minus gzip of the
// dump without the category), per-dump distributions and the largest dumps.
//
//   node scripts/dump-size/dump-size.ts [--out FILE.md] [--tsv FILE] SUITE=DIR[:SRCROOT] ...
//
// A suite is a directory of dumps (any depth, `*.js`). SRCROOT, when given, is where the
// dumped scripts' sources are: a dump at DIR/<case>/<k>.js has its source at
// SRCROOT/<case> (difftest's --plain-out layout), used for the provenance split.
// Producers:
//   corpus: target/debug/djs-difftest --engine djs --plain-out DIR    (SRCROOT tests/corpus)
//   v2:     DJS_PLAIN_OUT=DIR cargo test -p djs-core --lib plain_dumps_at_every_idle_point
//                                                                   (SRCROOT tests/dump/v2, .src.js)
//   sleep:  examples/sleep-dump/run.sh --restart 50 (examples/sleep-dump/out/)
//   pi:     work/research/0207/README.md says how (a two-line patch of js/pi/vm.ts)
//
//   node scripts/dump-size/dump-size.ts --show FILE    the spans of one dump
//   node scripts/dump-size/dump-size.ts --score tests/scores/dump-size.tsv [--update] corpus=DIR v2=DIR
//                                                       the gate's size score (D2901)

import { readdirSync, readFileSync, statSync, writeFileSync, existsSync } from "node:fs";
import { join, relative, dirname } from "node:path";
import { gzipSync } from "node:zlib";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");

// ---- Scanner -----------------------------------------------------------------------------

const KEYWORDS_BEFORE_REGEX = new Set([
  "return", "typeof", "instanceof", "in", "of", "new", "delete", "void", "throw", "case", "do", "else", "yield", "await",
]);

/**
 * Index just past the end of the token starting at `i` when it is a string, template,
 * comment or regex; otherwise -1. `prev` is the last significant token's text.
 */
function skipOpaque(t: string, i: number, prev: string): number {
  const c = t[i];
  if (c === '"' || c === "'") {
    let j = i + 1;
    while (j < t.length && t[j] !== c) j += t[j] === "\\" ? 2 : 1;
    return j + 1;
  }
  if (c === "/" && t[i + 1] === "/") {
    const j = t.indexOf("\n", i);
    return j < 0 ? t.length : j;
  }
  if (c === "/" && t[i + 1] === "*") {
    const j = t.indexOf("*/", i + 2);
    return j < 0 ? t.length : j + 2;
  }
  if (c === "/") {
    const regex = prev === "" || /[(,=:[!&|?{};+\-*%<>~^]$/.test(prev) || KEYWORDS_BEFORE_REGEX.has(prev);
    if (!regex) return -1;
    let j = i + 1;
    let cls = false;
    while (j < t.length && (cls || t[j] !== "/")) {
      if (t[j] === "\\") j++;
      else if (t[j] === "[") cls = true;
      else if (t[j] === "]") cls = false;
      else if (t[j] === "\n") return -1; // not a regex after all
      j++;
    }
    j++;
    while (j < t.length && /[a-z]/.test(t[j])) j++;
    return j;
  }
  return -1;
}

interface Tok {
  s: number;
  e: number;
  text: string;
}

/** Significant tokens of t[start, end), templates as one token each (their `${}` parts scanned for nesting only). */
function tokens(t: string, start: number, end: number): Tok[] {
  const out: Tok[] = [];
  let i = start;
  let prev = "";
  while (i < end) {
    const c = t[i];
    if (c === " " || c === "\n" || c === "\t" || c === "\r") {
      i++;
      continue;
    }
    if (c === "`") {
      const s = i;
      i = skipTemplate(t, i);
      out.push({ s, e: i, text: "`" });
      prev = "`";
      continue;
    }
    const j = skipOpaque(t, i, prev);
    if (j >= 0) {
      if (!(c === "/" && (t[i + 1] === "/" || t[i + 1] === "*"))) {
        out.push({ s: i, e: j, text: c === "/" ? "/re/" : '"' });
        prev = '"';
      }
      i = j;
      continue;
    }
    if (/[A-Za-z0-9_$]/.test(c)) {
      let k = i;
      while (k < end && /[A-Za-z0-9_$.]/.test(t[k])) k++;
      const w = t.slice(i, k);
      out.push({ s: i, e: k, text: w });
      prev = w;
      i = k;
      continue;
    }
    // Punctuation: `=>` as one token, others one char each.
    if (c === "=" && t[i + 1] === ">") {
      out.push({ s: i, e: i + 2, text: "=>" });
      prev = "=>";
      i += 2;
      continue;
    }
    out.push({ s: i, e: i + 1, text: c });
    prev = c;
    i++;
  }
  return out;
}

/** Index just past the template literal starting at `i`. */
function skipTemplate(t: string, i: number): number {
  let j = i + 1;
  while (j < t.length && t[j] !== "`") {
    if (t[j] === "\\") j += 2;
    else if (t[j] === "$" && t[j + 1] === "{") {
      // Scan the substitution to its matching brace.
      let depth = 1;
      j += 2;
      let prev = "{";
      while (j < t.length && depth > 0) {
        const c = t[j];
        if (c === "`") {
          j = skipTemplate(t, j);
          prev = "`";
          continue;
        }
        const k = skipOpaque(t, j, prev);
        if (k >= 0) {
          j = k;
          prev = '"';
          continue;
        }
        if (c === "{") depth++;
        else if (c === "}") depth--;
        if (!/\s/.test(c)) prev = c;
        j++;
      }
    } else j++;
  }
  return j + 1;
}

/** Opening brackets (a Set: `"__proto__" in {}` is true). */
const OPEN = new Set(["(", "[", "{"]);

/**
 * Splits t[start, end) into statements at bracket depth 0: a statement ends after `;`, or
 * after `}` that ends a line. Returns [s, e) ranges covering the input (whitespace
 * between statements attached to the following one).
 */
function statements(t: string, start: number, end: number): [number, number][] {
  const toks = tokens(t, start, end);
  const out: [number, number][] = [];
  let depth = 0;
  let s = start;
  for (const tok of toks) {
    if (OPEN.has(tok.text)) depth++;
    else if (tok.text === ")" || tok.text === "]" || tok.text === "}") depth--;
    if (depth !== 0) continue;
    const endsLine = tok.text === "}" && /^[ \t]*(\n|$)/.test(t.slice(tok.e, tok.e + 80));
    if (tok.text === ";" || endsLine) {
      let e = tok.e;
      if (t[e] === "\n") e++;
      out.push([s, e]);
      s = e;
    }
  }
  if (s < end) out.push([s, end]);
  return out;
}

/** Splits t[start, end) at depth-0 commas (object members, arguments). */
function commaSplit(t: string, start: number, end: number): [number, number][] {
  const toks = tokens(t, start, end);
  const out: [number, number][] = [];
  let depth = 0;
  let s = start;
  for (const tok of toks) {
    if (OPEN.has(tok.text)) depth++;
    else if (tok.text === ")" || tok.text === "]" || tok.text === "}") depth--;
    else if (tok.text === "," && depth === 0) {
      out.push([s, tok.e]);
      s = tok.e;
    }
  }
  if (s < end) out.push([s, end]);
  return out;
}

/** Index of the bracket matching the one at `open`. */
function matching(t: string, open: number, end = t.length): number {
  const toks = tokens(t, open, end);
  let depth = 0;
  for (const tok of toks) {
    if (OPEN.has(tok.text)) depth++;
    else if (tok.text === ")" || tok.text === "]" || tok.text === "}") {
      depth--;
      if (depth === 0) return tok.s;
    }
  }
  return -1;
}

// ---- Classification ------------------------------------------------------------------------

type Span = [cat: string, s: number, e: number];

const FN_START = /^(async\s+)?(function\b|class\b|\(.*?\)\s*=>|[A-Za-z_$][\w$]*\s*=>)/s;

/** Is the expression at t[s..] a function (or class) expression? */
function isFunction(t: string, s: number): boolean {
  const head = t.slice(s, s + 400).trimStart();
  if (/^(async\s+)?(function\b|class\b)/.test(head)) return true;
  if (/^(async\s+)?[A-Za-z_$][\w$]*\s*=>/.test(head)) return true;
  const m = /^(async\s*)?\(/.exec(head);
  if (m) {
    const open = s + (t.slice(s).length - t.slice(s).trimStart().length) + m[0].length - 1;
    const close = matching(t, open);
    return close >= 0 && /^\s*=>/.test(t.slice(close + 1, close + 10));
  }
  return false;
}

/** A continuation's text, split into scaffolding lines, blank lines, indentation and code. */
function contSpans(t: string, s: number, e: number, base: string): Span[] {
  const out: Span[] = [];
  let i = s;
  while (i < e) {
    let j = t.indexOf("\n", i);
    j = j < 0 || j >= e ? e : j + 1;
    const line = t.slice(i, j);
    if (line.trim() === "") out.push([`${base}.blank`, i, j]);
    else if (line.includes("$djs_")) out.push([`${base}.scaf`, i, j]);
    else {
      const ind = line.length - line.trimStart().length;
      if (ind > 0) out.push([`${base}.indent`, i, i + ind]);
      out.push([`${base}.code`, i + ind, j]);
    }
    i = j;
  }
  return out;
}

/** A `$djs` record `Object.defineProperty(K, "$djs", { value: { f: ..., at, segs, scaf, cmap } });`. */
function recordSpans(t: string, s: number, e: number): Span[] {
  const out: Span[] = [];
  const v = t.indexOf("{ value: {", s);
  if (v < 0 || v > e) return [["record.wrap", s, e]];
  const open = v + "{ value: ".length;
  const close = matching(t, open, e);
  out.push(["record.wrap", s, open + 1]);
  for (const [ms, me] of commaSplit(t, open + 1, close)) {
    const key = /^\s*(\w+)/.exec(t.slice(ms, me))?.[1] ?? "other";
    const k = ["f", "at", "segs", "scaf", "cmap"].includes(key) ? key : "other";
    if (k === "f" && /^\s*f:\s*"/.test(t.slice(ms, me))) out.push(["record.f-text", ms, me]);
    else out.push([`record.${k}`, ms, me]);
  }
  out.push(["record.wrap", close, e]);
  return out;
}

/** Object literal members t[open+1, close): classified by key. */
function memberSpans(t: string, open: number, close: number): Span[] {
  const out: Span[] = [];
  out.push(["scope.wrap", open, open + 1]);
  for (const [ms, me] of commaSplit(t, open + 1, close)) {
    const m = /^\s*([\w$]+|"[^"]*")\s*(:)?/.exec(t.slice(ms, me));
    const key = m?.[1] ?? "";
    if (!m?.[2]) {
      // Shorthand `name` or method `name() {}`.
      out.push([/\(/.test(t.slice(ms, me)) ? "fn" : "scope.wrap", ms, me]);
      continue;
    }
    const vs = ms + m[0].length;
    if (/^\$Ka\d/.test(key)) out.push(["cont.head", ms, vs], ...contSpans(t, vs, me, "cont"));
    else if (/^\$Fa\d/.test(key)) out.push(["origin", ms, me]);
    else if (isFunction(t, vs)) out.push(["fn", ms, me]);
    else out.push(["data", ms, me]);
  }
  out.push(["scope.wrap", close, close + 1]);
  return out;
}

/** A scope factory `(() => { BODY })()` whose `(` is at `open`. */
function factorySpans(t: string, s: number, e: number): Span[] {
  const brace = t.indexOf("{", s);
  const close = matching(t, brace, e);
  if (brace < 0 || close < 0) return [["scope.other", s, e]];
  const out: Span[] = [["scope.wrap", s, brace + 1]];
  for (const [ss, se] of statements(t, brace + 1, close)) {
    const lead = t.slice(ss, se).length - t.slice(ss, se).trimStart().length;
    const st = t.slice(ss + lead, se);
    if (lead > 0) out.push(["scope.wrap", ss, ss + lead]);
    const b = ss + lead;
    let m: RegExpExecArray | null;
    if (/^const \$s\d+ = \(\(\) => \{/.test(st)) {
      const eq = t.indexOf("(", b);
      out.push(["scope.wrap", b, eq], ...factorySpans(t, eq, se));
    } else if (/^(async\s+)?(function\b|class\b)/.test(st)) out.push(["fn", b, se]);
    else if ((m = /^(let|const|var) [\w$]+ = /.exec(st))) {
      out.push([isFunction(t, b + m[0].length) ? "fn" : "data.scopevar", b, se]);
    } else if (/^return \{/.test(st)) {
      const ob = t.indexOf("{", b);
      const oc = matching(t, ob, se);
      out.push(["scope.wrap", b, ob], ...memberSpans(t, ob, oc), ["scope.wrap", oc + 1, se]);
    } else out.push(["scope.other", b, se]);
  }
  out.push(["scope.wrap", close, e]);
  return out;
}

/** Spans of the statements in t[start, end). */
function bodySpans(t: string, start: number, end: number): Span[] {
  const out: Span[] = [];
  for (const [ss, se] of statements(t, start, end)) {
    const lead = t.slice(ss, se).length - t.slice(ss, se).trimStart().length;
    if (lead > 0) out.push(["layout", ss, ss + lead]);
    const b = ss + lead;
    const st = t.slice(b, se);
    let m: RegExpExecArray | null;
    if (/^"(djs-dump \d+|use strict)";/.test(st)) out.push(["header", b, se]);
    else if (/^\$djs\.seed\(/.test(st)) out.push(["seed", b, se]);
    else if (/^globalThis\.\$dump = /.test(st)) out.push(["helpers", b, se]);
    else if (/^\$dump\.b\d+ = /.test(st)) out.push(["builtin", b, se]);
    else if (/^\$dump\.p\d+ = \$djs\.resume\(/.test(st)) out.push(["hostcall", b, se]);
    else if (/^\$dump\.p\d+ = /.test(st)) out.push(["reenact", b, se]);
    else if ((m = /^\$dump\.K[a-z]\d+ = /.exec(st))) out.push(["cont.head", b, b + m[0].length], ...contSpans(t, b + m[0].length, se, "cont"));
    else if (/^\$dump\.Fa\d+ = /.test(st)) out.push(["origin", b, se]);
    else if (/^Object\.defineProperty\([^,]+, "\$djs"/.test(st)) out.push(...recordSpans(t, b, se));
    else if ((m = /^\$dump\.ga\d+ = \(\) => \{/.exec(st))) {
      const brace = b + m[0].length - 1;
      const close = matching(t, brace, se);
      out.push(["group", b, brace + 1], ...bodySpans(t, brace + 1, close), ["group", close, se]);
    } else if (/^\$dump\.ga\d+\(\);/.test(st)) out.push(["group", b, se]);
    else if ((m = /^\$dump\.s\d+ = \(\(\) => \{/.exec(st))) {
      const p = b + st.indexOf("((") + 1;
      out.push(["scope.wrap", b, p], ...factorySpans(t, p, se));
    } else if ((m = /^Object\.assign\(\$dump\.s\d+, \{/.exec(st))) {
      const ob = b + m[0].length - 1;
      const oc = matching(t, ob, se);
      out.push(["scope.wrap", b, ob], ...memberSpans(t, ob, oc), ["scope.wrap", oc + 1, se]);
    } else if (/^\$dump\.a\d+_\d+ = /.test(st)) out.push(["operand", b, se]);
    else if (/^\$dump\.a\d+( = |\.next\()/.test(st)) out.push(["activation", b, se]);
    else if (/^\$dump\.i\d+ = /.test(st)) out.push(["iterator", b, se]);
    else if ((m = /^\$dump\.[nsv]\d+ = /.exec(st))) out.push([isFunction(t, b + m[0].length) ? "fn" : "data", b, se]);
    else if (/^(var|let|const) [\w$]+ = /.test(st)) out.push(["globals", b, se]);
    else if (/^(async\s+)?(function\b|class\b)/.test(st)) out.push(["fn.global", b, se]);
    else if (/^\$dump\.(keep|D|Q)\(|^[\w$.]+\.prototype = /.test(st)) out.push(["fixup.other", b, se]);
    else if (/^Object\.defineProperty\([^;]*, "name", /.test(st)) out.push(["fixup.name", b, se]);
    else if (/^Object\.(freeze|seal|preventExtensions)\(/.test(st)) out.push(["fixup.freeze", b, se]);
    else if (/^(Object|Reflect)\.\w+\(/.test(st)) out.push(["fixup.other", b, se]);
    else if (/^[\w$]+(\.[\w$]+|\[[^\]]*\])+ = /.test(st) || /^\$dump\.s\d+\.\$set\(/.test(st)) out.push(["patch", b, se]);
    else if (/^try \{/.test(st)) out.push(["final", b, se]);
    else out.push(["other", b, se]);
  }
  return out;
}

/** Fills gaps so the spans partition [0, t.length); the gaps are `layout`. */
function partition(t: string, spans: Span[]): Span[] {
  const sorted = spans.filter(([, s, e]) => e > s).sort((a, b) => a[1] - b[1]);
  const out: Span[] = [];
  let at = 0;
  for (const [c, s, e] of sorted) {
    if (s > at) out.push(["layout", at, s]);
    const ss = Math.max(s, at);
    if (e > ss) out.push([c, ss, e]);
    at = Math.max(at, e);
  }
  if (at < t.length) out.push(["layout", at, t.length]);
  return out;
}

/**
 * The test environment's share: the corpus prelude's state (its functions, scopes and
 * globals) and the codemode-harvest harness (its origin, with the synthetic tool results
 * in its text, the result queues and the fake tools), which a pi session does not dump
 * (prelude-next keeps the prelude out of the dump; results come from tools). Such spans
 * move to `env.<category>` so the rest reads like a real session's dump.
 */
function envSpans(t: string, spans: Span[]): Span[] {
  return spans.map(([c, s, e]): Span => {
    const text = t.slice(s, e);
    if (c === "origin" && text.includes("// Harvested pi codemode script")) return ["env.harness", s, e];
    if (c === "data" && /^\$dump\.n\d+ = \[\{ ok: (true|false), value: /.test(text)) return ["env.harness", s, e];
    if (!/^(fn|scope|globals|fixup|data|patch)/.test(c)) return [c, s, e];
    const lines = text.split("\n").map((l) => l.trim()).filter((l) => l.length >= 8);
    if (!lines.length) return [c, s, e];
    const env = lines.filter((l) => ENV_LINES.has(l)).length;
    return env * 2 >= lines.length ? [`env.${c.split(".")[0]}`, s, e] : [c, s, e];
  });
}

// ---- Provenance ----------------------------------------------------------------------------

/** Lines of `text` trimmed, at least 8 chars. */
function lineSet(text: string): Set<string> {
  return new Set(text.split("\n").map((l) => l.trim()).filter((l) => l.length >= 8));
}

const PRELUDES: [string, Set<string>][] = [
  ["corpus-prelude", lineSet(readFileSync(join(root, "tests/corpus/prelude.js"), "utf8"))],
  ["pi-prelude", lineSet(existsSync(join(root, "tests/pi/prelude-next.ts")) ? readFileSync(join(root, "tests/pi/prelude-next.ts"), "utf8") : "")],
];

/** Lines of the environment: the corpus prelude and the harvest harness (each harvest file before its script). */
const ENV_LINES = (() => {
  const out = new Set(PRELUDES[0][1]);
  const dir = join(root, "tests/corpus/codemode-harvest");
  if (existsSync(dir))
    for (const f of readdirSync(dir).filter((f) => f.endsWith(".js"))) {
      const text = readFileSync(join(dir, f), "utf8");
      const cut = text.indexOf("return await (async");
      for (const l of lineSet(cut < 0 ? "" : text.slice(0, cut))) out.add(l);
    }
  return out;
})();

/**
 * Bytes by where each line comes from: `repeat` (a line, at least 8 chars trimmed, that
 * already appeared in this dump), `prelude` (a line of the corpus or pi prelude),
 * `script` (a line of the dumped script's own source), `generated` (everything else).
 */
function provenance(t: string, src: Set<string> | null): Record<string, number> {
  const seen = new Set<string>();
  const out: Record<string, number> = { repeat: 0, prelude: 0, script: 0, generated: 0 };
  for (const line of t.split("\n")) {
    const k = line.trim();
    const n = line.length + 1;
    if (k.length >= 8 && seen.has(k)) out.repeat += n;
    else if (k.length >= 8 && PRELUDES.some(([, s]) => s.has(k))) out.prelude += n;
    else if (k.length >= 8 && src?.has(k)) out.script += n;
    else out.generated += n;
    if (k.length >= 8) seen.add(k);
  }
  return out;
}

// ---- Main ----------------------------------------------------------------------------------

/**
 * Repeated lines (at least 8 chars trimmed, seen before in the dump) by the top category
 * of the repeat and of the first occurrence: `"origin<cont"` counts origin bytes that
 * repeat a line first written in a continuation.
 */
function repeats(t: string, spans: Span[]): Record<string, number> {
  const out: Record<string, number> = {};
  const first = new Map<string, string>();
  let si = 0;
  let i = 0;
  for (const line of t.split("\n")) {
    while (si < spans.length - 1 && spans[si][2] <= i + (line.length - line.trimStart().length)) si++;
    const cat = topCat(spans[si]?.[0] ?? "layout");
    const k = line.trim();
    if (k.length >= 8) {
      const f = first.get(k);
      if (f !== undefined) out[`${cat}<${f}`] = (out[`${cat}<${f}`] ?? 0) + line.length + 1;
      else first.set(k, cat);
    }
    i += line.length + 1;
  }
  return out;
}

interface Dump {
  suite: string;
  file: string;
  raw: number;
  gz: number;
  cats: Record<string, number>;
  marg: Record<string, number>;
  prov: Record<string, number>;
  rep: Record<string, number>;
}

const gz = (s: string) => gzipSync(s, { level: 9 }).length;

function walk(dir: string): string[] {
  return readdirSync(dir)
    .sort()
    .flatMap((e) => {
      const p = join(dir, e);
      return statSync(p).isDirectory() ? walk(p) : p.endsWith(".js") ? [p] : [];
    });
}

const topCat = (c: string) => c.split(".")[0];

function analyse(suite: string, file: string, srcRoot: string | null, dir: string): Dump {
  const t = readFileSync(file, "utf8");
  const spans = envSpans(t, partition(t, bodySpans(t, 0, t.length)));
  const cats: Record<string, number> = {};
  for (const [c, s, e] of spans) cats[c] = (cats[c] ?? 0) + (e - s);
  const full = gz(t);
  // Marginal gzip cost per top category: the dump with that category's bytes removed.
  const marg: Record<string, number> = {};
  for (const top of new Set(spans.map(([c]) => topCat(c)))) {
    const rest = spans.filter(([c]) => topCat(c) !== top).map(([, s, e]) => t.slice(s, e)).join("");
    marg[top] = full - gz(rest);
  }
  let src: Set<string> | null = null;
  if (srcRoot) {
    const caseRel = dirname(relative(dir, file));
    for (const cand of [join(srcRoot, caseRel), join(srcRoot, caseRel + ".src.js"), join(srcRoot, caseRel + ".body.js")]) {
      if (existsSync(cand) && statSync(cand).isFile()) {
        src = lineSet(readFileSync(cand, "utf8"));
        break;
      }
    }
  }
  return { suite, file: relative(root, file), raw: t.length, gz: full, cats, marg, prov: provenance(t, src), rep: repeats(t, spans) };
}

export { bodySpans, partition, envSpans, topCat, walk, gz, type Span };

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv[2] === "--score") score(process.argv.slice(3));
  else main();
}

/**
 * The size score (0207, D2901): `--score FILE [--update] SUITE=DIR ...` totals each suite's
 * dumps without their `env.*` bytes (the corpus prelude and the harvest harness, which a
 * pi session does not dump), raw and gzip -9 per dump, and compares them with FILE's
 * `SUITE_raw` / `SUITE_gzip` lines: a total above its line fails (beyond a tolerance for
 * the wall clock's digits in the dumps and zlib versions); one well below it asks for
 * `--update`, which rewrites the lines, so that the score is a ratchet.
 */
function score(argv: string[]) {
  const file = argv[0];
  const update = argv.includes("--update");
  const totals: [string, number][] = [];
  for (const spec of argv.slice(1).filter((a) => a.includes("="))) {
    const [name, dir] = spec.split("=");
    let raw = 0;
    let g = 0;
    let n = 0;
    for (const f of walk(dir)) {
      const t = readFileSync(f, "utf8");
      const kept = envSpans(t, partition(t, bodySpans(t, 0, t.length)))
        .filter(([c]) => !c.startsWith("env"))
        .map(([, s, e]) => t.slice(s, e))
        .join("");
      raw += kept.length;
      g += gz(kept);
      n++;
    }
    if (n === 0) throw new Error(`dump-size: no dumps in ${dir}`);
    totals.push([`${name}_raw`, raw], [`${name}_gzip`, g]);
  }
  const text = existsSync(file) ? readFileSync(file, "utf8") : "";
  const recorded = new Map(
    text
      .split("\n")
      .filter((l) => l && !l.startsWith("#"))
      .map((l) => l.split("\t"))
      .map(([k, v]) => [k, Number(v)] as const),
  );
  let failed = false;
  let stale = false;
  for (const [k, v] of totals) {
    const r = recorded.get(k);
    const tol = k.endsWith("_gzip") ? 0.005 : 0.001;
    const note = r === undefined ? "new" : v > r * (1 + tol) ? "ABOVE the score" : v < r * (1 - tol) ? "below the score: --update" : "ok";
    if (r !== undefined && v > r * (1 + tol)) failed = true;
    if (r === undefined || v < r * (1 - tol)) stale = true;
    console.log(`dump-size: ${k} ${v} (score ${r ?? "-"}): ${note}`);
  }
  if (update) {
    const head = text.split("\n").filter((l) => l.startsWith("#"));
    const keep = [...recorded].filter(([k]) => !totals.some(([t]) => t === k));
    writeFileSync(file, [...head, ...[...keep, ...totals].map(([k, v]) => `${k}\t${v}`)].join("\n") + "\n");
    console.log(`dump-size: wrote ${file}`);
    return;
  }
  if (failed) {
    console.log("dump-size: plain dumps grew; see work/research/0207 and scripts/dump-size");
    process.exit(1);
  }
  if (stale) console.log("dump-size: the score is stale; lower it with --update");
}

function main() {
const argv = process.argv.slice(2);
// `--show FILE`: the dump's spans, one per line (to check the scanner).
if (argv[0] === "--show") {
  const t = readFileSync(argv[1], "utf8");
  for (const [c, s, e] of partition(t, bodySpans(t, 0, t.length))) console.log(c.padEnd(16), String(e - s).padStart(7), JSON.stringify(t.slice(s, s + 70)));
  process.exit(0);
}
const outArg = argv.indexOf("--out");
const tsvArg = argv.indexOf("--tsv");
const outFile = outArg >= 0 ? argv[outArg + 1] : null;
const tsvFile = tsvArg >= 0 ? argv[tsvArg + 1] : null;
const suites = argv.filter((a, i) => a.includes("=") && argv[i - 1] !== "--out" && argv[i - 1] !== "--tsv");
const dumps: Dump[] = [];
for (const s of suites) {
  const [name, spec] = s.split("=");
  const [dir, srcRoot] = spec.split(":");
  const files = statSync(dir).isDirectory() ? walk(dir) : [dir];
  for (const f of files) dumps.push(analyse(name, f, srcRoot ?? null, statSync(dir).isDirectory() ? dir : dirname(dir)));
}

const kb = (n: number) => (n / 1024).toFixed(1);
const pct = (a: number, b: number) => (b ? ((100 * a) / b).toFixed(1) : "0.0");
const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);
const quant = (xs: number[], q: number) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length ? s[Math.min(s.length - 1, Math.floor(q * s.length))] : 0;
};

const lines: string[] = [];
const p = (l = "") => lines.push(l);
for (const suite of [...new Set(dumps.map((d) => d.suite)), "all"]) {
  const ds = suite === "all" ? dumps : dumps.filter((d) => d.suite === suite);
  if (!ds.length) continue;
  const raw = sum(ds.map((d) => d.raw));
  const gzt = sum(ds.map((d) => d.gz));
  p(`## ${suite}: ${ds.length} dumps, ${kb(raw)} KB raw, ${kb(gzt)} KB gzip -9 (each dump alone)`);
  p();
  p(`per dump raw: median ${quant(ds.map((d) => d.raw), 0.5)}, p90 ${quant(ds.map((d) => d.raw), 0.9)}, max ${quant(ds.map((d) => d.raw), 1)}; gzip median ${quant(ds.map((d) => d.gz), 0.5)}, p90 ${quant(ds.map((d) => d.gz), 0.9)}`);
  p();
  p("| category | raw KB | raw % | marginal gzip KB | gzip % | dumps with it | median share |");
  p("|---|---:|---:|---:|---:|---:|---:|");
  const allCats = [...new Set(ds.flatMap((d) => Object.keys(d.cats)))];
  const tops = [...new Set(allCats.map(topCat))];
  const rows = tops
    .map((top) => {
      const r = sum(ds.map((d) => sum(Object.entries(d.cats).filter(([c]) => topCat(c) === top).map(([, n]) => n))));
      const g = sum(ds.map((d) => d.marg[top] ?? 0));
      const shares = ds.map((d) => sum(Object.entries(d.cats).filter(([c]) => topCat(c) === top).map(([, n]) => n)) / d.raw);
      return { top, r, g, n: shares.filter((x) => x > 0).length, med: quant(shares, 0.5) };
    })
    .sort((a, b) => b.r - a.r);
  for (const row of rows) {
    p(`| **${row.top}** | ${kb(row.r)} | ${pct(row.r, raw)} | ${kb(row.g)} | ${pct(row.g, gzt)} | ${row.n} | ${(100 * row.med).toFixed(1)} |`);
    const subs = allCats.filter((c) => topCat(c) === row.top && c !== row.top);
    for (const c of subs.sort()) {
      const r = sum(ds.map((d) => d.cats[c] ?? 0));
      p(`| &nbsp; ${c} | ${kb(r)} | ${pct(r, raw)} | | | | |`);
    }
  }
  p();
  const prov = ["repeat", "prelude", "script", "generated"].map((k) => [k, sum(ds.map((d) => d.prov[k]))] as const);
  p(`lines by provenance: ${prov.map(([k, n]) => `${k} ${kb(n)} KB (${pct(n, raw)}%)`).join(", ")}`);
  p();
  const rep: Record<string, number> = {};
  for (const d of ds) for (const [k, n] of Object.entries(d.rep)) rep[k] = (rep[k] ?? 0) + n;
  const reps = Object.entries(rep).sort((a, b) => b[1] - a[1]).slice(0, 8);
  p(`repeated lines (repeat<first): ${reps.map(([k, n]) => `${k} ${kb(n)} KB (${pct(n, raw)}%)`).join(", ")}`);
  p();
  if (suite !== "all") {
    p("largest dumps:");
    p();
    for (const d of [...ds].sort((a, b) => b.raw - a.raw).slice(0, 5)) {
      const top3 = Object.entries(d.cats).sort((a, b) => b[1] - a[1]).slice(0, 4);
      p(`- ${d.file}: ${d.raw} B raw, ${d.gz} gzip; ${top3.map(([c, n]) => `${c} ${pct(n, d.raw)}%`).join(", ")}`);
    }
    p();
  }
}
const report = lines.join("\n");
if (outFile) writeFileSync(outFile, report + "\n");
else console.log(report);
if (tsvFile) {
  const cats = [...new Set(dumps.flatMap((d) => Object.keys(d.cats)))].sort();
  const head = ["suite", "file", "raw", "gz", ...cats, ...["repeat", "prelude", "script", "generated"].map((k) => `prov.${k}`)];
  const rows = dumps.map((d) => [d.suite, d.file, d.raw, d.gz, ...cats.map((c) => d.cats[c] ?? 0), ...["repeat", "prelude", "script", "generated"].map((k) => d.prov[k])].join("\t"));
  writeFileSync(tsvFile, [head.join("\t"), ...rows].join("\n") + "\n");
}
}
