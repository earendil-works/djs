#!/usr/bin/env node
// plain-node.mjs DIR [--list] [--refusals FILE [--update]]: runs the plain dumps `djs-difftest --plain-out DIR` wrote
// in node and checks that each continues exactly like the uninterrupted djs run (D2861).
//
// DIR/<case>/<k>.js is link k's dump, <k>.pre the output before it and `transcript` the
// djs run's whole transcript. Each dump runs as a Script in a fresh node context with
// the difftest prelude's host: output(text), done(kind, text), later(v, rejected) (the
// oldest open call is answered at each idle point, when the microtask queue is empty)
// and $djs: the dump runtime (js/src/dump-runtime.js) with the hooks seed, resume and host.
// The transcript it produces, after <k>.pre, must equal `transcript`.
//
// A case whose dump the exporter refused has a `refused` file (the run went on without
// dumps). With --refusals, FILE lists the cases that may refuse, one per line with the
// reason after a tab (the ratchet): another case's refusal fails, entries that no longer
// refuse are reported as stale; --update rewrites FILE.
//
// A case marked `// difftest: plain-xfail <reason>` (D2941) has a `plain-xfail` file: the
// reason and whether the djs run of its dumps differs. Its links may differ; once neither
// djs nor node differs, the mark is reported as stale (not a failure).
//
// Exit status 1 if any link differs or a case refuses that FILE does not list; one line
// per problem, and a summary.
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";

process.env.TZ = "UTC"; // as the difftest runs djs and qjs

const dir = process.argv[2];
const list = process.argv.includes("--list");
const update = process.argv.includes("--update");
const ri = process.argv.indexOf("--refusals");
const refusalsFile = ri > 0 ? process.argv[ri + 1] : null;
if (!dir || (ri > 0 && !refusalsFile)) {
  console.error("usage: plain-node.mjs DIR [--list] [--refusals FILE [--update]]");
  process.exit(2);
}

// Stack frame lines are not compared (the difftest's normalize drops them too), and
// node's wording of the common TypeErrors reads as djs's.
const norm = (t) =>
  t
    .split("\n")
    .filter((l) => !/^\s+at /.test(l))
    .map((l) => l.replace(/Cannot read properties of (undefined|null) \(reading '([^']*)'\)/g, "cannot read property '$2' of $1"))
    .join("\n");

// The dump runtime (D2903), evaluated in each link's context so that its helpers are that
// realm's: js/src/dump-runtime.js as a Script whose completion value is `helpers`.
const RUNTIME = `(() => {\n${fs
  .readFileSync(new URL("../js/src/dump-runtime.js", import.meta.url), "utf8")
  .replace(/^export (const|function) /gm, "$1 ")}\nreturn helpers;\n})()`;

async function runLink(file, now) {
  const text = fs.readFileSync(file, "utf8");
  const out = [];
  let result = null;
  const open = [];
  let finished = false;
  const ctx = vm.createContext({});
  // djs's `stack` holds only the frames (node's starts with the message line).
  vm.runInContext('Error.prepareStackTrace = (e, frames) => frames.map((f) => "    at " + f).join("\\n");', ctx);
  // djs's clock stands still at the case's start (the difftest sets it once).
  vm.runInContext(
    `{ const D = Date, now = ${now};
       globalThis.Date = new Proxy(D, {
         construct: (t, a, nt) => Reflect.construct(D, a.length ? a : [now], nt),
         apply: (t, self, a) => new D(now).toString(),
       });
       D.now = () => now; }`,
    ctx,
  );
  const g = vm.runInContext("globalThis", ctx);
  const host = {
    output: (t) => {
      out.push(String(t));
    },
    done: (kind, t) => {
      if (result === null) result = [kind, String(t)];
    },
    later: (v, rejected) =>
      new Promise((res, rej) => {
        open.push({ res, rej, v, rejected });
      }),
  };
  for (const [k, f] of Object.entries(host)) {
    Object.defineProperty(g, k, { value: f, writable: true, configurable: true, enumerable: false });
  }
  Object.defineProperty(g, "$djs", {
    value: Object.assign(vm.runInContext(RUNTIME, ctx), {
      // node's Math.random cannot be seeded: cases that use it are not comparable.
      seed() {},
      resume: (id, name, args) => g.later(...args),
      // The case's function the dump continues (D2882): node never dumps again, the one
      // thing that reads it, so any function stands for it.
      source: () => function source() {},
      // A host value by name (D2971): the host's functions are these globals.
      host: (name) => {
        if (!Object.hasOwn(host, name)) throw new Error(`host: unknown host value ${name}`);
        return g[name];
      },
    }),
    writable: true,
    configurable: true,
  });
  let api;
  try {
    api = new vm.Script(text, { filename: file }).runInContext(ctx);
  } catch (e) {
    if (process.env.PLAIN_NODE_DEBUG) console.log(e);
    return `=== crash\nload threw: ${e}\n`;
  }
  if (process.env.PLAIN_NODE_DEBUG) console.log(out, result);
  if ("$djs" in g || "$dump" in g) return "=== crash\nthe dump left $djs or $dump\n";
  for (let n = 0; n < 1_000_000 && result === null; n++) {
    await new Promise((r) => setImmediate(r));
    if (result !== null) break;
    if (!open.length) break;
    const c = open.shift();
    c.rejected === true ? c.rej(c.v) : c.res(c.v);
  }
  if (result === null && api && typeof api.stalled === "function") {
    api.stalled();
    await new Promise((r) => setImmediate(r));
  }
  finished = true;
  let s = out.map((l) => l + "\n").join("");
  if (result === null) return s + "=== crash\nno result\n";
  const [kind, t] = result;
  if (kind === "completed") return s + "=== completed\n";
  return s + `=== ${kind}\n${t}\n`;
}

let links = 0;
let tsLinks = 0;
let bad = 0;
let cases = 0;
const walk = (d) => {
  const found = [];
  for (const e of fs.readdirSync(d, { withFileTypes: true })) {
    const p = path.join(d, e.name);
    if (e.isDirectory()) {
      if (fs.existsSync(path.join(p, "transcript"))) found.push(p);
      else found.push(...walk(p));
    }
  }
  return found;
};
const refused = new Map();
const xfails = [];
for (const c of walk(dir).sort()) {
  if (fs.existsSync(path.join(c, "refused"))) {
    refused.set(path.relative(dir, c), fs.readFileSync(path.join(c, "refused"), "utf8").split("\n")[0]);
  }
  const want = norm(fs.readFileSync(path.join(c, "transcript"), "utf8"));
  const xf = path.join(c, "plain-xfail");
  const xfail = fs.existsSync(xf) ? { differs: /^djs: differs$/m.test(fs.readFileSync(xf, "utf8")), node: false } : null;
  if (xfail) xfails.push([path.relative(dir, c), xfail]);
  const ks = fs
    .readdirSync(c)
    .filter((f) => /^\d+\.js$/.test(f))
    .map((f) => parseInt(f))
    .sort((a, b) => a - b);
  if (!ks.length) continue;
  cases++;
  for (const k of ks) {
    const file = path.join(c, `${k}.js`);
    const text = fs.readFileSync(file, "utf8");
    // Origin tracking (D2870): a continuation is written from its origin's text, never
    // from an earlier continuation's (whose scaffold names it would have to rename).
    if (/\$djs_[a-z]\d+_\d+\b/.test(text)) {
      if (xfail) xfail.node = true;
      else {
        bad++;
        console.log(`${path.relative(dir, c)} link ${k}: a continuation of a continuation`);
      }
    }
    // TypeScript dumps are written as JavaScript (D2879); one whose TypeScript is more
    // than types (an enum, a namespace) stays TypeScript, which node cannot run.
    if (text.startsWith('"djs-dump typescript"')) {
      tsLinks++;
      continue;
    }
    links++;
    const pre = fs.readFileSync(path.join(c, `${k}.pre`), "utf8");
    const now = fs.existsSync(path.join(c, "now")) ? Number(fs.readFileSync(path.join(c, "now"), "utf8")) : Date.now();
    const got = norm(pre + (await runLink(file, now)));
    if (got !== want && xfail) {
      xfail.node = true;
    } else if (got !== want) {
      bad++;
      const a = want.split("\n");
      const b = got.split("\n");
      let i = 0;
      while (i < a.length && a[i] === b[i]) i++;
      const name = path.relative(dir, c);
      // An end the run did not have (`=== crash`, `=== throw`) says why on its next line.
      const why = /^=== /.test(b[i] ?? "") && a[i] !== b[i] ? ` ${JSON.stringify(b[i + 1] ?? "")}` : "";
      console.log(`${name} link ${k}: line ${i + 1}: expected ${JSON.stringify(a[i] ?? "")}, got ${JSON.stringify(b[i] ?? "")}${why}`);
    } else if (list) {
      console.log(`${path.relative(dir, c)} link ${k}: ok`);
    }
  }
}
for (const [n, x] of xfails) {
  if (!x.differs && !x.node) console.log(`${n}: stale plain-xfail (djs and node continue identically now)`);
}
let unlisted = 0;
if (refusalsFile) {
  const listed = new Map();
  if (fs.existsSync(refusalsFile)) {
    for (const l of fs.readFileSync(refusalsFile, "utf8").split("\n")) {
      if (!l.trim() || l.startsWith("#")) continue;
      const [name, why] = l.split("\t");
      listed.set(name, why ?? "");
    }
  }
  if (update) {
    const head = "# Corpus cases whose plain dump the exporter refuses (scripts/plain-node.mjs --refusals).\n";
    const body = [...refused].map(([n, w]) => `${n}\t${w}\n`).join("");
    fs.writeFileSync(refusalsFile, head + body);
  } else {
    for (const [n, w] of refused) {
      if (!listed.has(n)) {
        unlisted++;
        console.log(`${n}: refused, not in ${refusalsFile}: ${w}`);
      }
    }
    for (const n of listed.keys()) if (!refused.has(n)) console.log(`${n}: stale entry in ${refusalsFile} (no longer refused)`);
  }
}
console.log(`plain-node: ${links} links in ${cases} cases, ${bad} differ; ${refused.size} cases refused${xfails.length ? `; ${xfails.filter(([, x]) => x.differs || x.node).length} of ${xfails.length} plain-xfail cases differ as marked` : ""}${unlisted ? ` (${unlisted} not listed)` : ""}${tsLinks ? `; ${tsLinks} TypeScript links (not only types) not run` : ""}`);
process.exit(bad || unlisted ? 1 : 0);
