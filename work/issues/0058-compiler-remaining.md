# 0058: Compiler: `using`, decorators, `accessor`, Script completion values, `import()`

status: open
agent: -
branch: issue/0058-compiler-remaining
depends-on: 0048, 0051 (disposables runtime, SYM_DISPOSE/SYM_ASYNC_DISPOSE)
touches: crates/djs-core/src/compiler/, crates/djs-core/src/bytecode/ (additive), crates/djs-core/src/interp/ (small additive ops, logged), builtins/ (decorator helpers), tests/test262.toml (promotion), tests/corpus/ (prelude/engine workarounds removal), ratchets

## Goal
The remaining language gaps after classes and TypeScript:

## Acceptance criteria
- [ ] `using` and `await using` declarations (block, for-of heads, for heads, switch cases;
      the disposal order, SuppressedError chaining, and await semantics per spec), on the
      0051 runtime (DisposableStack machinery, SYM_DISPOSE/SYM_ASYNC_DISPOSE). Promote the
      `using` test262 paths that stay in later by path (heap's 0051 note) to active.
- [ ] Script completion values (the 0014 follow-up): Script mode returns the completion
      value of the last statement per spec, so `$262.evalScript` and the difftest engine's
      temporary-global workaround (D1440) can be removed. Remove the workaround in
      djs-difftest and djs-test262 once it works.
- [ ] `import()` in Scripts/function bodies: compiles (with import attributes syntax) and
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

## Follow-ups

## Result
