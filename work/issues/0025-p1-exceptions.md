# 0025: P1-3 Exceptions (compiler side) + P1-4 leftovers

status: done
agent: lexer
branch: issue/0025-p1-exceptions
depends-on: 0024
touches: crates/djs-core/src/compiler/, docs/compiler.md (corrections), crates/djs-core/src/bytecode/ (additive only)

## Goal
docs/compiler.md section 10, row P1-3: `try`/`catch`/`finally` (with and without a
catch binding), `throw` in all positions, handler tables, `gosub`/`ret` for `finally`,
including `return`, `break` and `continue` crossing `finally` blocks (section 7.4, D1084).
The interpreter already executes throw, the handler tables and gosub/ret (0015). Also
check P1-4 (lexical scoping) for anything not covered by 0014/0024 and close the gaps
(the switch lexical scope, TDZ edge cases, and redeclaration early errors in all
positions).

## Acceptance criteria
- [x] All try forms compile per 7.4. Nested try/finally with return, break and continue
      through any number of finally blocks, and labeled break/continue crossing them.
      Exceptions thrown inside catch and finally.
- [x] Catch parameter scoping (and the early errors: redeclaring the catch parameter
      with let/const in the catch block). Destructuring catch parameters may stay "not
      supported yet" until P1-6.
- [x] e2e programs (checked against qjs), plus disassembly snapshots for the finally
      interactions. Programs that need interp pieces still landing in 0022 go into the
      ignored `_0022` test, following the 0024 pattern.
- [x] P1-4: a list of what was missing and is now fixed, in the Log.
- [x] Error `stack` from the line table is an interpreter feature. Note what the
      compiler must provide (it presumably already emits the line table) and leave the
      `stack` property itself as a follow-up.
- [x] The test262 compile sweep before/after numbers.

## Log

- 2026-11-01: Before: 23,482 files compile, 27,156 "not supported yet".
- `try` in all forms (catch with or without a binding, finally, both). A token scan
  finds the `finally` before the try block is parsed. A `Finally` BlockEnv records the
  label and the base depth, and `stmt_depth` tracks the operand depth at statement
  boundaries (base + 2 inside a finally block). `break`/`continue` (labeled or not) drop
  to each finally's base, `undefined; gosub; drop`, then drop to the target's depth and
  jump. `return` cuts the stack under the value (`nip`), gosubs every enclosing finally,
  then returns; a bare `return` inside a finally region pushes `undefined` first.
  Handler tables come from labels after the final fixups. D1250.
- The interpreter needed nothing new: handlers, `gosub`/`ret`, `throw` and the catch
  value all worked as they were (0015). No `_0022` test was needed: all 26
  `end_to_end_exceptions` cases and the 14 `end_to_end_lexical` cases pass today and
  match qjs (`work/scratch/0025/qjs_check.py <test fn>`).
- P1-4 review: what was missing and is now fixed:
  - catch parameter scoping: a `Catch` scope shared with the catch block;
    `catch (e) { let e }` / `{ function e(){} }` are redefinitions; `var e` is allowed and
    assigns the parameter (qjs, Annex B.3.4); captured catch parameters get a fresh Box
    per entry;
  - `function f(){} let f` at function or script level now says "invalid redefinition
    of global identifier" / "of a variable" (was "lexical identifier"), as in qjs;
  - TDZ checks for catch parameters are not needed (initialized by the handler);
  - checked and already right: switch case scope (TDZ never elided, redeclaration
    across cases), for-head `let` vs body `var`, block function duplicates, shadowing
    in nested blocks, `typeof` in TDZ, TDZ through closures, per-iteration copies
    (`end_to_end_lexical`).
- Error `stack` (follow-up for the interpreter): the compiler already provides what it
  needs. Every instruction that can throw has a line table entry (`FuncTemplate::LINES`,
  1-based line and code-point column; `throw` maps to the `throw` keyword, calls to the
  `(`, member accesses to the `.`/`[`). Functions have `NAME` and the `Script` has the
  file name. The interpreter walks the frames (the current pc for the top frame,
  `ret_pc - 1` for callers) and formats `at name (file:line:col)` like qjs.
- After: 24,742 files compile, 25,786 "not supported yet", and no new missing early
  errors (the 8 that compile are all module-only).

## Follow-ups

- Interpreter (0022 or later): the `stack` property of Error objects from the line
  table (see the Log for what the compiler provides).
- P1-5: for-of must close its iterator when `break`/`return`/`throw` leave it. The
  BlockEnv/`stmt_depth` machinery here is the place: a for-of env needs an
  `iterator_close` on the way out, like the finally gosubs.
- Destructuring catch parameters (P1-6).

## Result

`try`/`catch`/`finally` compile (D1250): handler tables, `gosub`/`ret`, and `return`,
`break`, `continue` (labeled too) through any number of finally blocks, with
exceptions thrown in try, catch and finally blocks. The catch parameter is scoped with
the catch block. P1-4 gaps are closed (see the Log).

Tests: snapshot `exceptions.js`, 13 new error cases, `end_to_end_exceptions` (26
programs) and `end_to_end_lexical` (14), all matching qjs and passing on today's
interpreter (no ignored test needed). test262 compile sweep: 23,482 → 24,742 compile;
"not supported yet" 27,156 → 25,786.
