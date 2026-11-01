// qjs-wasm-run.mjs <workload>...: the same bodies in quickjs-wasi (pi's engine; read-only
// import from pi-private's node_modules), median of 5 after a warm-up.
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
const here = dirname(fileURLToPath(import.meta.url));
const pkg = join(process.env.HOME, "Development/pi-private/node_modules/quickjs-wasi");
const { QuickJS } = await import(join(pkg, "dist/index.js"));
const wasm = await WebAssembly.compile(readFileSync(join(pkg, "quickjs.wasm")));
const reps = Number(process.env.REPS ?? 5);
const once = process.env.ONCE === "1";
for (const name of process.argv.slice(2)) {
  const code = readFileSync(name.endsWith(".js") ? name : join(here, "w", name + ".js"), "utf8");
  const times = [];
  for (let i = 0; i < (once ? 1 : reps + 1); i++) {
    const vm = await QuickJS.create({ wasm });
    const t = performance.now();
    const r = vm.evalCode('"use strict"; (() => {' + code + "\n})()");
    const dt = performance.now() - t;
    if (i === 0) { try { console.error(name, "=", r.toNumber ? r.toNumber() : r); } catch {} }
    r.dispose?.(); vm.dispose?.();
    if (i > 0 || once) times.push(dt);
  }
  times.sort((a, b) => a - b);
  console.log(`${name}\t${times[times.length >> 1].toFixed(1)} ms\t(min ${times[0].toFixed(1)}, max ${times[times.length - 1].toFixed(1)})`);
}
