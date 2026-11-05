// trapText (D2543): a wasm trap's panic message, a stack overflow, or the runtime's text.

import { test } from "node:test";
import assert from "node:assert/strict";
import { trapText } from "../src/trap.ts";

function exports(panic: string, sp: number | undefined) {
  const memory = new WebAssembly.Memory({ initial: 2 });
  const bytes = new TextEncoder().encode(panic);
  new Uint8Array(memory.buffer).set(bytes, 100);
  return {
    memory,
    djs_panic_ptr: () => 100,
    djs_panic_len: () => bytes.length,
    __stack_pointer:
      sp === undefined ? undefined : new WebAssembly.Global({ value: "i32", mutable: true }, sp | 0),
  };
}

test("a trap names a stack overflow, a panic, an engine panic, or the runtime's message", () => {
  const oob = new WebAssembly.RuntimeError("memory access out of bounds");
  // The stack pointer wrapped below 0: an overflow of the shadow stack.
  assert.match(trapText(exports("", 0xffffffc0), oob), /^stack overflow: .*memory access out of bounds/);
  // The panic message is not read then (the call would trap again).
  assert.match(trapText(exports("djs-wasm panicked", 0xffffffc0), oob), /^stack overflow/);
  assert.equal(trapText(exports("djs-wasm panicked", 60_000), oob), "djs-wasm panicked");
  // A stack pointer inside memory: some other fault.
  assert.equal(trapText(exports("", 60_000), oob), "memory access out of bounds");
  // A panic of the shipped module: an `unreachable` trap without a message.
  const unreachable = new WebAssembly.RuntimeError("unreachable");
  assert.match(trapText(exports("", 60_000), unreachable), /^engine panic .*: unreachable$/);
  assert.equal(trapText(exports("djs-wasm panicked", 60_000), unreachable), "djs-wasm panicked");
  // A module without the export.
  assert.equal(trapText(exports("", undefined), oob), "memory access out of bounds");
});
