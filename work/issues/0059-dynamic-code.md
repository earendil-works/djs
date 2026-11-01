# 0059: Indirect eval, Function constructors, queueMicrotask

status: done
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
- [x] Indirect eval (`(0, eval)(src)`, `globalThis.eval(src)`, and plain `eval(src)`
      treated the same): compiles as a Script in the realm (D004 Script mode, with
      completion values once 0058 lands; coordinate with the lexer via the coordinator),
      SyntaxErrors as thrown SyntaxErrors, fuel and suspension work through it (the
      compile itself is bounded; record the fuel charge per source byte).
- [x] The four dynamic function constructors per spec (parameter and body parsing as
      separate goals, the `anonymous` name, the `toString` source format, new.target and
      prototype handling, async/generator kinds), with qjs messages.
- [x] A native `queueMicrotask` (dquickts has one), and the pi adapter's polyfill is
      removed.
- [x] test262: split scan-eval into indirect-eval/Function tests (move them to active) and
      direct-eval tests (still skipped: detect direct-eval reliance, for example a
      reference to a local binding inside the eval'd source, or keep the conservative
      scan and hand-maintain a list). Report the numbers.
- [x] pi's suite rerun (`node js/pi/pi-tests.ts`), with the eval/Function failures gone.

## Log
- Compiler (lexer's area, agreed with the coordinator): new compiler/dynamic.rs and
  `compile_dynamic_function`/`DynamicKind` in compiler/mod.rs. `Parser::compile` is split
  into the statement loop and `finish_unit`, so the dynamic path reuses the finishing.
  Nothing else in the parser changed (D1590).
- Leaves CompileEval/CompileFunction/GlobalObject/EnqueueMicrotask (builtins/eval.rs);
  builtins/eval.ts holds eval, CreateDynamicFunction, queueMicrotask and MicrotaskJob. The
  four constructors call CreateDynamicFunction. Fuel: 4 per code unit (D1591). The
  RealmIntrinsic names (Async)(Generator)FunctionPrototype are added.
- Outside touches: djs-bgen allows `new.target` in `@intrinsic` builtins (one condition).
  djs-difftest escapes control characters in ratchet reasons (test_string's NUL made the
  ratchet file binary). scripts/difftest-import-dquickts.py has a hand-kept DIRECT_EVAL
  list instead of the dynamic-code regex. TESTING.md was updated for all three.
- Eval completion values: as agreed, eval compiles as a Script and returns undefined until
  0058, with the TODO in `builtins/eval.rs::compile_eval`. Coordinator: lexer wires 0058's
  completion values and an eval goal (strict eval's own environment) there.
- test262: `direct-eval` replaces `scan-eval` (D1593): 525 skipped, 540 active, 461 of
  them pass. Of the 79 failures, 56 are sloppy-only (always strict), 20 need completion
  values or the eval goal (0058), and 3 are resizable typed array gaps. 50 older ratchet
  entries pass now. Active: 37,464 / 37,722 (99.3%).
- difftest: the 11 dynamic-code skips are gone. 4 direct-eval cases stay skipped; test_function
  and test_rejected pass; 5 wait for completion values (in the ratchet). New case
  builtins/dynamic-code.js.
- queueMicrotask native (D1592); the pi adapter drops its polyfill. The adapter also removes
  WeakRef/FinalizationRegistry in deterministic runs (pi's test; 0051 added them), and its
  image cache keys on the mode.
- pi's suite: 215/226 (213 before). "keeps eval and Function" still fails only on
  `eval("typeof process")`'s completion value.

## Follow-ups
- 0058/lexer: completion values and an eval goal for `compile_eval` (TODO there). That
  should fix about 20 test262 tests, 5 difftest cases and pi's eval test.
- Decision: `Function("return this")()` is `undefined` (strict). It is the classic way
  for bundled libraries (lodash, polyfills) to find the global object when `global` and
  `self` do not exist, as in djs. One option: functions from the Function constructors
  without a "use strict" body get sloppy `this` binding only (undefined/null to the global
  object, primitives boxed). That would also fix most of the 56 sloppy test262 failures in
  built-ins/Function.
- Lone surrogates in eval'd source become U+FFFD (the compiler takes UTF-8).
- 3 resizable-buffer typed array tests (newly active) fail: Array.prototype.fill on a
  resized TA, Object.freeze of a TA on a resizable buffer, subarray.
- ARCH.md still says "indirect eval and Function may come later". Coordinator: it needs an
  edit for D014.

## Result
- Indirect eval (`eval(x)` is indirect, D014), `Function`, `GeneratorFunction`,
  `AsyncFunction` and `AsyncGeneratorFunction` per CreateDynamicFunction, with
  injection-proof separate-goal checks (D1590). Compiling is a leaf; running is an ordinary
  call (D1591).
- Native `queueMicrotask` (D1592).
- test262: `direct-eval` group (D1593). 540 tests are active (461 pass), and active is
  37,464 / 37,722.
- pi's suite: 215/226.
- Known gaps: eval returns undefined and its declarations are global until 0058;
  dynamic functions are strict.
