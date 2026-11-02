// The snapshot container (D2260): the Sandbox's bytes()/fromBytes() and the pi adapter's
// snapshot()/resumeVm() round trip through it; images are collected and zstd-compressed
// against a reference image (the base snapshot, or pi's prelude image); snapshots of
// another build or reference are refused with a clear error; the formats before the
// container still load.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defaultWasmPath, loadModule, Sandbox } from "../src/sandbox.ts";
import {
  APP_SANDBOX,
  COMPRESSION_NONE,
  COMPRESSION_ZSTD,
  baseReference,
  encodeSnapshot,
  expandSession,
  IMAGE_SESSION,
  readSnapshotHeader,
  REFERENCE_BASE,
  REFERENCE_PI_PRELUDE,
  type SnapshotHeader,
} from "../src/snapshot.ts";
import { resumeVm, startVm, type ResumeOptions, type VmSession, type WorkerToHostMessage } from "../pi/vm.ts";
import type { VmStartData } from "../pi/protocol.ts";
import { PRELUDE_SOURCE } from "../../tests/pi/prelude-source.ts";

const wasm = await loadModule();
const path = defaultWasmPath();
/** A second compile of the same module: a process that has none of this one's prelude images. */
const freshModule = () => WebAssembly.compile(readFileSync(typeof path === "string" ? path : fileURLToPath(path)));

const header = (bytes: Uint8Array): SnapshotHeader => readSnapshotHeader(bytes)!;
/** The module's base image (D2066): what a session image is completed with. */
const base = () => baseReference(wasm, new WebAssembly.Instance(wasm, {}).exports as never)!.bytes;
/** A container's image as a raw image (the formats before the container held those). */
const rawImage = (h: SnapshotHeader) => (h.imageKind === IMAGE_SESSION ? expandSession(h.payload, base()) : h.payload);
const add = (a: unknown, b: unknown) => (a as number) + (b as number);

/** A snapshot in the format before the container, from a container's metadata and image. */
function legacy(magic: number, meta: unknown, image: Uint8Array): Uint8Array {
  const m = new TextEncoder().encode(JSON.stringify(meta));
  const out = new Uint8Array(12 + m.length + image.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, magic, true);
  view.setUint32(4, 1, true);
  view.setUint32(8, m.length, true);
  out.set(m, 12);
  out.set(image, 12 + m.length);
  return out;
}

test("sandbox: bytes() is a compressed container that restores", async () => {
  const sb = await Sandbox.create({ functions: { add } });
  sb.run("globalThis.items = Array.from({ length: 500 }, (_, i) => ({ i, name: 'item ' + i })); return 1");
  const zstd = sb.bytes();
  const plain = sb.bytes({ compression: "none" });
  const h = header(zstd);
  assert.equal(h.app, APP_SANDBOX);
  assert.equal(h.compression, COMPRESSION_ZSTD);
  assert.equal(h.referenceKind, REFERENCE_BASE);
  assert.equal(header(plain).compression, COMPRESSION_NONE);
  // Most of the image is the base realm, which the reference supplies.
  assert.ok(zstd.length * 5 < plain.length, `${zstd.length} vs ${plain.length} bytes`);
  for (const bytes of [zstd, plain]) {
    const back = await Sandbox.fromBytes(bytes, { functions: { add } });
    assert.equal(back.run("return items.length + items[499].name + add(1, 2)"), "500item 4993");
    back.dispose();
  }
  // Without a codec the snapshot is written uncompressed.
  const nocodec = await Sandbox.create({ snapshotCodec: null });
  assert.equal(header(nocodec.bytes()).compression, COMPRESSION_NONE);
  // A compressed snapshot cannot be read without one.
  await assert.rejects(Sandbox.fromBytes(zstd, { functions: { add }, snapshotCodec: null }), /has no zstd/);
  nocodec.dispose();
  sb.dispose();
});

test("sandbox: another build, another reference and damage are refused", async () => {
  const sb = await Sandbox.create({ functions: { add } });
  const bytes = sb.bytes();
  sb.dispose();
  const otherBuild = bytes.slice();
  otherBuild[8] ^= 1; // the engine build hash
  await assert.rejects(
    Sandbox.fromBytes(otherBuild, { functions: { add } }),
    /snapshot from djs build [0-9a-f]{8}; this module is build [0-9a-f]{8}: an image restores only into the engine build that wrote it/,
  );
  const otherReference = bytes.slice();
  otherReference[16] ^= 1; // the reference hash
  await assert.rejects(
    Sandbox.fromBytes(otherReference, { functions: { add } }),
    /compressed against the base image [0-9a-f]{16}; this host's is [0-9a-f]{16}/,
  );
  const damaged = bytes.slice();
  damaged[damaged.length - 20] ^= 0xff; // in the compressed image (zstd's checksum)
  await assert.rejects(Sandbox.fromBytes(damaged, { functions: { add } }), /corrupt snapshot/);
  await assert.rejects(Sandbox.fromBytes(bytes.subarray(0, 100), { functions: { add } }), /truncated snapshot/);
});

test("sandbox: bytes() stores the session image, not the builtin prefix (D2066)", async () => {
  const sb = await Sandbox.create({ functions: { add } });
  const h = header(sb.bytes({ compression: "none" }));
  assert.equal(h.imageKind, IMAGE_SESSION);
  const raw = expandSession(h.payload, base());
  // A fresh sandbox: the roots cell, no changed prefix word, and the per-VM tail.
  assert.equal(new DataView(h.payload.buffer, h.payload.byteOffset).getUint32(28, true), 0);
  assert.ok(h.payload.length * 8 < raw.length, `${h.payload.length} of ${raw.length} bytes`);
  // Changing a builtin puts its words in the diff, and they come back.
  sb.run("Array.prototype.extra = 41; return 1");
  const h2 = header(sb.bytes({ compression: "none" }));
  assert.ok(new DataView(h2.payload.buffer, h2.payload.byteOffset).getUint32(28, true) > 0);
  const back = await Sandbox.fromBytes(sb.bytes(), { functions: { add } });
  assert.equal(back.run("return add([].extra, 1)"), 42);
  back.dispose();
  // A format 1 container (a raw image, before session images) still loads.
  const v1 = encodeSnapshot({ app: APP_SANDBOX, build: h2.build, meta: h2.meta, image: expandSession(h2.payload, base()), codec: null });
  new DataView(v1.buffer).setUint16(4, 1, true);
  const old = await Sandbox.fromBytes(v1, { functions: { add } });
  assert.equal(old.run("return [].extra"), 41);
  old.dispose();
  sb.dispose();
});

test("sandbox: the format before the container still loads", async () => {
  const sb = await Sandbox.create({ functions: { add } });
  sb.setGlobal("config", { n: 41 });
  const h = header(sb.bytes({ compression: "none" }));
  sb.dispose();
  const old = legacy(0x53534a44 /* "DJSS" */, h.meta, rawImage(h));
  const back = await Sandbox.fromBytes(old, { functions: { add } });
  assert.equal(back.run("return add(config.n, 1)"), 42);
  back.dispose();
});

// ---- pi sessions ---------------------------------------------------------------------

const STARTED_AT = Date.UTC(2026, 0, 15, 10, 0, 0);
const CODE = `
  const notes = Array.from({ length: 200 }, (_, i) => ({ i, text: "note " + i }));
  const first = await tools.echo({ n: notes.length });
  const second = await tools.echo({ sum: first.n + notes[199].i, at: Date.now(), r: Math.random() });
  return second;
`;

function data(): VmStartData {
  return {
    code: CODE,
    tools: [{ name: "echo", jsName: "echo", description: "" }],
    globals: [],
    memoryLimitBytes: undefined,
    // A store no other test uses, so the prelude image is this file's own.
    store: { "0117": JSON.stringify("snapshot container") },
    deterministic: { startedAt: STARTED_AT, seed: 4 },
  };
}

/**
 * Runs CODE, answering each echo at the next tick. With `at`, snapshots the session when it
 * waits on call number `at` and continues on `resume(bytes)` (which may throw).
 */
async function piRun(
  options: {
    reuseImages?: boolean;
    codec?: null;
    at?: number;
    resume?: (bytes: Uint8Array, hooks: ResumeOptions) => VmSession;
  } = {},
): Promise<{ messages: WorkerToHostMessage[]; snapshots: Uint8Array[] }> {
  const messages: WorkerToHostMessage[] = [];
  const snapshots: Uint8Array[] = [];
  let session!: VmSession;
  let seq = 0;
  let resolveEnd!: () => void;
  let rejectEnd!: (e: unknown) => void;
  const ended = new Promise<void>((resolve, reject) => ((resolveEnd = resolve), (rejectEnd = reject)));
  const hooks: ResumeOptions = {
    wasm,
    reuseImages: options.reuseImages,
    snapshotCodec: options.codec,
    shouldInterrupt: () => false,
    post: (message) => {
      messages.push(message);
      if (message.type === "done" || message.type === "crash") setTimeout(resolveEnd);
      if (message.type !== "call") return;
      setTimeout(() => {
        try {
          seq++;
          if (seq === options.at) {
            const bytes = session.snapshot();
            snapshots.push(bytes);
            session.dispose();
            session = options.resume!(bytes, hooks);
          }
          session.deliver({ type: "result", id: message.id, ok: true, payload: message.args, seq, now: STARTED_AT + 1000 * seq });
        } catch (e) {
          rejectEnd(e);
        }
      });
    },
  };
  session = await startVm({ ...hooks, prelude: PRELUDE_SOURCE, data: data() });
  await ended;
  session.dispose();
  return { messages, snapshots };
}

test("pi: snapshot() is compressed against the prelude image and resumes", async () => {
  const reference = await piRun();
  const resumed = await piRun({ at: 2, resume: (bytes, hooks) => resumeVm(bytes, hooks) });
  assert.deepEqual(resumed.messages, reference.messages);
  const h = header(resumed.snapshots[0]);
  assert.equal(h.referenceKind, REFERENCE_PI_PRELUDE);
  assert.equal(h.compression, COMPRESSION_ZSTD);
  // A waiting session is mostly its prelude image.
  assert.ok(resumed.snapshots[0].length < 64 * 1024, `${resumed.snapshots[0].length} bytes`);
});

test("pi: another process recomputes the prelude image from the session's inputs", async () => {
  const reference = await piRun();
  const module = await freshModule();
  // Without the inputs, the snapshot's reference is missing: a clear error.
  await assert.rejects(
    piRun({ at: 1, resume: (bytes, hooks) => resumeVm(bytes, { ...hooks, wasm: module }) }),
    /compressed against pi's prelude image [0-9a-f]{16}: resume it with the session's prelude and start data/,
  );
  // Inputs that make another prelude image are refused.
  await assert.rejects(
    piRun({
      at: 1,
      resume: (bytes, hooks) => resumeVm(bytes, { ...hooks, wasm: module, prelude: PRELUDE_SOURCE, data: { ...data(), store: {} } }),
    }),
    /the given prelude and start data make [0-9a-f]{16}/,
  );
  // With them, it continues exactly like the session that was never snapshotted.
  const other = await freshModule();
  const resumed = await piRun({
    at: 1,
    resume: (bytes, hooks) => resumeVm(bytes, { ...hooks, wasm: other, prelude: PRELUDE_SOURCE, data: data() }),
  });
  assert.deepEqual(resumed.messages, reference.messages);
});

test("pi: without prelude images, snapshots use the base image", async () => {
  const reference = await piRun({ reuseImages: false });
  const resumed = await piRun({ reuseImages: false, at: 2, resume: (bytes, hooks) => resumeVm(bytes, hooks) });
  assert.deepEqual(resumed.messages, reference.messages);
  assert.equal(header(resumed.snapshots[0]).referenceKind, REFERENCE_BASE);
});

test("pi: the format before the container still loads; another build is refused", async () => {
  const reference = await piRun();
  const resumed = await piRun({
    at: 2,
    codec: null,
    resume: (bytes, hooks) => {
      const h = header(bytes);
      assert.equal(h.compression, COMPRESSION_NONE);
      return resumeVm(legacy(0x49504a44 /* "DJPI" */, h.meta, rawImage(h)), hooks);
    },
  });
  assert.deepEqual(resumed.messages, reference.messages);
  await assert.rejects(
    piRun({
      at: 1,
      resume: (bytes, hooks) => {
        const other = bytes.slice();
        other[8] ^= 1;
        return resumeVm(other, hooks);
      },
    }),
    /snapshot from djs build [0-9a-f]{8}; this module is build [0-9a-f]{8}/,
  );
  await assert.rejects(
    Sandbox.fromBytes(resumed.snapshots[0], {}),
    /a pi session snapshot, not a sandbox snapshot/,
  );
});
