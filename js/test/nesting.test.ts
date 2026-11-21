// The parser's nesting limit fits djs-wasm's shadow stack (D1941, docs/compiler.md 2.4):
// for every recursive construct, the deepest nesting that compiles runs in wasm, and one
// level more is a SyntaxError ("stack overflow"), never a trap. The stack is 128 KiB, the
// deepest accepted nesting needs at most ~115 KiB (D2540), so a construct whose stack per
// level grows a lot shows up here.

import { test } from "node:test";
import assert from "node:assert/strict";
import { Sandbox, SandboxError, loadModule } from "../src/sandbox.ts";
import { stackMoves } from "../src/trap.ts";

const nested: Record<string, (n: number) => string> = {
  parens: (n) => "(".repeat(n) + "1" + ")".repeat(n),
  arrays: (n) => "[".repeat(n) + "]".repeat(n),
  objects: (n) => "{a:".repeat(n) + "1" + "}".repeat(n),
  arrows: (n) => "() => ".repeat(n) + "1",
  ternary: (n) => "1 ? ".repeat(n) + "1" + " : 0".repeat(n),
  calls: (n) => "f(".repeat(n) + "1" + ")".repeat(n),
  funcs: (n) => "function g(){ return ".repeat(n) + "1" + "}".repeat(n),
  blocks: (n) => "(() => {" + "{".repeat(n) + "}".repeat(n) + "})",
  ifs: (n) => "(() => {" + "if (1) ".repeat(n) + "; })",
  fors: (n) => "(() => {" + "for (;0;) ".repeat(n) + "; })",
  whiles: (n) => "(() => {" + "while (0) ".repeat(n) + "; })",
  trys: (n) => "(() => {" + "try {".repeat(n) + "}finally{}".repeat(n) + "})",
  switches: (n) => "(() => {" + "switch (1) { case 1: ".repeat(n) + "}".repeat(n) + "})",
  template: (n) => "`${".repeat(n) + "1" + "}`".repeat(n),
  classes: (n) => "class C0 { m() { ".repeat(n) + "}}".repeat(n),
  methods: (n) => "({ m() { return ".repeat(n) + "1" + "}})".repeat(n),
  patterns: (n) => "(() => { let " + "[".repeat(n) + "a" + "]".repeat(n) + " = []; })",
  objPatterns: (n) => "(() => { let " + "{a:".repeat(n) + "a" + "}".repeat(n) + " = {}; })",
  regexpGroups: (n) => "/" + "(".repeat(n) + "a" + ")".repeat(n) + "/",
};

test("the deepest accepted nesting runs in wasm", async () => {
  const module = await loadModule();
  // Compiles (and runs) one source in a fresh sandbox: true, false for the depth limit's
  // SyntaxError; a trap (stack overflow in wasm) fails the test.
  const compiles = (src: string): boolean => {
    const sb = new Sandbox({ module });
    try {
      sb.run(`const f = (x) => x; const v = ${src};\nreturn 1;`);
      return true;
    } catch (e) {
      assert.ok(e instanceof SandboxError, String(e));
      assert.notEqual(e.kind, "trap", `trap: ${e.message}`);
      assert.equal(e.kind, "syntax", e.message);
      assert.match(e.message, /stack overflow/);
      return false;
    } finally {
      sb.dispose();
    }
  };
  for (const [name, mk] of Object.entries(nested)) {
    let lo = 1;
    let hi = 2;
    while (compiles(mk(hi))) {
      lo = hi;
      hi *= 2;
      assert.ok(hi <= 1 << 13, `${name}: no depth limit`);
    }
    while (hi - lo > 1) {
      const m = (lo + hi) >> 1;
      if (compiles(mk(m))) lo = m;
      else hi = m;
    }
    assert.ok(lo >= 20, `${name}: only ${lo} levels`);
  }
});

// The elastic stack (D3280): a compile that nests deeper than the module's small stack
// allows stops before any effect and runs again on a larger one. That must leave the same
// heap and spend the same fuel as a compile that never stopped: the same module with the
// elastic stack off compiles at once on its own stack (only where that stack is large: the
// debug module's).
test("a compile on the larger stack leaves the same heap and fuel", async () => {
  const module = await loadModule();
  const deep = (n: number) => "(".repeat(n) + "1" + ")".repeat(n);
  const groups = (n: number) => "(".repeat(n) + "a" + ")".repeat(n);
  const programs = [
    // The script itself, a regexp literal in it, then run-time compiles.
    `return ${deep(150)};`,
    `return /${groups(100)}/.exec("a").length;`,
    `return eval(${JSON.stringify(deep(150))}) + eval("1 + 1");`,
    `return new Function("x", ${JSON.stringify(`return x + ${deep(150)};`)})(1);`,
    `return new RegExp(${JSON.stringify(groups(100))}).exec("a").length;`,
    // A function compiled at its first call (where functions compile lazily).
    `const f = () => { return ${deep(150)}; }; return f() + f();`,
    // Deep, then too deep: the same SyntaxError as without the small stack.
    `try { eval(${JSON.stringify(deep(400))}); } catch (e) { return String(e); }`,
    `try { new RegExp(${JSON.stringify(groups(300))}); } catch (e) { return String(e); }`,
  ];
  type Inner = { x: Record<string, any>; vm: number };
  const run = (src: string, elastic: boolean) => {
    const sb = new Sandbox({ module, fuelSlice: 1e9, deterministic: { startedAt: 0, seed: 1 } });
    const { x, vm } = sb as unknown as Inner;
    if (!elastic) x.djs_elastic_stack(0);
    const moves = stackMoves;
    const value = sb.run(src);
    const arena = new Uint8Array(x.memory.buffer, x.vm_arena_ptr(vm), x.vm_arena_len(vm)).slice();
    const fuel = x.vm_fuel_used(vm);
    sb.dispose();
    return { value, arena, fuel, moved: stackMoves > moves };
  };
  const probe = new Sandbox({ module });
  const bigStack = ((probe as unknown as Inner).x.__stack_pointer.value as number) >= 128 * 1024;
  probe.dispose();
  for (const src of programs) {
    const a = run(src, true);
    assert.ok(a.moved, `did not move the stack: ${src.slice(0, 40)}`);
    if (!bigStack) continue;
    const b = run(src, false);
    assert.ok(!b.moved);
    assert.deepEqual(a.value, b.value, src.slice(0, 40));
    assert.equal(a.fuel, b.fuel, `fuel: ${src.slice(0, 40)}`);
    assert.ok(Buffer.compare(a.arena, b.arena) === 0, `heap: ${src.slice(0, 40)}`);
  }
});
