// Heap bytes per object (0223): N objects of one kind held in an array, after a collection,
// minus the same array holding numbers, divided by N. Shape mode against dictionary mode
// (forced with an attribute change, which turns an object into a private table):
//   node scripts/size/object-bytes.ts [MODULE.wasm] [N]
// Prints `name<TAB>bytes per object`; scripts/status.py heap records them.
import { readFileSync } from "node:fs";
import { Sandbox, defaultWasmPath } from "../../js/src/sandbox.ts";

const path = process.argv[2] ?? defaultWasmPath();
const n = Number(process.argv[3] ?? 100_000);
const module = await WebAssembly.compile(readFileSync(path));

// `make(i)` builds object i; the array is allocated first, at its final length.
const cases: [string, string][] = [
  ["literal4/shape", "({ a: i, b: i, c: i, d: i })"],
  ["literal4/dictionary", "Object.defineProperty({ a: i, b: i, c: i, d: i }, 'a', { writable: false })"],
  ["json5/shape", "JSON.parse('{\"id\":' + i + ',\"name\":\"n\",\"ok\":true,\"score\":1.5,\"tag\":null}')"],
  [
    "json5/dictionary",
    "Object.defineProperty(JSON.parse('{\"id\":' + i + ',\"name\":\"n\",\"ok\":true,\"score\":1.5,\"tag\":null}'), 'id', { writable: false })",
  ],
];

function heapAfter(make: string): number {
  const sb = new Sandbox({ module, memoryLimit: 1 << 30 });
  sb.run(`globalThis.xs = new Array(${n}).fill(0); for (let i = 0; i < ${n}; i++) xs[i] = ${make};`);
  sb.collect();
  const size = sb.heapSize;
  sb.dispose();
  return size;
}

const base = heapAfter("i");
for (const [name, make] of cases) {
  console.log(`${name}\t${((heapAfter(make) - base) / n).toFixed(1)}`);
}
