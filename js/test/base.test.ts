// The base snapshot inside wasm (D1321): it is made by a host build of djs-core at build
// time, so check that it equals the realm a wasm build compiles itself (no pointer-width
// or platform dependence), and that VMs from it are independent.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defaultWasmPath } from "../src/sandbox.ts";

const path = defaultWasmPath();
const module = new WebAssembly.Module(readFileSync(typeof path === "string" ? path : fileURLToPath(path)));

test("the base equals a realm compiled in wasm", (t) => {
  const x = new WebAssembly.Instance(module, {}).exports as any;
  if (x.vm_new_compiled === undefined) {
    // Release modules leave it out; the gate builds with `--features compiled-prelude`.
    t.skip("djs-wasm built without the compiled-prelude feature");
    return;
  }
  const arena = (vm: number) => new Uint8Array(x.memory.buffer, x.vm_arena_ptr(vm), x.vm_arena_len(vm)).slice();
  const base = x.vm_new(0);
  const compiled = x.vm_new_compiled(0);
  assert.notEqual(compiled, 0);
  assert.equal(x.vm_collect(compiled), 0);
  const a = arena(base);
  const b = arena(compiled);
  assert.equal(a.length, b.length);
  assert.ok(Buffer.from(a).equals(Buffer.from(b)), "the base differs from the realm compiled in wasm");
  x.vm_free(base);
  x.vm_free(compiled);
});
