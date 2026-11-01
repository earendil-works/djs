# 0082: The long tail: the remaining test262 active and difftest failures

status: in-progress
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
- [ ] Cluster all remaining failures by root cause (the first error line plus a look at
      each cluster) and classify each as: fix now, by-design difference (record it in
      TESTING.md / a decision with the reason, e.g. sloppy-only semantics), or belongs to
      another area (send the list to the coordinator for routing).
- [ ] Fix the compiler-side ones (the compound/logical assignment clusters look like
      evaluation-order or reference semantics) and the small builtin gaps that nobody else
      owns (Math.sumPrecise, Error.isError, Math.f16round, Function.prototype/length),
      with tests, and update the ratchets.
- [ ] The same for the 9 difftest failures.
- [ ] A short report: what's left and why, as the "known differences" section in TESTING.md.

## Log

## Follow-ups

## Result
