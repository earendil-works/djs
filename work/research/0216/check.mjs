// Runs plain dumps in node (0216) with a generic host and prints each one's transcript, so
// that a rewritten dump can be compared with main's: the same host answers both the same way.
//   node work/research/0216/check.mjs A.js B.js        transcripts of both, and whether equal
//   node work/research/0216/check.mjs --dirs A B       every dump in A against the same path in B
// Host: `later(v)` / `$djs.resume(id, name, args)` answer at the next idle point, the oldest
// first (`later` with its argument, other calls with `name(args)` as a string); `print`,
// `console.log`, `output` print; `$djs.host(name)` returns the global of that path, or a stub
// that records what is done with it; unknown globals are stubs too. The runtime is
// js/src/dump-runtime.js with `k` taking both the record object (main) and positional
// arguments (minimal), and the host deletes `$djs`/`$dump`/`$d` after the Script.
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";

const RUNTIME = `(() => {\n${fs
  .readFileSync(new URL("../../../js/src/dump-runtime.js", import.meta.url), "utf8")
  .replace(/^export const /gm, "const ")}\nreturn helpers;\n})()`;

const show = (v, seen = new Set()) => {
  if (v === null || typeof v !== "object" && typeof v !== "function") return typeof v === "string" ? JSON.stringify(v) : String(v);
  if (seen.has(v)) return "<cycle>";
  seen.add(v);
  if (typeof v === "function") return `<fn ${v.name}>`;
  if (Array.isArray(v)) return `[${v.map((x) => show(x, seen)).join(",")}]`;
  if (v instanceof Map) return `Map{${[...v].map(([a, b]) => show(a, seen) + "=>" + show(b, seen)).join(",")}}`;
  if (typeof v.then === "function") return "<promise>";
  return `{${Object.keys(v).map((k) => k + ":" + show(v[k], seen)).join(",")}}`;
};

export async function run(file) {
  const text = fs.readFileSync(file, "utf8");
  const out = [];
  const open = [];
  const stubs = new Map();
  const stub = (name) => {
    if (!stubs.has(name)) {
      const f = function () {
        out.push(`call ${name}(${[...arguments].map((a) => show(a)).join(",")})`);
        return new Promise((res) => open.push({ res, v: `${name}()` }));
      };
      stubs.set(
        name,
        new Proxy(f, {
          get: (t, k) => (k === "then" || typeof k === "symbol" ? undefined : k === "name" ? name : stub(`${name}.${String(k)}`)),
          has: () => true,
          ownKeys: () => ["a", "b"],
          getOwnPropertyDescriptor: (t, k) =>
            k === "prototype" ? Reflect.getOwnPropertyDescriptor(t, k) : { value: stub(`${name}.${String(k)}`), enumerable: true, configurable: true },
        }),
      );
    }
    return stubs.get(name);
  };
  const g0 = {};
  const ctx = vm.createContext(g0);
  const g = vm.runInContext("globalThis", ctx);
  const host = {
    print: (...a) => out.push(a.map((x) => (typeof x === "string" ? x : show(x))).join(" ")),
    output: (t) => out.push(String(t)),
    done: (k, t) => out.push(`done ${k} ${t}`),
    later: (v, rejected) => new Promise((res, rej) => open.push({ res, rej, v, rejected })),
  };
  for (const [k, f] of Object.entries(host)) Object.defineProperty(g, k, { value: f, writable: true, configurable: true });
  vm.runInContext("globalThis.console = { log: (...a) => print(...a) }", ctx);
  const runtime = vm.runInContext(RUNTIME, ctx);
  const k0 = runtime.k;
  const genProto = vm.runInContext("Object.getPrototypeOf(function* () {})", ctx);
  const agenProto = vm.runInContext("Object.getPrototypeOf(async function* () {})", ctx);
  Object.defineProperty(g, "$djs", {
    value: Object.assign(runtime, {
      k: (K, r, a, p) => {
        if (r && typeof r === "object" && "f" in r && typeof r !== "function") return k0(K, r, a);
        // positional: k(K, f, at, proto); proto defaults to f.prototype for generators
        const gen = Object.getPrototypeOf(K) === genProto || Object.getPrototypeOf(K) === agenProto;
        return k0(K, { f: r, at: a }, p ?? (gen && typeof r === "function" ? r.prototype : undefined));
      },
      seed() {},
      resume: (id, name, args) => (name === "later" ? g.later(...args) : stub(name)(...args)),
      source: () => function source() {},
      host: (name) => (name in host ? g[name] : stub(name)),
    }),
    writable: true,
    configurable: true,
  });
  for (const m of new Set(text.match(/\b(models|tools|ALL_TOOLS|call|bridge)\b/g) ?? [])) {
    if (!(m in g)) Object.defineProperty(g, m, { value: stub(m), writable: true, configurable: true });
  }
  let result;
  try {
    result = new vm.Script(text, { filename: file }).runInContext(ctx);
  } catch (e) {
    return out.join("\n") + `\n=== load threw: ${e}`;
  }
  delete g.$djs;
  delete g.$dump;
  delete g.$d;
  let settled = "pending";
  if (result && typeof result.then === "function") {
    result.then(
      (v) => (settled = "fulfilled " + show(v)),
      (e) => (settled = "rejected " + show(e?.message ?? e)),
    );
  } else settled = "value " + show(result);
  for (let n = 0; n < 10000; n++) {
    await new Promise((r) => setImmediate(r));
    if (!open.length) break;
    const c = open.shift();
    c.rejected === true ? c.rej(c.v) : c.res(c.v);
  }
  await new Promise((r) => setImmediate(r));
  return out.join("\n") + `\n=== ${settled}`;
}

const a = process.argv.slice(2);
if (a[0] === "--dirs") {
  const walk = (d) =>
    fs.readdirSync(d, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(d, e.name)) : [path.join(d, e.name)]));
  let n = 0;
  let bad = 0;
  for (const f of walk(a[1]).filter((f) => f.endsWith(".js")).sort()) {
    const g = path.join(a[2], path.relative(a[1], f));
    let [x, y] = [await run(f), await run(g)];
    // No handle: the host reads no completion value (main writes `undefined`).
    if (x.endsWith("\n=== value undefined")) [x, y] = [x.replace(/\n=== [^\n]*$/, ""), y.replace(/\n=== [^\n]*$/, "")];
    n++;
    if (x !== y) {
      bad++;
      console.log(`DIFF ${f}\n--- main\n${x}\n--- rewritten\n${y}`);
    }
    if (/load threw/.test(x)) console.log(`(main itself threw: ${f}: ${x.split("\n").pop()})`);
  }
  console.log(`${n} dumps, ${bad} differ`);
} else if (a.length) {
  const xs = [];
  for (const f of a) {
    const t = await run(f);
    xs.push(t);
    console.log(`--- ${f}\n${t}`);
  }
  if (xs.length === 2 && xs[0].endsWith("\n=== value undefined")) xs[1] = xs[1].replace(/\n=== [^\n]*$/, "\n=== value undefined");
  if (xs.length === 2) console.log(xs[0] === xs[1] ? "SAME" : "DIFFERENT");
}
