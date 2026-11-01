# 0082: The long tail: the remaining test262 active and difftest failures

status: done
agent: lexer
branch: issue/0082-long-tail
depends-on: 0065
touches: crates/djs-core/src/compiler/, plus small fixes elsewhere where a cluster needs them (logged; coordinate interp/heap/builtins changes through the coordinator, since infra, heap and num are active there)

## Goal
test262 active is at ~99.5%: 203 expected failures remain (tests/test262/expected-failures.txt),
plus 9 difftest failures. Clusters on main (2026-11-01): compound-assignment 33,
logical-assignment 12, eval-code/indirect 12 (sloppy-eval leaks; partly by design, D014),
Math.sumPrecise 10, Error.isError 10, class 17, variable 8, Math.f16round 5,
Function/prototype 5, object 4, Function/length 4, …

## Acceptance criteria
- [x] Cluster all remaining failures by root cause (the first error line plus a look at
      each cluster) and classify each as: fix now, by-design difference (record it in
      TESTING.md / a decision with the reason, e.g. sloppy-only semantics), or belongs to
      another area (send the list to the coordinator for routing).
- [x] Fix the compiler-side ones (the compound/logical assignment clusters look like
      evaluation-order or reference semantics) and the small builtin gaps that nobody else
      owns (Math.sumPrecise, Error.isError, Math.f16round, Function.prototype/length),
      with tests, and update the ratchets.
- [x] The same for the 9 difftest failures.
- [x] A short report: what's left and why, as the "known differences" section in TESTING.md.

## Log

- Clustered all 203 + 7 failures (the difftest file had 7 entries, not 9). D1820 has
  the fixes; TESTING.md "Known differences" has the rest.
- Compiler and interpreter order of evaluation: compound assignment, update and logical
  assignment on `o[k]` (new op `get_elem2`), plain `o[k]` on a nullish base,
  ToNumeric(lhs) before the rhs in binary operators, and `{ [k]: v }` key conversion
  (new op `to_propkey`). That made 69 tests pass. Logical assignment NamedEvaluation
  gave 6 of them.
- Scopes: a separate var scope for bodies with non-simple parameters (21
  `scope-paramsbody-var-open` tests), a separate block scope for `catch ([pattern])`
  (1), and `(a?.b)()` keeping `this` (1).
- Interpreter and builtins, outside `touches:` and logged here, each small:
  - interp/run.rs: `primitive_operands` checks, `get_elem2`/`to_propkey`, the
    arguments object's `callee`, Script function declarations
    (CanDeclareGlobalFunction), and qjs's "no setter for property".
  - interp/run/intrinsics.rs: %ThrowTypeError% property order, the Object.prototype
    immutable prototype, and the `F64F16round` / `F64SumPrecise` leaves.
  - builtins: math.ts (`Math.f16round`, `Math.sumPrecise`), error.ts (`Error.isError`),
    object.ts (`Object(v)` with a subclass new.target), promise.ts (finally calls
    `then` with one argument), number.ts (`toString` length), function.ts
    (`@intrinsic` on ThrowTypeError).
  - Rust: crates/djs-core/src/builtins/math.rs (the f16 rounding and the exact sum,
    with unit tests).
- test262 active: 203 → 56 expected failures. difftest: 7 → 4 (type-errors' message,
  optional chaining, test_math).
- Tests: tests/data/e2e/long_tail.js (expected from qjs), compiler snapshots
  (operators, objects, literals), the builtins::math unit tests.

## Follow-ups

Routed to the coordinator (TESTING.md "Known differences"):
- Function.prototype must be callable (9 test262 tests; heap/realm).
- [[Set]] of an array's `length` with a non-Number value skips ArraySetLength's
  conversions (4; interp).
- Typed arrays on resizable buffers that shrink during fill/freeze/subarray (3).
- difftest: `btoa`/`atob` and `DOMException` (qjs host extensions: by design or a
  host-API issue?), and the immutable ArrayBuffer proposal (2 cases).

## Result

test262 active expected failures went from 203 to 56, and difftest failures from 7 to
4. The 40 by-design tests (always strict, indirect eval is strict, Annex B
`__lookupGetter__`, one shared with qjs) and the 16 open ones are listed with reasons in
TESTING.md "Known differences". Decision D1820.
