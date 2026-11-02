// What a fresh Sandbox's wasm memory is made of (0156): pages, the shadow stack, static
// data, the arena (its length and the capacity reserved for it) and the resident memory
// per sandbox (process RSS over N sandboxes, which counts only touched pages).
//   node --expose-gc scripts/size/sandbox-memory.ts [MODULE.wasm] [N]
// The capacity comes from djs_alloc_stats when the module has it (`--features
// alloc-stats`): the largest Rust allocation of a fresh instance is the arena.
import { readFileSync } from "node:fs";
import { Sandbox, defaultWasmPath } from "../../js/src/sandbox.ts";

const path = process.argv[2] ?? defaultWasmPath();
const n = Number(process.argv[3] ?? 200);
const module = await WebAssembly.compile(readFileSync(path));
const KiB = (b: number) => `${(b / 1024).toFixed(0)} KiB`;

// Below the arena: the shadow stack (wasm-ld puts it first, D1941) and the static data;
// `wasm-objdump -x -j Global -j Data` shows both.
const exported = WebAssembly.Module.exports(module).map((e) => e.name);
const first = new Sandbox({ module }); // decodes the base once; later instances get it injected
const probe = new Sandbox({ module });
const x = (probe as any).x;
const vm = (probe as any).vm;
const pages = x.memory.buffer.byteLength;
const arenaPtr = x.vm_arena_ptr(vm);
const arenaLen = x.vm_arena_len(vm);
console.log(`module ${path}`);
console.log(`wasm memory of a fresh sandbox: ${KiB(pages)} (${pages / 65536} pages)`);
console.log(`  arena at ${KiB(arenaPtr)}: length ${KiB(arenaLen)} (base ${KiB(x.djs_base_len())})`);
if (exported.includes("djs_alloc_stats")) {
  const p = x.djs_alloc_stats(0) >> 2;
  const u = new Uint32Array(x.memory.buffer);
  console.log(`  Rust heap live ${KiB(u[p])}, largest allocation ${KiB(u[p + 3])} (the arena's capacity)`);
}
await probe.run("return 1");
void first;

const keep: Sandbox[] = [];
(globalThis as any).gc?.();
const r0 = process.memoryUsage().rss;
for (let i = 0; i < n; i++) {
  const s = new Sandbox({ module });
  await s.run("return 1");
  keep.push(s);
}
(globalThis as any).gc?.();
const r1 = process.memoryUsage().rss;
console.log(`resident per sandbox (RSS over ${n}): ${KiB((r1 - r0) / n)}`);
