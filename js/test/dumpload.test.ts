// Dumps (0167, docs/functions-as-code.md): `sandbox.dump()` writes the VM's bindings as
// JavaScript, `load()` runs it in another sandbox; host functions are re-bound by name.

import { test } from "node:test";
import assert from "node:assert/strict";
import { loadModule, Sandbox, SandboxError } from "../src/sandbox.ts";

test("a dump recreates closures, data and buffers in another sandbox", async () => {
  const module = await loadModule();
  const functions = { fetchName: (id: unknown) => `user${String(id)}` };
  const a = new Sandbox({ module, functions });
  a.run(`
    globalThis.lib = (() => {
      let calls = 0;
      const cache = new Map();
      return {
        name(id) { calls++; if (!cache.has(id)) cache.set(id, fetchName(id)); return cache.get(id); },
        stats: () => ({ calls, cached: cache.size }),
      };
    })();
    globalThis.blob = new Uint8Array([1, 2, 3]);
    lib.name(1); lib.name(1);
  `);
  const text = a.dump();
  assert.match(text, /^\/\/ djs-dump 1/);
  assert.doesNotMatch(text, /user1"\)|fetchName = /);
  const b = new Sandbox({ module, functions });
  b.load(text);
  const check = "return JSON.stringify([lib.name(2), lib.stats(), [...blob]]);";
  assert.equal(b.run(check), a.run(check));
  assert.equal(b.run("return typeof $dump"), "undefined");
});

test("live values are refused with their path", async () => {
  const s = new Sandbox({ module: await loadModule() });
  s.run("globalThis.p = new Promise(() => {});");
  assert.throws(() => s.dump(), (e: SandboxError) => e.kind === "misuse" && /p: a pending promise/.test(e.message));
  assert.throws(() => s.load("x = 1"), /not a djs dump/);
});

test("a dump that ends with a cycle patch loads", async () => {
  const module = await loadModule();
  const a = new Sandbox({ module });
  a.run("globalThis.data = { name: 'x' }; data.self = data;");
  const text = a.dump();
  assert.match(text, /data\.self = data;\n$/);
  const b = new Sandbox({ module });
  b.load(text);
  assert.equal(b.run("return data.self === data && data.name"), "x");
});
