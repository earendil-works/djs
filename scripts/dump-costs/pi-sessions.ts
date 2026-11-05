// pi's adapter in next mode with a dump and a resume in a new instance at every idle point
// (D2651), N sessions of a small script, for profiling the wasm side as pi pays it (0215):
//
//   DJS_WASM=target/djs-dump.wasm N=300 node --cpu-prof scripts/dump-costs/pi-sessions.ts
//
// work/research/0215/pi-phases.patch adds per-phase timings to js/pi (PI_PROF_OUT=file).
import { loadModule } from "../../js/src/sandbox.ts";
import { startVm, type VmStartData, type WorkerToHostMessage } from "../../js/pi/vm.ts";
import { PRELUDE_NEXT } from "../../tests/pi/prelude-next.ts";
const wasm = await loadModule();
const STARTED_AT = Date.UTC(2026, 0, 15, 10, 0, 0);
const code = "let n = 0; for (let i = 0; i < 3; i++) n += await tools.echo(i); text('n ' + n); return n;";
const data: VmStartData = { code, tools: [{ name: "echo", jsName: "echo", description: "" }], globals: [], memoryLimitBytes: undefined, store: {}, deterministic: { startedAt: STARTED_AT, seed: 7 } };
const N = Number(process.env.N ?? 200);
const t0 = performance.now();
for (let k = 0; k < N; k++) {
  let session: any; let seq = 0;
  let end!: (m: WorkerToHostMessage) => void;
  const ended = new Promise<WorkerToHostMessage>((r) => (end = r));
  const hooks = { wasm, shape: "next" as const, shouldInterrupt: () => false, post: (m: WorkerToHostMessage) => { setTimeout(() => {
    if (m.type === "call") { seq++; session.deliver({ type: "result", id: m.id, ok: true, payload: JSON.stringify(seq), seq, now: STARTED_AT + 1000 * seq }); }
    if (m.type === "done" || m.type === "crash") end(m);
  }); } };
  session = await startVm({ ...hooks, dumpEveryIdle: true, prelude: PRELUDE_NEXT, data });
  const m = await ended; if (m.type !== "done") throw new Error(JSON.stringify(m));
  session.dispose?.();
}
console.log(`${N} sessions ${(performance.now() - t0).toFixed(0)} ms`);
