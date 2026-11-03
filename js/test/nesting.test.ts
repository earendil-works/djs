// The parser's nesting limit fits djs-wasm's shadow stack (D1941, docs/compiler.md 2.4):
// for every recursive construct, the deepest nesting that compiles runs in wasm, and one
// level more is a SyntaxError ("stack overflow"), never a trap. The stack is 128 KiB, the
// deepest accepted nesting needs at most ~115 KiB (D2540), so a construct whose stack per
// level grows a lot shows up here.

import { test } from "node:test";
import assert from "node:assert/strict";
import { Sandbox, SandboxError, loadModule } from "../src/sandbox.ts";

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
