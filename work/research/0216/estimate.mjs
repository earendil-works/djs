// Upper-bound estimates (0216) of the items the hand-written cases show but rules.mjs does
// not do mechanically, over whole suites of main's dumps: the bytes of the constructs each
// item removes (raw only; the hand-written cases give the gzip ratio).
//   node work/research/0216/estimate.mjs SUITE=DIR ...
import fs from "node:fs";
import path from "node:path";
import { tokens, statements } from "./scan.mjs";

const walk = (d) =>
  fs.readdirSync(d, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(d, e.name)) : [path.join(d, e.name)]));

const items = {
  // `$dump.X = E;` whose X occurs once more: the definition's `$dump.X = ` and `;\n`, the
  // use's `$dump.X` (E moves to the use). Excludes host values and lone resumes (rules.mjs).
  "inline: single-use values (all kinds)": (t) => {
    const n = new Map();
    for (const k of tokens(t)) {
      const m = /^\$dump\.([A-Za-z_$][\w$]*)/.exec(k.t);
      if (m) n.set(m[1], (n.get(m[1]) ?? 0) + 1);
    }
    let b = 0;
    for (const [s, e] of statements(t)) {
      const m = /^\$dump\.([\w$]+) = /.exec(t.slice(s, s + 40));
      if (m && n.get(m[1]) === 2) b += m[0].length + 2 + `$dump.${m[1]}`.length;
    }
    return b;
  },
  // Callbacks: `$dump.gaN = () => {\n…};` wrappers and their calls, factory `Object.assign`
  // plumbing and `var x = $dump.sN.x;` rebindings (the expressions move to the site).
  "callback: plumbing": (t) => {
    let b = 0;
    for (const m of t.matchAll(/^\$dump\.ga[\w]+ = \(\) => \{\n|^\};\n|^\$dump\.ga[\w]+\(\);\n|^\$dump\.s\d+ = \{\};\n|^var [\w$]+ = \$dump\.s\d+\.[\w$]+;\n|Object\.assign\(\$dump\.s\d+, \{ |\$dump\.s\d+\.\$Ka\d+/gm)) b += m[0].length;
    return b;
  },
  // Operand copies and temporaries a reread makes unnecessary: `$dump.aN_k = …;` lines,
  // `const $djs_tN = …` declarations, captures of R/C/D/Q/A.
  "reread: temporaries and operand copies": (t) => {
    let b = 0;
    for (const m of t.matchAll(/^\$dump\.a\d+_\d+ = .*;\n|^const \$djs_[RCDQAG] = .*;\n|^const \$djs_t\d+ = .*;\n/gm)) b += m[0].length;
    return b;
  },
  // Loops over builtin iterators: the helper capture, the scaffold constant, `$djs_I(…)`,
  // `$djs.advance(Array.prototype.values.call(` (an argument `[…].values()` instead).
  "loop: iterator plumbing": (t) => {
    let b = 0;
    for (const m of t.matchAll(/^const \$djs_[IL] = \$djs\.[IL];\n|^const \$djs_i\d+ = .*;\n|\$djs_I\(|\$djs\.advance\(Array\.prototype\.values\.call\(|\), \d+\)/gm)) b += m[0].length;
    return b;
  },
  // `$djs.keep(C, [])` and friends: upper bound (only no-op ones go).
  "keep: no-op calls (upper bound)": (t) => (t.match(/^\$djs\.keep\(.*\);\n/gm) ?? []).join("").length,
  // Continuations of the same function and site after the first one: their text.
  "same site: repeated continuations": (t) => {
    const seen = new Map();
    let b = 0;
    for (const m of t.matchAll(/\$Ka\d+: (async |function\*? ?|\()?[\s\S]*?\n\}(?=, \$|\s\}\))/g)) {
      const body = m[0].replace(/^\$Ka\d+: /, "").replace(/\$dump\.p\d+/g, "P");
      if (seen.has(body)) b += m[0].length;
      else seen.set(body, 1);
    }
    return b;
  },
  // Scaffold names `$djs_x`: 4 bytes each as `$x` (rules.mjs does it; here for reference).
  "names: $djs_ (reference)": (t) => (t.match(/\$djs_/g) ?? []).length * 4,
};

for (const s of process.argv.slice(2)) {
  const [name, dir] = s.split("=");
  const tot = Object.fromEntries(Object.keys(items).map((k) => [k, 0]));
  let raw = 0;
  let n = 0;
  for (const f of walk(dir).filter((f) => f.endsWith(".js"))) {
    const t = fs.readFileSync(f, "utf8");
    if (!t.startsWith('"djs-dump')) continue;
    n++;
    raw += Buffer.byteLength(t);
    for (const [k, fn] of Object.entries(items)) tot[k] += fn(t);
  }
  console.log(`\n### ${name}: ${n} dumps, ${raw} B raw\n\n| item | bytes (upper bound) | share |\n|---|---:|---:|`);
  for (const [k, b] of Object.entries(tot)) console.log(`| ${k} | ${b} | ${((b / raw) * 100).toFixed(1)}% |`);
}
