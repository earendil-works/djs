// A session suspended on djs build N, restored on build N+1 (0140, docs/snapshot-versioning.md).
// N is an older checkout with its djs-wasm built, N+1 this tree's (scripts/build-wasm.sh).
// Each side uses its own host code (js/) and its own module, as two deployed versions would.
//
//   git archive OLD_COMMIT | tar -x -C OLD && (cd OLD && scripts/build-wasm.sh)
//   scripts/build-wasm.sh && node scripts/snapshot-research/upgrade.ts OLD [OUT]
//
// OUT (default work/scratch/snapshot-versioning) gets both base images, pi's prelude images,
// the snapshots and the session's image, for `djs heap`.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import * as hostM from "../../js/src/sandbox.ts";
import * as snapM from "../../js/src/snapshot.ts";
import * as piM from "../../js/pi/vm.ts";
import { PRELUDE_SOURCE } from "../../tests/pi/prelude-source.ts";

const old = resolve(process.argv[2] ?? "");
if (!process.argv[2]) throw new Error("usage: node scripts/snapshot-research/upgrade.ts OLD_TREE [OUT]");
const out = process.argv[3] ?? "work/scratch/snapshot-versioning";
mkdirSync(out, { recursive: true });
const load = (p: string) => import(pathToFileURL(`${old}/${p}`).href);
const hostN = (await load("js/src/sandbox.ts")) as typeof hostM;
const snapN = (await load("js/src/snapshot.ts")) as typeof snapM;
const piN = (await load("js/pi/vm.ts")) as typeof piM;
const modN = await WebAssembly.compile(readFileSync(`${old}/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm`));
const modM = await WebAssembly.compile(readFileSync("target/wasm32-unknown-unknown/wasm/djs_wasm.wasm"));
const log = (...a: unknown[]) => console.log(...a);
const hex = (n: number) => (n >>> 0).toString(16).padStart(8, "0");
const facts = (mod: WebAssembly.Module, snap: typeof snapN) => {
  const x = new WebAssembly.Instance(mod, {}).exports as any;
  const ref = snap.baseReference(mod, x)!;
  return { build: hex(x.djs_build_hash()), base: ref.hash, baseBytes: ref.bytes };
};
const fN = facts(modN, snapN);
const fM = facts(modM, snapM);
writeFileSync(`${out}/base-n.img`, fN.baseBytes);
writeFileSync(`${out}/base-m.img`, fM.baseBytes);
log(`build N   ${fN.build}, base image ${fN.base} (${fN.baseBytes.length} bytes)`);
log(`build N+1 ${fM.build}, base image ${fM.base} (${fM.baseBytes.length} bytes)`);

// ---- The Sandbox: state kept between runs ------------------------------------------------
log("\n== Sandbox");
const sbN = new hostN.Sandbox({ module: modN });
sbN.run("globalThis.notes = Array.from({ length: 100 }, (_, i) => ({ i, text: 'note ' + i })); globalThis.count = 0; return notes.length");
const sbSnap = sbN.bytes();
writeFileSync(`${out}/sandbox-n.djsn`, sbSnap);
log(`snapshot on N: ${sbSnap.length} bytes`);
try {
  new hostM.Sandbox({ module: modM }, sbSnap);
  log("restored on N+1 (unexpected)");
} catch (e) {
  log(`restore on N+1: ${(e as Error).name} ${(e as any).kind}: ${(e as Error).message}`);
}
const sbBack = new hostN.Sandbox({ module: modN }, sbSnap);
log(`restore on N (engine registry): ${sbBack.run("count++; return notes[99].text + ', count ' + count")}`);

// Journals: a deterministic run recorded on N, replayed on N+1.
const det = { startedAt: Date.UTC(2026, 0, 1), seed: 7 };
const lookup = (k: unknown) => ({ key: k, owner: "team-" + String(k).length });
const code = "const a = await lookup('alpha'); const b = await lookup(a.owner); return [a.owner, b.owner, Math.random() < 2, new Date().toISOString()];";
const jN: hostM.JournalEntry[] = [];
const recN = new hostN.Sandbox({ module: modN, deterministic: det, now: () => det.startedAt + 1000, asyncFunctions: { lookup } });
const valueN = await recN.runAsync(code, {}, { journal: jN });
log(`recorded on N: ${JSON.stringify(valueN)}; journal of ${jN.length} entries, ${JSON.stringify(jN).length} bytes`);
const repM = new hostM.Sandbox({ module: modM, deterministic: det, asyncFunctions: { lookup } });
try {
  await repM.replay(jN as hostM.JournalEntry[]);
  log("Sandbox.replay on N+1: same result");
} catch (e) {
  log(`Sandbox.replay on N+1: ${(e as any).kind}: ${(e as Error).message}`);
}
// A replay that checks what a script observably does (calls, arguments, result) and not
// the heap's canonical hash, which differs between builds by construction.
const asyncs = jN.filter((e) => e.type === "async") as Extract<hostM.JournalEntry, { type: "async" }>[];
let next = 0;
const calls: string[] = [];
const replayed = new hostM.Sandbox({
  module: modM,
  deterministic: det,
  now: () => asyncs[Math.min(next, asyncs.length - 1)]?.now ?? det.startedAt,
  asyncFunctions: {
    lookup: (...args: unknown[]) => {
      const e = asyncs[next++];
      calls.push(JSON.stringify(args));
      if (e === undefined || e.fn !== "lookup" || JSON.stringify(e.args) !== JSON.stringify(args)) throw new Error("diverged");
      return e.value;
    },
  },
});
const valueM = await replayed.runAsync(code, {});
log(`semantic replay on N+1: ${JSON.stringify(valueM)}; same as N: ${JSON.stringify(valueM) === JSON.stringify(valueN)}; calls ${calls.join(" ")}`);

// ---- pi: a session waiting on a tool ---------------------------------------------------
log("\n== pi");
const askCode = readFileSync("scripts/snapshot-research/cases/ask.js", "utf8");
const data = {
  code: askCode,
  tools: ["ask", "shell"].map((name) => ({ name, jsName: name, description: `The ${name} tool.` })),
  globals: [],
  memoryLimitBytes: undefined,
  store: {},
  deterministic: { startedAt: 1_762_000_000_000, seed: 42 },
};
writeFileSync(`${out}/start.json`, JSON.stringify(data));
const answer = (name: string) => (name === "ask" ? { choice: "yes" } : { code: 0, stdout: "building\nlinking\ndeployed to staging" });

type Msg = { type: string; [k: string]: unknown };
/** Runs the session on `pi`/`mod`, snapshotting at call `snapAt` and continuing on `resume`. */
async function piRun(
  pi: typeof piN,
  mod: WebAssembly.Module,
  opts: { snapAt?: number; resume?: (bytes: Uint8Array, hooks: any) => any; journal?: { seq: number; now: number; id: number; payload: string }[] } = {},
) {
  const messages: Msg[] = [];
  let session: any;
  let seq = 0;
  let snapshot: Uint8Array | undefined;
  let error: string | undefined;
  const done = new Promise<void>((resolve) => {
    const hooks: any = {
      wasm: mod,
      prelude: PRELUDE_SOURCE,
      data,
      shouldInterrupt: () => false,
      post: (m: Msg) => {
        messages.push(m);
        if (m.type === "done" || m.type === "crash") setTimeout(resolve);
        if (m.type !== "call") return;
        setTimeout(() => {
          seq++;
          if (seq === opts.snapAt) {
            snapshot = session.snapshot();
            session.dispose();
            try {
              session = opts.resume!(snapshot!, hooks);
            } catch (e) {
              error = `${(e as Error).name}: ${(e as Error).message}`;
              resolve();
              return;
            }
          }
          const payload = JSON.stringify(answer(m.name as string));
          const now = 1_762_000_000_000 + 1000 * seq;
          opts.journal?.push({ seq, now, id: m.id as number, payload });
          session.deliver({ type: "result", id: m.id, ok: true, payload, seq, now });
        });
      },
    };
    pi.startVm(hooks).then((s: any) => (session = s));
  });
  await done;
  session?.dispose();
  return { messages, snapshot, error };
}
const plain = (ms: Msg[]) => ms.filter((m) => m.type !== "started");
const journal: { seq: number; now: number; id: number; payload: string }[] = [];
const uninterrupted = await piRun(piN, modN, { journal });
log(`N, uninterrupted: ${uninterrupted.messages.length} messages; done ${JSON.stringify(uninterrupted.messages.find((m) => m.type === "done"))}`);
log(`  fingerprint N:   ${JSON.stringify((uninterrupted.messages[0] as any).fingerprint)}`);
// Suspended on N at the second call (shell), restored on N+1.
const onM = await piRun(piN, modN, { snapAt: 2, resume: (b, hooks) => piM.resumeVm(b, { ...hooks, wasm: modM }) });
log(`N -> N+1 resumeVm: ${onM.error}`);
writeFileSync(`${out}/pi-n.djsn`, onM.snapshot!);
log(`  the snapshot: ${onM.snapshot!.length} bytes`);
// The engine registry: the same snapshot, restored with build N's host and module.
const onN = await piRun(piN, modN, { snapAt: 2, resume: (b, hooks) => piN.resumeVm(b, hooks) });
log(`N -> N (registry): ${JSON.stringify(plain(onN.messages)) === JSON.stringify(plain(uninterrupted.messages)) ? "same messages as the uninterrupted run" : "DIFFERENT"}`);
// Replay on N+1: the same start data and the journaled results, in order.
let t = performance.now();
const replayM = await piRun(piM, modM);
const replayMs = performance.now() - t;
const same = JSON.stringify(plain(replayM.messages)) === JSON.stringify(plain(uninterrupted.messages));
log(`replay on N+1: ${same ? "same calls, arguments, output and result as on N" : "DIFFERENT"} (${replayMs.toFixed(1)} ms)`);
log(`  fingerprint N+1: ${JSON.stringify((replayM.messages[0] as any).fingerprint)}`);
log(`  journal: ${journal.length} results, ${JSON.stringify(journal).length} bytes`);

// The decompressed images, for djs heap.
const prelN = piN.preludeImage(modN, PRELUDE_SOURCE, data as any);
const prelM = piM.preludeImage(modM, PRELUDE_SOURCE, data as any);
writeFileSync(`${out}/prelude-n.img`, prelN);
writeFileSync(`${out}/prelude-m.img`, prelM);
const h = snapN.readSnapshotHeader(onM.snapshot!)!;
const img = snapN.snapshotImage(h, { kind: snapN.REFERENCE_PI_PRELUDE, bytes: prelN, hash: snapN.fnv1a64(prelN) }, undefined);
writeFileSync(`${out}/pi-n.img`, img);
log(`pi prelude image N ${prelN.length} bytes, N+1 ${prelM.length} bytes; session image N ${img.length} bytes`);
