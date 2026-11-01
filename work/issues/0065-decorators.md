# 0065: `accessor` auto-accessors and decorators (TC39, then TS legacy)

status: in-progress
agent: lexer
branch: issue/0065-decorators
depends-on: 0058
touches: crates/djs-core/src/compiler/, crates/djs-core/src/bytecode/ (additive), crates/djs-core/src/interp/ (small additive ops, logged), builtins/ (decorator helpers), crates/djs-cli + djs-wasm + js/src/sandbox.ts (a legacy-decorators compile flag, part 2), tests/test262.toml (promote `decorators`), ratchets

## Goal
The last class features. Part 1: TC39 decorators (the 2023-11 spec at the test262 commit:
the `decorators` feature) and `accessor` auto-accessors, which is the decorator
application order, the context objects (`kind`, `name`, `access`, `static`, `private`,
`metadata`, `addInitializer`), `Symbol.metadata`, and class/method/getter/setter/field/
accessor decorators. Part 2 (optional, after part 1): TypeScript legacy
`experimentalDecorators` (+ `emitDecoratorMetadata`?) behind a compile flag, matching
dquickts' `--experimental-decorators` and tests/test_decorators_legacy.ts.

## Acceptance criteria
- [ ] Part 1, with qjs messages, early errors, e2e checked against qjs, difftest cases,
      and the `decorators` feature promoted to active (report the numbers).
- [ ] strip-types: the ~134 decorator files now match too (report the new rate).
- [ ] Part 2 behind a flag (`CompileOptions`, `djs run --experimental-decorators`, a Sandbox
      option), the dquickts legacy test passing in difftest.

## Log

- Part 1 (TC39 decorators + `accessor`), design D1650 / docs/compiler.md 7.11, after
  qjs's js_apply_decorators: runtime in builtins/decorators.ts (`%DecoratorStart`,
  `%ApplyDecorators`, `%DecoratorRunInits`, `%DecoratorRunExtras`, builtin-closure kinds
  20-23), `Symbol.metadata` (heap atom, interp `WellKnownSymbol`, outside `touches`:
  one line each in heap/atom.rs and interp/run/intrinsics.rs, plus the closure table).
  Compiler: compiler/decorators.rs (decorator lists, synthesized private access arrows
  and auto-accessor getter/setter, the application code), class.rs (elements, `accessor`,
  hidden `<dec>N` bindings in qjs's numbering), stmt.rs/expr.rs entry points.
- qjs messages and positions match for the early errors tried (compiler tests `errors`).
- test262: the `decorators` feature is active (tests/test262.toml); its 18 tests pass,
  ratchet unchanged (active: no new failures, 205 expected).
- difftest: dquickts tests/test_decorators.js imported (7 cases; test_misc is
  `skip-djs`: it uses direct eval, D014), language/decorators.js (= e2e decorators.js),
  ts/decorators.ts; all pass plain, `--suspend-every 97` and `--gc-stress`. The
  dquickts `@dec function f() {}` rejected-syntax case passes now (ratchet updated).
- strip-types: 4085 → 4307/4802 match, 0 differ, 717 → 495 errors. Decorators of members
  without code are erased with the member (as ts-blank-space). The remaining decorator
  errors are deliberate TypeScript errors, and parameter decorators (part 2).
- qjs divergence: qjs rejects `@dec abstract class C {}` (TypeScript), djs accepts it.

## Follow-ups

## Result
