// wasm-run.ts <workload|file.js>...: runs scripts/perf/w/<workload>.js as a codemode body
// in djs-wasm ($DJS_WASM, default the setup.sh build) through the Sandbox. Prints the
// median of REPS (5) runs after a warm-up in ms, and the wasm memory size; ONCE=1 runs
// once (for instruction counts); FUEL is the fuel per vm_run (default 10,000, pi's
// FUEL_PER_POLL: V8 has no OSR for wasm, so one huge slice keeps run_loop in Liftoff).
// With node's --cpu-prof it profiles.
//   node scripts/perf/wasm-run.ts loop forof
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { Sandbox, loadModule } from "../../js/src/sandbox.ts";

const here = dirname(fileURLToPath(import.meta.url));
const out = process.env.DJS_PERF_DIR ?? join(here, "../../work/scratch/perf");
const wasm = process.env.DJS_WASM ?? join(out, "target/wasm32-unknown-unknown/wasm/djs_wasm.wasm");
const module = await loadModule(wasm);
const reps = Number(process.env.REPS ?? 5);
const once = process.env.ONCE === "1";
for (const name of process.argv.slice(2)) {
  const code = readFileSync(name.endsWith(".js") ? name : join(here, "w", name + ".js"), "utf8");
  const times: number[] = [];
  let mem = 0;
  for (let i = 0; i < (once ? 1 : reps + 1); i++) {
    const sb = new Sandbox({ module, fuelSlice: Number(process.env.FUEL ?? 10000) } as never);
    const t = performance.now();
    sb.run(code);
    const dt = performance.now() - t;
    mem = (sb as any).x.memory.buffer.byteLength;
    sb.dispose();
    if (i > 0 || once) times.push(dt);
  }
  times.sort((a, b) => a - b);
  console.log(`${name}\t${times[times.length >> 1].toFixed(1)} ms\t(min ${times[0].toFixed(1)}, max ${times[times.length - 1].toFixed(1)})\twasm memory ${(mem / 1048576).toFixed(1)} MiB`);
}
