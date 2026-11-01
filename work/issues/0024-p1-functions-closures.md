# 0024: P1-1 Functions and closures (compiler side)

status: open
agent: -
branch: issue/0024-p1-functions-closures
depends-on: 0014, 0015
touches: crates/djs-core/src/compiler/, docs/compiler.md (corrections), crates/djs-core/src/bytecode/ (additive only)

## Goal
docs/compiler.md section 10, row P1-1: function declarations and expressions, arrows,
calls and `new`, `return`, captures and Boxes (the resolver walking into enclosing
functions, D1083), `this`/`arguments`/`new.target`, default and rest params, named
function expressions, and hoisting of function declarations. The interpreter already
executes the closure/Box/capture ops (0015). 0022 (heap agent, running in parallel)
adds the missing interp ops for P0 and `Vm::compile`.

## Acceptance criteria
- [ ] All the constructs above compile per the note (sections 5.3 to 5.7, 6.3), with
      early errors (duplicate params with defaults or rest, strict reserved names,
      `arguments`/`eval` as binding names, `new.target` outside functions, …).
- [ ] Per-iteration `let` bindings in `for` loops with closures (`rebox`, 5.9).
- [ ] Function objects get `name` and `length`, and `prototype` for constructors (decide
      where: the compiler emits a `make_closure` flag and the interpreter creates them;
      coordinate with 0022 via the log if interp changes are needed, and keep those
      changes minimal and additive).
- [ ] Disassembly snapshots and e2e programs (through the interpreter) for closures:
      counters, captured loop variables, nested captures across three levels, recursion,
      mutual recursion via hoisting, `this` in methods called via `obj.f()`, arrows
      capturing `this` and `arguments`, default params referencing earlier params, rest.
- [ ] The test262 compile sweep: the number of "not supported yet" files drops; record
      before and after.

## Notes
- 0022 runs in parallel in `interp/` and `vm/`. If you need an interpreter change,
  coordinate through the coordinator rather than editing interp/ directly, unless it is
  a tiny additive fix (log it).

## Log

## Follow-ups

## Result
