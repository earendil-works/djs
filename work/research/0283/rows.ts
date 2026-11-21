// rows.ts MODULE.wasm: idle memory, then each codemode workload's memory (first, warm) and a
// hash of its output.
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
const W = "/Users/mitsuhiko/Development/djs/.worktrees/pstart";
const { record } = await import(`${W}/tests/bench/codemode/tools.ts`);
const { bodyProgram, djs } = await import(`${W}/js/bench/engines/engines.ts`);
const wasm = process.argv[2];
const kib = (b: number) => b / 1024;
{
  const e = djs(wasm); await e.load();
  const a = e.create({}); const m1 = kib(a.memory()); a.dispose();
  const b = e.create({}); const m2 = kib(b.memory()); b.dispose();
  console.log(`idle\t${m1}\t${m2}`);
}
for (const name of ["sequential", "fanout", "media", "inspect", "loop", "text", "json", "heavy"]) {
  const src = readFileSync(`${W}/tests/bench/codemode/${name}.js`, "utf8");
  const data = JSON.stringify(await record(src));
  const program = bodyProgram(src);
  const e = djs(wasm); await e.load();
  const first = e.create({}); const out = first.run(program, data); const a = kib(first.memory()); first.dispose();
  const warm = e.create({}); warm.run(program, data); const b = kib(warm.memory()); warm.dispose();
  console.log(`${name}\t${a}\t${b}\t${createHash("sha1").update(out).digest("hex").slice(0, 10)}`);
}
