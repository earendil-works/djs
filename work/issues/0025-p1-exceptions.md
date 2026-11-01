# 0025: P1-3 Exceptions (compiler side) + P1-4 leftovers

status: open
agent: -
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
- [ ] All try forms compile per 7.4. Nested try/finally with return, break and continue
      through any number of finally blocks, and labeled break/continue crossing them.
      Exceptions thrown inside catch and finally.
- [ ] Catch parameter scoping (and the early errors: redeclaring the catch parameter
      with let/const in the catch block). Destructuring catch parameters may stay "not
      supported yet" until P1-6.
- [ ] e2e programs (checked against qjs), plus disassembly snapshots for the finally
      interactions. Programs that need interp pieces still landing in 0022 go into the
      ignored `_0022` test, following the 0024 pattern.
- [ ] P1-4: a list of what was missing and is now fixed, in the Log.
- [ ] Error `stack` from the line table is an interpreter feature. Note what the
      compiler must provide (it presumably already emits the line table) and leave the
      `stack` property itself as a follow-up.
- [ ] The test262 compile sweep before/after numbers.

## Log

## Follow-ups

## Result
