# 0042: Async generator runtime, function-kind intrinsics, rejections in the host

status: in-progress
agent: heap
branch: issue/0042-async-gen-runtime
depends-on: 0037
touches: crates/djs-core/src/interp/, crates/djs-core/src/vm/, builtins/ (generator.ts, asyncgen.ts, async-iteration helpers), crates/djs-wasm/, js/src/sandbox.ts, docs/compiler.md 7.7 (contract additions), docs/host.md

## Goal
The 0037 follow-ups that the compiler side (0041, lexer, in parallel) needs next, plus
the host-side gap for unhandled rejections.

## Acceptance criteria
- [ ] Async generators: the per-generator request queue (AsyncGeneratorEnqueue /
      Resume / Drain per spec), `%AsyncGeneratorPrototype%` (next/return/throw),
      `%AsyncIteratorPrototype%`, `%AsyncFromSyncIteratorPrototype%` (for `for await`
      over sync iterables), and the ops/contract the compiler needs (extend
      docs/compiler.md 7.7). Tests with hand-assembled bytecode, as in 0037.
- [ ] `%GeneratorFunction%`, `%AsyncFunction%`, `%AsyncGeneratorFunction%` and their
      prototypes (the constructors themselves throw like `Function`: no eval), wired
      so the closures the compiler creates for each kind get the right [[Prototype]]
      and `prototype` objects. Coordinate the function-kind flags with the lexer via the
      coordinator.
- [ ] Unhandled rejections exported from djs-wasm and reported by the Sandbox (an
      option: throw, warn, or collect).
- [ ] Ratchets updated; test262 numbers for built-ins/AsyncGenerator*, GeneratorPrototype
      and AsyncFromSyncIteratorPrototype (likely blocked on 0041 until it lands).

## Log
- Flags agreed with the lexer (via coordinator): existing `func_flags` bits, any kind.
- Part 1 (D1420-D1422): function-kind prototypes and constructors; async generators
  (no new ops; state 4 for await, request queue, builtins/asyncgen.ts);
  %AsyncIteratorPrototype%, %AsyncFromSyncIteratorPrototype%, %GetAsyncIterator,
  %AsyncIteratorReturnMethod, %AsyncIteratorCloseThrow; `yield_star` (0041's request).
  docs/compiler.md 7.7 extended. Tests on hand-assembled bytecode (vm/tests.rs): async
  generator request queue (order checked against node: qjs never settles a next() queued
  behind return() on an unstarted async generator), yield_star, for await over sync,
  async-generator and custom async iterables incl. break-close, function kinds.
  test262: no change yet (the tests need 0041's syntax).

## Follow-ups

## Result
