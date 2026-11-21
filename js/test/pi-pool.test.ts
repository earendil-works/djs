// pi's adapter reuses disposed instances (D3023): a pooled instance holds nothing of the
// session before, also where a restored prelude image goes without being zeroed first
// (D3452).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defaultWasmPath } from "../src/sandbox.ts";
import { startVm, type VmSession } from "../pi/vm.ts";
import { PRELUDE_SOURCE } from "../../tests/pi/prelude-source.ts";

const path = defaultWasmPath();
const bytes = readFileSync(typeof path === "string" ? path : fileURLToPath(path));

/** Runs `code` to its end and disposes the session. */
function run(wasm: WebAssembly.Module, code: string): Promise<void> {
  return new Promise((resolve, reject) => {
    let session: VmSession | undefined;
    let ended = false;
    startVm({
      wasm,
      prelude: PRELUDE_SOURCE,
      data: { code, tools: [], globals: [], memoryLimitBytes: undefined, store: {} },
      post: (m) => {
        if (m.type !== "done" && m.type !== "crash") return;
        ended = true;
        queueMicrotask(() => {
          session?.dispose();
          if (m.type === "crash" || !m.ok) reject(new Error(JSON.stringify(m)));
          else resolve();
        });
      },
      shouldInterrupt: () => false,
    }).then((s) => {
      session = s;
      if (ended) s.dispose();
    }, reject);
  });
}

test("a pooled instance keeps nothing of the session before (D3023, D3452)", async () => {
  // A module of its own: its own pool and image cache.
  const wasm = new WebAssembly.Module(bytes);
  const made: WebAssembly.Instance[] = [];
  const Instance = WebAssembly.Instance;
  WebAssembly.Instance = function (m: WebAssembly.Module, i?: WebAssembly.Imports) {
    const x = new Instance(m, i);
    made.push(x);
    return x;
  } as unknown as typeof WebAssembly.Instance;
  const marker = "pool-hygiene-marker";
  const find = () => {
    const text = new TextEncoder().encode(marker);
    for (const x of made) {
      const m = new Uint8Array((x.exports.memory as WebAssembly.Memory).buffer);
      for (let i = m.indexOf(text[0]); i !== -1; i = m.indexOf(text[0], i + 1)) {
        if (text.every((b, k) => m[i + k] === b)) return true;
      }
    }
    return false;
  };
  try {
    await run(wasm, "return 1"); // the prelude image
    // Restored sessions that leave the marker all over their heap; the later ones restore
    // where the earlier ones staged the image, without zeroing it first.
    for (let i = 0; i < 3; i++) {
      await run(wasm, `const a = []; for (let i = 0; i < 2000; i++) a.push(["${marker}", i].join(":")); return a.length;`);
      assert.ok(find(), "the session's strings are in its memory");
      await run(wasm, "return 1");
      assert.ok(!find(), "a restored session's instance holds nothing of the one before");
    }
    assert.equal(made.length, 1, "every session ran in the pooled instance");
  } finally {
    WebAssembly.Instance = Instance;
  }
});
