# 0041: Async functions, await, generators, yield* (compiler side)

status: open
agent: -
branch: issue/0041-async-generators-compiler
depends-on: 0037
touches: crates/djs-core/src/compiler/, crates/djs-core/src/bytecode/ (additive), docs/compiler.md, ratchets

## Goal
Codemode scripts are async function bodies (D008), and `await` is everywhere in them.
heap's 0037 part 3 built the runtime: the ops `generator_start`, `yield`, `async_start`,
`await`, `async_return`, `async_reject` (D1375), `%GeneratorPrototype%`, and the exact
contract in docs/compiler.md 7.7. This issue is the compiler side.

## Acceptance criteria
- [ ] `async function`, async arrows, async methods (object literals now; class methods
      come with 0040), `await` expressions (with the precedence and the early errors:
      `await` as an identifier in async bodies, in parameters, …), and the codemode
      async-body mode compiled as an async function (top-level `await` in the body).
- [ ] Generators: `function*`, generator methods, `yield`, `yield*` (delegation with
      next/throw/return forwarding and iterator closing), generators inside for-of, the
      destructuring follow-up from 0036 (a pattern's iterator is closed when a `yield`
      in a default ends in a return).
- [ ] Async generators and `for await`: the runtime has no async generator request
      queue yet (0037 follow-up). Check with the coordinator before doing the runtime
      part. The compiler side may land with them as "not supported yet".
- [ ] difftest: the codemode async-body cases (await over `tools.*`), the shared prelude
      cases that only needed `await`. e2e tests checked against qjs (job ordering), a
      disassembly snapshot, the test262 compile sweep before/after, and the ratchets.

## Notes
- 0040 (classes) is paused for this. Its branch stays; resume it afterwards. Keep the
  async/generator function kinds ready for class methods.

## Log

## Follow-ups

## Result
