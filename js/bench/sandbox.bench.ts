// Startup and round-trip timings of the Sandbox on the `wasm` profile build
// (scripts/build-wasm.sh first, or set DJS_WASM).
//
//   node js/bench/sandbox.bench.ts

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { Sandbox, defaultWasmPath, loadModule } from "../src/sandbox.ts";

function median(f: () => void, n = 200): number {
  for (let i = 0; i < 10; i++) f();
  const runs: number[] = [];
  for (let i = 0; i < n; i++) {
    const t = performance.now();
    f();
    runs.push(performance.now() - t);
  }
  runs.sort((a, b) => a - b);
  return runs[runs.length >> 1] * 1000; // us
}

async function medianAsync(f: () => Promise<void>, n = 200): Promise<number> {
  for (let i = 0; i < 10; i++) await f();
  const runs: number[] = [];
  for (let i = 0; i < n; i++) {
    const t = performance.now();
    await f();
    runs.push(performance.now() - t);
  }
  runs.sort((a, b) => a - b);
  return runs[runs.length >> 1] * 1000;
}

const path = defaultWasmPath();
const bytes = readFileSync(typeof path === "string" ? path : fileURLToPath(path));
console.log(`module: ${bytes.length} bytes (${path})`);

let t = performance.now();
const module = await WebAssembly.compile(bytes);
console.log(`WebAssembly.compile: ${(performance.now() - t).toFixed(2)} ms (once per process)`);
await loadModule();

const inst = await WebAssembly.instantiate(module, {});
const x = inst.exports as unknown as { vm_new(n: number): number; vm_free(vm: number): void };
console.log(`instantiate: ${(await medianAsync(async () => void (await WebAssembly.instantiate(module, {})))).toFixed(1)} us`);
console.log(`vm_new: ${median(() => x.vm_free(x.vm_new(0))).toFixed(1)} us (realm built from scratch; no base snapshot yet)`);
console.log(`Sandbox.create: ${(await medianAsync(async () => (await Sandbox.create({ module })).dispose())).toFixed(1)} us`);
const sb = await Sandbox.create({ module });
console.log(`run("return 1") (compile + call + run): ${median(() => void sb.run("return 1")).toFixed(1)} us`);
const first = await medianAsync(async () => {
  const s = await Sandbox.create({ module });
  s.run("return 1");
  s.dispose();
}, 100);
console.log(`Sandbox.create + first run: ${first.toFixed(1)} us`);
const script = "let total = 0; for (let i = 0; i < data.items.length; i++) total += data.items[i].price; console.log('n', data.items.length); return {total, n: data.items.length}";
const data = { items: Array.from({ length: 1000 }, (_, i) => ({ id: i, name: `item ${i}`, price: i * 0.5 })) };
const quiet = await Sandbox.create({ module, console: { log() {}, info() {}, warn() {}, error() {} } });
console.log(`1000-item tool result in, loop, object out: ${median(() => void quiet.run(script, { data }), 100).toFixed(1)} us`);
