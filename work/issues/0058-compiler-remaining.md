# 0058: Compiler: `using`, decorators, `accessor`, Script completion values, `import()`

status: in-progress
agent: lexer
branch: issue/0058-compiler-remaining
depends-on: 0048, 0051 (disposables runtime, SYM_DISPOSE/SYM_ASYNC_DISPOSE)
touches: crates/djs-core/src/compiler/, crates/djs-core/src/bytecode/ (additive), crates/djs-core/src/interp/ (small additive ops, logged), builtins/ (decorator helpers), tests/test262.toml (promotion), tests/corpus/ (prelude/engine workarounds removal), ratchets

## Goal
The remaining language gaps after classes and TypeScript:

## Acceptance criteria
- [x] `using` and `await using` declarations (block, for-of heads, for heads, switch cases;
      the disposal order, SuppressedError chaining, and await semantics per spec), on the
      0051 runtime (DisposableStack machinery, SYM_DISPOSE/SYM_ASYNC_DISPOSE). Promote the
      `using` test262 paths that stay in later by path (heap's 0051 note) to active.
- [x] Script completion values (the 0014 follow-up): Script mode returns the completion
      value of the last statement per spec, so `$262.evalScript` and the difftest engine's
      temporary-global workaround (D1440) can be removed. Remove the workaround in
      djs-difftest and djs-test262 once it works.
- [x] `import()` in Scripts/function bodies: compiles (with import attributes syntax) and
      rejects at runtime with qjs's error when no module loader exists (pi expects a
      runtime rejection, 0049), instead of a compile-time SyntaxError.
- [ ] `accessor` class fields (auto-accessors) and decorators: TC39 decorators (the
      2023-11 spec that test262 tests, the `decorators` feature) and TypeScript legacy
      `experimentalDecorators` behind a flag (dquickts' `--experimental-decorators`,
      test_decorators_legacy.ts). Decide scope with the coordinator after the first two
      items; decorators may become their own issue.
- [ ] strip-types: the files ts-blank-space accepts that djs rejected because of
      `using`/`accessor`/decorators now match too; report the new rate.
- [ ] e2e checked against qjs, difftest cases, the compile sweep before/after, ratchets.

## Log

- `using`/`await using` (D1580, compiler/using.rs, builtins/disposable.ts `Using*`): a
  statement list opens a try/finally region at its first declaration with a resource
  Record on the operand stack; disposal is a `gosub` subroutine (sync: one intrinsic;
  async: a loop with a real `await` per resource, ticks match qjs). Blocks, function and
  static-block bodies, catch blocks, codemode bodies, `for (using x = ...;;)` (around the
  loop) and `for ([await] using x of ...)` (per iteration). Switch clauses and Script /
  eval top level reject it. qjs's messages. Promoted language/statements/using,
  await-using and the for-of/for-await-of using heads (161 tests, all pass).
- Completion values (D1581): Script (not the privileged prelude) and eval code return
  their completion value (hidden local, qjs's eval_ret but spec-like for do-while).
  `Goal::Eval` (`CompileOptions::eval`): strict eval code, declarations local to the
  call; builtins/eval.rs (0059) compiles indirect eval with it. djs-difftest's
  `__difftestSetup`/`__difftestCase` globals are gone; djs-test262 had no workaround
  (`$262.evalScript` already returned the call's result). test262: +20 (indirect eval
  completion/this/lexical env), -8 indirect-eval tests that need sloppy eval code
  (vars leaking to the global object; djs is always strict). difftest: 4 more cases pass
  (test_regexp, test_string, test_parse_arrow_function, test_enums used eval values).
  vm/tests.rs' OOM test now accepts a completion value as the result (outside touches,
  one assertion).
- `import()` (D1582, builtins/import.ts): compiles with the attributes argument, rejects
  in a job with qjs's no-loader ReferenceError; `import.meta` is a SyntaxError.
- strip-types: 4085/4802 match (was 4053), 0 differ, 717 djs syntax errors (134 `@`
  decorators; the remaining `using` files are the conformance tests' deliberate errors).

## Follow-ups

- test262 `dynamic-import`: 354/682 would pass now; the syntax and rejection tests could
  be promoted by path if wanted (coordinator).
- `accessor` and decorators: scope to be decided (asked the coordinator).

## Result
