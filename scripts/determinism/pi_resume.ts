// pi adapter, deterministic mode: the same run uninterrupted vs resumed from a snapshot at every
// tool call (resumeVm). Compares outputs and heap sizes (Engine.heapSize via the session image).
import { loadModule } from "../../js/src/sandbox.ts";
import { startVm, resumeVm } from "../../js/pi/vm.ts";
import { PRELUDE_SOURCE } from "../../tests/pi/prelude-source.ts";
const wasm = await loadModule(process.argv[2]);
const code = `let keep = [];
for (let round = 0; round < 30; round++) {
  const r = await tools.echo({ round });
  let junk = [];
  for (let i = 0; i < 20000; i++) junk.push({ i, s: "x" + i });
  keep.push(junk.length + r.round);
}
console.log(keep.reduce((a, b) => a + b, 0));`;
async function run(resume: boolean) {
  const msgs: any[] = [];
  const hooks = { wasm, post: (m: any) => msgs.push(m), shouldInterrupt: () => false };
  let session: any = await startVm({
    data: { code, tools: [{ name: "echo", jsName: "echo", description: "" }], globals: [], memoryLimitBytes: 64 << 20, store: {}, deterministic: { startedAt: 1_700_000_000_000, seed: 1 } },
    prelude: PRELUDE_SOURCE, reuseImages: false, ...hooks,
  });
  const sizes: number[] = [];
  for (let seq = 1; ; seq++) {
    const calls = msgs.filter((m) => m.type === "call");
    if (calls.length < seq) break;
    const call = calls[seq - 1];
    if (resume) {
      const snap = session.snapshot();
      sizes.push(snap.length);
      session.dispose();
      session = resumeVm(snap, hooks);
    } else sizes.push(session.snapshot().length);
    session.deliver({ type: "result", id: call.id, ok: true, payload: JSON.stringify(JSON.parse(call.args)), seq, now: 1_700_000_000_000 + seq });
  }
  const out = msgs.filter((m) => m.type === "output").map((m) => m.item.text).join(" ");
  console.log(`${resume ? "resumed at every call" : "uninterrupted       "}: output ${out}; image bytes at calls 1/10/20/30: ${[0, 9, 19, 29].map((i) => sizes[i]).join(" / ")}`);
}
await run(false);
await run(true);
