# 0024: P1-1 Functions and closures (compiler side)

status: done
agent: lexer
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
- [x] All the constructs above compile per the note (sections 5.3 to 5.7, 6.3), with
      early errors (duplicate params with defaults or rest, strict reserved names,
      `arguments`/`eval` as binding names, `new.target` outside functions, …).
- [x] Per-iteration `let` bindings in `for` loops with closures (`rebox`, 5.9).
- [x] Function objects get `name` and `length`, and `prototype` for constructors (decide
      where: the compiler emits a `make_closure` flag and the interpreter creates them;
      coordinate with 0022 via the log if interp changes are needed, and keep those
      changes minimal and additive).
- [x] Disassembly snapshots and e2e programs (through the interpreter) for closures:
      counters, captured loop variables, nested captures across three levels, recursion,
      mutual recursion via hoisting, `this` in methods called via `obj.f()`, arrows
      capturing `this` and `arguments`, default params referencing earlier params, rest.
- [x] The test262 compile sweep: the number of "not supported yet" files drops; record
      before and after.

## Notes
- 0022 runs in parallel in `interp/` and `vm/`. If you need an interpreter change,
  coordinate through the coordinator rather than editing interp/ directly, unless it is
  a tiny additive fix (log it).

## Log

- 2026-11-01: Before: the test262 compile sweep had 11,438 files compiling and 39,903
  "not supported yet".
- Compiler: `function.rs` (declarations, expressions, arrows, parameters, bodies, the
  directive check), a function stack in the parser (`begin_function`/`end_function`;
  finished functions by id), pseudo-bindings found by name and declared in the
  innermost non-arrow function (`this`, `arguments`, `new.target`), and named function
  expressions bound in a scope outside the function scope. Hoisted function declarations
  are registered per scope and created at its `enter_scope`. NamedEvaluation is done
  with compile-time name hints.
- Resolution (D1081) now has three passes over all functions: classify (children first;
  free names walk into enclosing functions, mark the binding captured and add capture
  slots on the path), storage (captured → Box), emit (children first, so capture
  descriptors can use the parent's Box slots). Lexical bindings reached through
  captures are always TDZ-checked.
- Bytecode, additive: `rest index` (args from `index` on as an array) and the phase-1
  pseudo-op `iter_scope s` (per-iteration `rebox`). `DefineElem` became `push 1`, the
  same one-character fix 0022 made, so the rebase is trivial.
- Interpreter needs were routed through the coordinator: special_object kinds, closure
  `length`/`name`/`prototype`, construct. 0022 does them; `rest` comes after this lands.
- Early errors with qjs messages and positions: duplicate parameters, "use strict"
  with non-simple parameters, eval/arguments as function or parameter names, rest not
  last, missing formal parameter, `new.target` outside functions (arrows at top level
  too), "invalid redefinition of parameter name", var/let conflicts with block-level
  functions, labeled function declarations.
- After: 23,482 files compile and 27,156 are "not supported yet". The negative parse
  tests that compile are now only module-only cases. The sweep found the missing
  var-vs-block-function redeclaration errors.
- Snapshots `closures.js` (body mode) and `functions.js` (Script). `end_to_end_functions`:
  26 programs, all cross-checked with qjs (`work/scratch/0024/qjs_check.py`). Pure
  closure cases pass on the current interpreter; the rest need 0022.

- Per the coordinator, landing before 0022: the cases that need 0022's interpreter
  pieces are in the ignored test `compiler::tests::end_to_end_functions_0022`.

## Follow-ups

- 0022 (heap agent): implement `rest` and un-ignore
  `compiler::tests::end_to_end_functions_0022` (11 cases: `this` in methods, arrows
  capturing `this`/`arguments`, `name`/`length`, `new`, `new.target`,
  `arguments.length`, rest parameters, a named function expression).
- Computed-key function names (`{ [k]: () => 1 }`) need a runtime `set_name`.
- Spec gap (compiler.md 5.5): a body `var` named like a parameter shares the parameter's
  binding even with parameter expressions.
- Still "not supported yet": methods, getters and setters in object literals (P1-2),
  classes (P1-7), destructuring parameters (P1-6), async functions and generators
  (P2/P5), spread arguments.

## Result

Functions compile: declarations (hoisted per scope, block-level ones lexical), function
expressions (named ones bind their own name), arrows (expression and block bodies),
default and rest parameters with TDZ between them, `return`, calls and `new`, `this`,
`arguments` and `new.target` (arrows capture them from the enclosing function),
closures over any binding through Boxes (resolution walks enclosing functions, D1081,
D1083), and per-iteration `let` copies in `for` loops (`iter_scope` → `rebox`). Templates
carry `KIND_NORMAL`/`KIND_ARROW`, `NAME` (with NamedEvaluation), `LENGTH`, `PARAMS`
and capture descriptors. The interpreter creates `name`/`length`/`prototype` (D1240).
Parameters follow D1241.

Tests: snapshots `closures.js` and `functions.js`, about 25 new error cases, and
`end_to_end_functions` (18 programs: counters, captured loop variables, three-level
captures, recursion, mutual recursion via hoisting, defaults, TDZ through closures, block
functions). **Un-ignore `compiler::tests::end_to_end_functions_0022` (11 programs) when
0022 lands.** All outputs are checked against qjs (`work/scratch/0024/qjs_check.py`
<test fn name>).

test262 compile sweep: 11,438 → 23,482 files compile; "not supported yet" 39,903 →
27,156. The only negative parse tests that compile are module-only.
