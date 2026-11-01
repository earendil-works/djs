# 0059: Indirect eval, Function constructors, queueMicrotask

status: in-progress
agent: infra
branch: issue/0059-dynamic-code
depends-on: 0054
touches: builtins/ (function.ts, global), crates/djs-core/src/builtins/, crates/djs-core/src/vm/ + interp/ (a leaf that compiles source at runtime: additive), crates/djs-core/src/compiler/ (CreateDynamicFunction entry points only: parse parameters + body as a function, minimal), tests/test262.toml (re-split scan-eval), ratchets, js/pi/ (drop the queueMicrotask polyfill)

## Goal
ARCH: "There is no direct `eval`; indirect eval and `Function` may come later." pi's suite
expects them (0049/0054), the compiler is part of the VM anyway, and test262's
scan-eval group (1,065 tests) is mostly indirect eval and the Function constructors.
Decision (coordinator): implement **indirect** eval (global scope, strict as always) and
`Function`/`AsyncFunction`/`GeneratorFunction`/`AsyncGeneratorFunction` per
CreateDynamicFunction. **Direct** eval (access to the caller's local scope) stays a
non-goal: a call `eval(x)` is treated as indirect eval, and tests that rely on direct
eval semantics stay skipped.

## Acceptance criteria
- [ ] Indirect eval (`(0, eval)(src)`, `globalThis.eval(src)`, and plain `eval(src)`
      treated the same): compiles as a Script in the realm (D004 Script mode, with
      completion values once 0058 lands; coordinate with the lexer via the coordinator),
      SyntaxErrors as thrown SyntaxErrors, fuel and suspension work through it (the
      compile itself is bounded; record the fuel charge per source byte).
- [ ] The four dynamic function constructors per spec (parameter and body parsing as
      separate goals, the `anonymous` name, the `toString` source format, new.target and
      prototype handling, async/generator kinds), with qjs messages.
- [ ] A native `queueMicrotask` (dquickts has one), and the pi adapter's polyfill is
      removed.
- [ ] test262: split scan-eval into indirect-eval/Function tests (move them to active) and
      direct-eval tests (still skipped: detect direct-eval reliance, for example a
      reference to a local binding inside the eval'd source, or keep the conservative
      scan and hand-maintain a list). Report the numbers.
- [ ] pi's suite rerun (`node js/pi/pi-tests.ts`), with the eval/Function failures gone.

## Log

## Follow-ups

## Result
