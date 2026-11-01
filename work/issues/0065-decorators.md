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

## Follow-ups

## Result
