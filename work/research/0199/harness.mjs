// harness.mjs plain.js: runs a plain dump as an ordinary Script in node, with the host
// functions `djs run` provides: print / console.log (sync), later(v, rejected) (async: the
// oldest open call is answered at each idle point, i.e. once the microtask queue is empty).
import vm from "node:vm";
import fs from "node:fs";
import util from "node:util";

const file = process.argv[2];
const text = fs.readFileSync(file, "utf8");
const out = (s) => process.stdout.write(s + "\n");
const display = (v) => (typeof v === "string" ? v : v instanceof Error ? (v.message ? `${v.name}: ${v.message}` : v.name) : util.inspect(v, { depth: 4 }));
const open = [];
let issued = 0;
globalThis.later = (v, rejected) => new Promise((res, rej) => open.push({ id: ++issued, res, rej, v, rejected }));
globalThis.print = (...a) => out(a.map(display).join(" "));
globalThis.console = { log: globalThis.print };
// like the CLI's: a tool is an async host function; without answers its calls stay open
const made = Object.create(null);
globalThis.tools = new Proxy(made, { get: (t, k) => (typeof k !== "string" || k === "then" ? undefined : (t[k] ??= () => new Promise(() => {}))) });
const unhandled = new Map();
process.on("unhandledRejection", (r, p) => unhandled.set(p, r));
process.on("rejectionHandled", (p) => unhandled.delete(p));
try {
  new vm.Script(text, { filename: file }).runInThisContext();
} catch (e) {
  out(`Uncaught ${display(e)}`);
  process.exit(1);
}
for (let n = 0; n < 100000; n++) {
  await new Promise((r) => setImmediate(r));
  if (!open.length) break;
  const c = open.shift();
  c.rejected ? c.rej(c.v) : c.res(c.v);
}
await new Promise((r) => setImmediate(r));
for (const r of unhandled.values()) out(`Possibly unhandled promise rejection: ${display(r)}`);
