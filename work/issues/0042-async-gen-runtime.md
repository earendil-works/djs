# 0042: Async generator runtime, function-kind intrinsics, rejections in the host

status: open
agent: -
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

## Follow-ups

## Result
