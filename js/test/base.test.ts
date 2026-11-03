// The base snapshot inside wasm (D1321): it is made by a host build of djs-core at build
// time, so check that it equals the realm a wasm build compiles itself (no pointer-width
// or platform dependence), and that VMs from it are independent.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defaultWasmPath } from "../src/sandbox.ts";
import { provideCodedBase } from "../src/snapshot.ts";

const path = defaultWasmPath();
const module = new WebAssembly.Module(readFileSync(typeof path === "string" ? path : fileURLToPath(path)));

test("the base equals a realm compiled in wasm", (t) => {
  const x = new WebAssembly.Instance(module, {}).exports as any;
  provideCodedBase(module, x);
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
  // Both collected: a canonical image schedules its first collection as full (D2045).
  assert.equal(x.vm_collect(base), 0);
  const a = arena(base);
  const b = arena(compiled);
  assert.equal(a.length, b.length);
  assert.ok(Buffer.from(a).equals(Buffer.from(b)), "the base differs from the realm compiled in wasm");
  x.vm_free(base);
  x.vm_free(compiled);
});

test("the coded base: decoded by the first instance, injected into later ones (D2252)", (t) => {
  const first = new WebAssembly.Instance(module, {}).exports as any;
  if (first.djs_base_inject_ptr === undefined) {
    t.skip("djs-wasm without the coded base");
    return;
  }
  // The coded base is a custom section, outside the instance's memory (D2541): without it
  // handed over, there is no base and no VM.
  assert.equal(first.djs_base_ptr(), 0);
  assert.equal(first.vm_new(0), 0);
  assert.equal(first.djs_base_coded_ptr(5), 0, "not the section's length");
  assert.ok(provideCodedBase(module, first));
  assert.ok(!provideCodedBase(module, first), "it has the coded base");
  const read = (x: any) => {
    const at = x.djs_base_ptr(); // may decode, growing the memory
    return new Uint8Array(x.memory.buffer, at, x.djs_base_len()).slice();
  };
  const arena = (x: any, vm: number) => new Uint8Array(x.memory.buffer, x.vm_arena_ptr(vm), x.vm_arena_len(vm)).slice();
  const base = read(first);
  const baseAt = first.djs_base_ptr();
  const vm1 = first.vm_new(0);
  // The decoded bytes became the VM's arena, grown in place: no second copy (D2060).
  assert.equal(first.vm_arena_ptr(vm1), baseAt, "the decoded base is the arena");
  // An instance given the bytes takes them and creates the same VM.
  const second = new WebAssembly.Instance(module, {}).exports as any;
  const at = second.djs_base_inject_ptr(base.length);
  assert.notEqual(at, 0);
  new Uint8Array(second.memory.buffer, at, base.length).set(base);
  assert.equal(second.djs_base_inject(), 1);
  assert.equal(second.djs_base_ptr(), at, "the injected copy is the base");
  assert.equal(second.djs_base_inject_ptr(base.length), 0, "a base is set: no more room");
  const vm2 = second.vm_new(0);
  assert.equal(second.vm_arena_ptr(vm2), at, "the injected base is the arena");
  assert.ok(Buffer.from(arena(first, vm1)).equals(Buffer.from(arena(second, vm2))));
  // The base was given up: reading it again needs the coded base, decodes a fresh copy,
  // and a second VM in the same instance starts from the same bytes.
  assert.equal(second.djs_base_ptr(), 0);
  assert.ok(provideCodedBase(module, second));
  assert.ok(Buffer.from(read(second)).equals(Buffer.from(base)));
  const vm2b = second.vm_new(0);
  assert.ok(Buffer.from(arena(second, vm2b)).equals(Buffer.from(arena(second, vm2))));
  // A damaged copy (or the wrong length) is refused, and the instance decodes its own.
  const third = new WebAssembly.Instance(module, {}).exports as any;
  assert.equal(third.djs_base_inject_ptr(base.length + 8), 0);
  const at3 = third.djs_base_inject_ptr(base.length);
  const bad = base.slice();
  bad[bad.length >> 1] ^= 4;
  new Uint8Array(third.memory.buffer, at3, bad.length).set(bad);
  assert.equal(third.djs_base_inject(), 0);
  provideCodedBase(module, third);
  assert.ok(Buffer.from(read(third)).equals(Buffer.from(base)));
  const vm3 = third.vm_new(0);
  assert.ok(Buffer.from(arena(first, vm1)).equals(Buffer.from(arena(third, vm3))));
});
