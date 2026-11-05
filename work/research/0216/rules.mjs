// Mechanical rewrites of main's plain dumps toward the byte-minimal form (0216). Each rule is
// one category of the accounting: it takes the dump's text and returns the rewritten text.
// They implement only what can be done safely on the text alone; the hand-written minimal
// dumps (cases/) go further. A rewritten dump needs the host changes the README lists
// (the host deletes `$djs`, positional `$djs.k`), which check.mjs's harness provides.

import { tokens, matchClose, splitTop, statements } from "./scan.mjs";

const FINAL = /^try \{ \((.*)\); \} finally \{ delete globalThis\.\$dump; delete globalThis\.\$djs; \}\n?$/m;

/** The site name as a number where it is `await#N` (the common case), else the string. */
export function siteCode(at) {
  if (at === undefined) return undefined;
  const m = /^"await#(\d+)"$/.exec(at);
  return m ? m[1] : at;
}

/** Splits `{ k: v, ... }` (text with braces) into [key, valueText] pairs. */
function objFields(text) {
  const toks = tokens(text);
  const close = matchClose(text, 0, toks);
  return splitTop(text, 1, close, toks)
    .map(([s, e]) => text.slice(s, e).trim())
    .filter(Boolean)
    .map((f) => {
      const i = f.indexOf(":");
      return [f.slice(0, i).trim(), f.slice(i + 1).trim()];
    });
}

/**
 * The continuation's record, `$djs.k(K, { f, at, segs, scaf, cmap }[, proto])`. Mode "layout":
 * the record without its layout fields (segs/scaf/cmap; a re-dump makes K's layout again
 * from f and the site, 0207 item 4). Mode "positional": `$djs.k(K, f[, at][, proto])`, the
 * site a number where it is `await#N`, the proto left out where it is `f.prototype`.
 */
export function records(text, mode) {
  let out = "";
  let i = 0;
  const toks = tokens(text);
  for (;;) {
    const j = text.indexOf("$djs.k(", i);
    if (j < 0) break;
    const open = j + "$djs.k".length;
    const close = matchClose(text, open, toks);
    const args = splitTop(text, open + 1, close, toks).map(([s, e]) => text.slice(s, e));
    const K = args[0];
    const rec = objFields(args[1].trim());
    const get = (k) => rec.find(([key]) => key === k)?.[1];
    const f = get("f");
    let proto = args[2]?.trim();
    let repl;
    if (mode === "layout") {
      const kept = rec.filter(([k]) => !["segs", "scaf", "cmap"].includes(k)).map(([k, v]) => `${k}: ${v}`);
      repl = `$djs.k(${K}, { ${kept.join(", ")} }${proto !== undefined ? ", " + proto : ""})`;
    } else {
      const pos = [f];
      const at = siteCode(get("at"));
      if (at !== undefined) pos.push(at);
      if (proto !== undefined && proto === `${f}.prototype`) proto = undefined;
      if (proto !== undefined) {
        if (at === undefined) pos.push("1");
        pos.push(proto);
      }
      repl = `$djs.k(${K}, ${pos.join(", ")})`;
    }
    out += text.slice(i, j) + repl;
    i = close + 1;
  }
  return out + text.slice(i);
}

/** The final `try { (X); } finally { delete ... }` → `X` (nothing when no handle). */
export function final(text) {
  return text.replace(FINAL, (_, x) => (x === "undefined" ? "" : (x.startsWith("{") ? `(${x})` : x) + "\n"));
}

/** `$djs.seed(lo, hi, next)`: `next` left out where it is one past the highest reopened id. */
export function seed(text) {
  const ids = [...text.matchAll(/\$djs\.resume\((\d+),/g)].map((m) => Number(m[1]));
  const derived = ids.length ? Math.max(...ids) + 1 : undefined;
  return text.replace(/^\$djs\.seed\((-?\d+), (-?\d+), (\d+)\);$/m, (all, lo, hi, next) =>
    Number(next) === derived ? `$djs.seed(${lo}, ${hi});` : all,
  );
}

/** Uses of `$dump.NAME` in text, by name. */
function uses(text) {
  const n = new Map();
  for (const t of tokens(text)) {
    const m = /^\$dump\.([A-Za-z_$][\w$]*)/.exec(t.t);
    if (m) n.set(m[1], (n.get(m[1]) ?? 0) + 1);
  }
  return n;
}

/**
 * Single-use values written in place: a top-level `$dump.X = E;` whose X occurs once more,
 * for E of the kinds whose evaluation can move later without a visible difference:
 * `$djs.source(...)` (a host lookup), `$djs.host(...)`, and a `$djs.resume(...)` when it is the
 * dump's only one (so the order of reopened calls cannot change).
 */
export function inline(text) {
  const resumes = (text.match(/= \$djs\.resume\(/g) ?? []).length;
  for (let guard = 0; guard < 1000; guard++) {
    const st = statements(text);
    const n = uses(text);
    let done = true;
    for (const [s, e] of st) {
      const m = /^\$dump\.([\w$]+) = (\$djs\.(source|host|resume)\(.*\));\n$/s.exec(text.slice(s, e));
      if (!m || n.get(m[1]) !== 2) continue;
      if (m[3] === "resume" && resumes !== 1) continue;
      const rest = text.slice(0, s) + text.slice(e);
      const re = new RegExp(`\\$dump\\.${m[1].replace(/\$/g, "\\$")}(?![\\w$])`);
      text = rest.replace(re, m[2]);
      done = false;
      break;
    }
    if (done) break;
  }
  return text;
}

/** The dump's holder: `$dump.` → `$d.` (an owner decision: shorter generated names). */
export function holder(text) {
  return text.replace(/globalThis\.\$dump = \{\};/, "globalThis.$d = {};").replace(/\$dump\./g, "$d.");
}

/** Drops `globalThis.$dump = {};` when nothing uses `$dump` any more. */
export function holderGone(text) {
  if (!/\$dump\.|\$d\./.test(text.replace(/^globalThis\.\$(dump|d) = \{\};\n/m, ""))) {
    return text.replace(/^globalThis\.\$(dump|d) = \{\};\n/m, "");
  }
  return text;
}

/** Generated scaffold names `$djs_x` → `$x` (an owner decision). */
export function scaffoldNames(text) {
  return text.replace(/\$djs_/g, "$");
}

/** Builtin calls written as methods where frozen builtins make them the same. */
export function methods(text) {
  return text
    .replace(/Promise\.prototype\.then\.call\((\$dump\.[\w$]+), /g, "$1.then(")
    .replace(/Array\.prototype\.values\.call\((\[[^\]\n]*\])\)/g, "$1.values()")
    .replace(/\.then\(([^()\n]*?), undefined\)/g, ".then($1)");
}

/**
 * Whitespace of generated top-level statements (those that hold no function text): spaces
 * after commas and colons, around `=`, inside braces; the user's own text is not touched.
 */
export function whitespace(text) {
  const st = statements(text);
  let out = "";
  for (const [s, e] of st) {
    let line = text.slice(s, e);
    const generated = /^(\$dump\.|\$d\.|\$djs\.|globalThis\.|try \{)/.test(line) && !/=>|function|class /.test(line);
    if (generated) {
      let r = "";
      for (const t of tokens(line)) {
        if (/^\s+$/.test(t.t)) {
          // keep one space only between two word characters, and the final newline
          if (t.t.includes("\n") && t.e === line.length) r += "\n";
          else {
            const a = line[t.s - 1] ?? "";
            const b = line[t.e] ?? "";
            if (/[\w$]/.test(a) && /[\w$]/.test(b)) r += " ";
          }
        } else r += t.t;
      }
      line = r;
    }
    out += line;
  }
  return out;
}

/** The rules in the order the accounting applies them. */
export const RULES = [
  ["final: host deletes $djs; completion as the last expression", final],
  ["record: layout fields (segs/scaf/cmap) left out", (t) => records(t, "layout")],
  ["record: positional, at as a number, proto derived", (t) => records(t, "positional")],
  ["seed: next left out where derivable", seed],
  ["single-use host values in place", inline],
  ["builtins as methods (then/values)", methods],
  ["holder: $dump. -> $d.", holder],
  ["holder gone when unused", holderGone],
  ["scaffold names $djs_ -> $", scaffoldNames],
  ["whitespace of generated statements", whitespace],
];
