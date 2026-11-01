// Polls (shouldInterrupt calls) and heap size of the same deterministic pi run: cold start
// (prelude runs) vs warm start (prelude image reused), as pi's inline launcher counts them.
import { loadModule } from "../../js/src/sandbox.ts";
import { startVm } from "../../js/pi/vm.ts";
import { PRELUDE_SOURCE } from "../../tests/pi/prelude-source.ts";
const wasm = await loadModule(process.argv[2]);
const code = `let s = 0; for (let i = 0; i < 300000; i++) s += i; const r = await tools.echo({ s }); console.log(Math.random(), Date.now(), r); return s;`;
async function once(label: string, reuseImages: boolean) {
  let polls = 0;
  const msgs: any[] = [];
  let session: any;
  const pollsAt: number[] = [];
  session = await startVm({
    data: { code, tools: [{ name: "echo", jsName: "echo", description: "" }], globals: [], memoryLimitBytes: 8 << 20, store: {}, deterministic: { startedAt: 1_700_000_000_000, seed: 42 } },
    wasm,
    prelude: PRELUDE_SOURCE,
    reuseImages,
    post: (m) => { msgs.push(m); if (m.type === "started") pollsAt.push(polls); },
    shouldInterrupt: () => { polls++; return false; },
  });
  const call = msgs.find((m) => m.type === "call");
  const before = polls;
  session.deliver({ type: "result", id: call.id, ok: true, payload: JSON.stringify({ ok: 1 }), seq: 1, now: 1_700_000_000_500 });
  const done = msgs.find((m) => m.type === "done");
  const out = msgs.filter((m) => m.type === "output").map((m) => m.item.text).join(" ");
  console.log(`${label}: polls before 'started' ${pollsAt[0]}, after start ${before}, total ${polls}; output ${out}; done ${JSON.stringify(done?.value)}`);
  session.dispose();
}
await once("cold (no image)", false);
await once("cold (fills cache)", true);
await once("warm (image reused)", true);
