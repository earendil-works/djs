// icount.ts MODULE.wasm NAME REPS: REPS runs of a codemode workload, each in a new sandbox of one instance (for /usr/bin/time -l).
import { readFileSync } from "node:fs";
const W = "/Users/mitsuhiko/Development/djs/.worktrees/pstart";
const { record } = await import(`${W}/tests/bench/codemode/tools.ts`);
const { bodyProgram, djs } = await import(`${W}/js/bench/engines/engines.ts`);
const [wasm, name, reps] = process.argv.slice(2);
const e = djs(wasm); await e.load();
if (name === "idle") { for (let i = 0; i < Number(reps); i++) { const v = e.create({}); v.dispose(); } process.exit(0); }
const src = readFileSync(`${W}/tests/bench/codemode/${name}.js`, "utf8");
const data = JSON.stringify(await record(src)); const prog = bodyProgram(src);
for (let i = 0; i < Number(reps); i++) { const v = e.create({}); v.run(prog, data); v.dispose(); }
