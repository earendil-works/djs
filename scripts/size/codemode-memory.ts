// The wasm memory of an idle VM and of each codemode workload (tests/bench/codemode) after its run, in a first
// instance of the module (which decodes the base) and in a later one (which gets it
// injected), against tests/scores/codemode-memory.tsv (D3367). Memory is a deterministic
// but chaotic function of where the allocator places the arena (D2047): a run whose arena
// had to move past a block placed behind it costs hundreds of KiB more, which is what this
// catches, e.g. a compile whose peak outgrew dlmalloc's room in front of the arena
// (D3362, D3366). An increase fails; a decrease is reported, for --update.
//   node scripts/size/codemode-memory.ts MODULE.wasm --check FILE [--update]
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { record } from "../../tests/bench/codemode/tools.ts";
import { bodyProgram, djs } from "../../js/bench/engines/engines.ts";

const args = process.argv.slice(2);
const wasm = args[0];
const file = args[args.indexOf("--check") + 1];
const update = args.includes("--update");
if (!wasm || !args.includes("--check")) throw new Error("usage: codemode-memory.ts MODULE.wasm --check FILE [--update]");
const root = join(import.meta.dirname, "../..");
const kib = (b: number) => b / 1024;

const rows: [string, number, number][] = [];
// Idle: a new VM that ran nothing, the arena at the base plus the per-VM cells (0271).
{
  const e = djs(wasm);
  await e.load();
  const first = e.create({});
  const a = kib(first.memory());
  first.dispose();
  const warm = e.create({});
  const b = kib(warm.memory());
  warm.dispose();
  rows.push(["idle", a, b]);
}
// js/bench/engines/run.ts WORKLOADS, but ts (its body is TypeScript, compiled by the host).
for (const name of ["sequential", "fanout", "media", "inspect", "loop", "text", "json", "heavy"]) {
  const src = readFileSync(join(root, "tests/bench/codemode", `${name}.js`), "utf8");
  const data = JSON.stringify(await record(src));
  const program = bodyProgram(src);
  // A fresh module: its first instance decodes the base.
  const e = djs(wasm);
  await e.load();
  const first = e.create({});
  first.run(program, data);
  const a = kib(first.memory());
  first.dispose();
  const warm = e.create({});
  warm.run(program, data);
  const b = kib(warm.memory());
  warm.dispose();
  rows.push([name, a, b]);
}

const text = `# workload\tfirst KiB\twarm KiB (scripts/size/codemode-memory.ts, D3367)\n${rows.map((r) => r.join("\t")).join("\n")}\n`;
if (update) {
  writeFileSync(file, text);
  console.log(`codemode-memory: wrote ${file}`);
  process.exit(0);
}
const want = new Map<string, [number, number]>();
for (const line of readFileSync(file, "utf8").split("\n")) {
  if (!line || line.startsWith("#")) continue;
  const [n, a, b] = line.split("\t");
  want.set(n, [Number(a), Number(b)]);
}
let worse = 0;
let better = 0;
for (const [n, a, b] of rows) {
  const w = want.get(n);
  if (!w) {
    console.log(`codemode-memory: ${n} is not in ${file} (--update)`);
    worse++;
    continue;
  }
  for (const [kind, got, exp] of [["first", a, w[0]], ["warm", b, w[1]]] as const) {
    if (got > exp) {
      console.log(`codemode-memory: ${n} (${kind} instance) ${got} KiB, recorded ${exp} KiB`);
      worse++;
    } else if (got < exp) {
      console.log(`codemode-memory: ${n} (${kind} instance) ${got} KiB, recorded ${exp} KiB: lower, --update`);
      better++;
    }
  }
}
console.log(`codemode-memory: ${worse ? `${worse} rows above the record` : "OK"}${better ? `, ${better} below (stale)` : ""}`);
process.exit(worse ? 1 : 0);
