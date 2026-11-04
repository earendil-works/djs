#!/usr/bin/env node
// plain-node.mjs DIR [--list]: runs the plain dumps `djs-difftest --plain-out DIR` wrote
// in node and checks that each continues exactly like the uninterrupted djs run (D2864).
//
// DIR/<case>/<k>.js is link k's dump, <k>.pre the output before it and `transcript` the
// djs run's whole transcript. Each dump runs as a Script in a fresh node context with
// the difftest prelude's host: output(text), done(kind, text), later(v, rejected) (the
// oldest open call is answered at each idle point, when the microtask queue is empty)
// and the plain-dump hooks $djs.seed / $djs.resume. The transcript it produces, after
// <k>.pre, must equal `transcript`.
//
// Exit status 1 if any link differs; one line per differing link, and a summary.
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";

process.env.TZ = "UTC"; // as the difftest runs djs and qjs

const dir = process.argv[2];
const list = process.argv.includes("--list");
if (!dir) {
  console.error("usage: plain-node.mjs DIR [--list]");
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
    value: {
      // node's Math.random cannot be seeded: cases that use it are not comparable.
      seed() {},
      resume: (id, name, args) => g.later(...args),
    },
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
for (const c of walk(dir).sort()) {
  const want = norm(fs.readFileSync(path.join(c, "transcript"), "utf8"));
  const ks = fs
    .readdirSync(c)
    .filter((f) => /^\d+\.js$/.test(f))
    .map((f) => parseInt(f))
    .sort((a, b) => a - b);
  if (!ks.length) continue;
  cases++;
  for (const k of ks) {
    const file = path.join(c, `${k}.js`);
    const head = fs.readFileSync(file, "utf8").slice(0, 40);
    if (head.startsWith('"djs-dump 3 typescript"')) continue;
    links++;
    const pre = fs.readFileSync(path.join(c, `${k}.pre`), "utf8");
    const now = fs.existsSync(path.join(c, "now")) ? Number(fs.readFileSync(path.join(c, "now"), "utf8")) : Date.now();
    const got = norm(pre + (await runLink(file, now)));
    if (got !== want) {
      bad++;
      const a = want.split("\n");
      const b = got.split("\n");
      let i = 0;
      while (i < a.length && a[i] === b[i]) i++;
      const name = path.relative(dir, c);
      console.log(`${name} link ${k}: line ${i + 1}: expected ${JSON.stringify(a[i] ?? "")}, got ${JSON.stringify(b[i] ?? "")}`);
    } else if (list) {
      console.log(`${path.relative(dir, c)} link ${k}: ok`);
    }
  }
}
console.log(`plain-node: ${links} links in ${cases} cases, ${bad} differ`);
process.exit(bad ? 1 : 0);
