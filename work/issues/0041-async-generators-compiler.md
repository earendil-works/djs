# 0041: Async functions, await, generators, yield* (compiler side)

status: done
agent: lexer
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

- Function kinds: the existing `func_flags` kinds plus the ASYNC/GENERATOR bits (sent to
  the coordinator for heap's D1420).
- Async functions, arrows, object-literal methods (`async m`, `*m`, `async *m`), `await`
  with qjs's early errors, the codemode body as an async function (D1410). The test
  harnesses (compiler tests, vm tests, tests/e2e.rs, djs-wasm tests, djs-test262 fixture)
  now read the body's promise.
- Generators: `yield`, `yield*` (with heap's `yield_star`, D1422), return/throw
  resumption. A return from a resumed yield goes through a one-instruction throw region
  at statement depth (D1411); the same chain closes open array-pattern and for-of-head
  iterators (the 0036 follow-up: the dstr `*-rtrn-close*` tests pass).
- Async generators and `for await` on heap's 0042 contract (docs/compiler.md 7.7).
- difftest: `tools.echo`/`tools.fail` are async host functions now (as pi's tools), so
  `codemode/parallel.js` and `tool-failure.js` pass; new case `codemode/async-helpers.js`
  (plain, `--suspend-every 97` and `--gc-stress`).
- Outside the touches, small: tests/e2e.rs (rejected body promise = `Uncaught`),
  djs-wasm tests, djs-difftest (async tools, glue test), the djs-test262 fixture (a
  different unsupported construct), and in interp/run.rs the debug-only stack-effect
  check skips TERM ops and the suspending ops: under GC stress a tail call's new frame template can
  land at the old template's offset, which made the check fire falsely (`async_reject`
  in `tests/data/e2e/async_await.js` with `--gc-stress`).
- Compile sweep (test262, all files): before 30,606 ok / 19,361 not supported yet /
  2,045 negative / 594 other; after 34,745 / 14,281 / 2,682 / 706 (the new "other" are
  constructs behind async functions: `import`, `using`).
- test262 active: 18,833 pass / 5,287 fail after rebasing on main (ratchet 5,638 → 5,289
  lines). Later group: 4,051 / 13,449 pass (generators, async-iteration are there).

## Follow-ups

- Move the test262 features `generators` and `async-iteration` (and the
  GeneratorPrototype/GeneratorFunction/AsyncGenerator* built-in dirs) from `later` to
  active: their language tests now mostly pass (coordinator decision).
- `arguments` objects have no `Symbol.iterator`: `[...arguments]` and `yield* arguments`
  throw "value is not iterable" (12 test262 async-generator yield-star tests use
  `[...arguments]`). Runtime/builtins.
- Class async/generator methods and `await`/`yield` in class bodies (static blocks, field
  initializers): 0040 (FuncState flavors are ready: `start_flavor`).
- qjs closes a `for await` iterator after rethrowing a body error (its order differs from
  the spec and from djs, which awaits `return()` first); no oracle case covers it.
- Small message differences from qjs: `yield\n* 2`, `for (yield in x)`, `({yield})`,
  `catch (await)`.
- `scope-paramsbody-var-open` (separate var environment for parameter expressions) fails
  for every function kind, not just generators.

## Result

Landed: async functions/arrows/methods and `await`; the codemode body is an async
function (its result is a promise, D1410); generators with `yield`, `yield*` and
return/throw resumption (D1411, D1412); async generators and `for await`; early errors as
qjs. Code: compiler/function.rs (`start_flavor`, async region, return helpers),
compiler/generator.rs (yield, yield*, the return path), for_in_of.rs (`for await`),
docs/compiler.md 7.8. Tests: e2e cases `async_await.js`, `generators.js`,
`async_generators.js`; compiler tests (errors, `end_to_end_async_generators`); snapshot
`async_generators.js`; difftest `codemode/async-helpers.js`. Ratchets updated.

Known gaps: see Follow-ups (arguments iterability, class forms in 0040).
