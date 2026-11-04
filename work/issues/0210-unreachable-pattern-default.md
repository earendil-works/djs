# 0210: Compiler: array destructuring with a default in unreachable code

status: done
agent: pjk
branch: issue/0210-unreachable-pattern-default
depends-on: -
guide: -
touches: crates/djs-core/src/compiler/ (pattern.rs / handler depth tracking), tests/corpus or e2e

## Goal
`function f() { return 1; let [a = 1] = []; }` fails to compile with SyntaxError
"internal compiler error: unreachable relative handler depth" (array destructuring with a
default in code after return/throw). Node runs it. Found by 0208's fuzzer; it also shows up
as "plain load: compile: … relative handler depth" when a dump contains such code. Fix the
root cause, check related shapes (object patterns, nested defaults, for-of heads, after
break/continue/throw, inside try/finally), and add tests. Decision IDs D2960–D2964 if needed.

## Log
- Root cause: `resolve_relative_depths` (compiler/resolve.rs) resolves a relative
  handler's depth from the stack walk's depth at its anchor label. In dead code the
  anchor is never reached, the fixpoint stalls, and it raised the internal error. Object
  patterns were fine (no iterator, no close handler); every shape with an array pattern
  default (nested, in object patterns, for-of heads, assignment patterns, after
  return/throw/break/continue/labelled break/endless loop, in try/catch/finally, class
  methods, async) failed, and so did generators with `yield`/`yield*` in dead code
  (`yield*` alone too).
- First fix (drop stalled handlers whose regions the walk never reaches) failed for
  `yield` in a dead pattern default: the yield's return path is the target of an
  absolute handler in the same dead code, which the walk seeds, so the pattern region
  looked reached. Final fix: on a stall, `drop_dead_handlers` recomputes reachability
  from the entry alone, activating a handler (resolving its relative depth) once code in
  its region is reached, and drops all handlers never activated. Fast path unchanged.
  D2960.
- Tests: `compiler::tests::dead_pattern_handlers_dropped` (handler counts + runs),
  corpus case `language/dead-code-patterns.js` (34 shapes, live patterns beside dead
  ones, idle points); passes plain, --suspend-every 1/97, --gc-stress, --determinism,
  --dump-every-idle, --plain-dumps. test262 for-of/assignment/variable/yield/generators:
  no change.
- Guide: no page affected.

## Follow-ups

## Result
Functions with array-pattern defaults (or `yield*`, or `yield` inside patterns) in
unreachable code compile and run. When relative handler depths stop resolving, the
compiler (`drop_dead_handlers` in compiler/resolve.rs) recomputes reachability from the
entry and drops every exception handler no execution reaches (D2960); code without such
handlers compiles exactly as before. Tests: `compiler::tests::dead_pattern_handlers_dropped`
and the corpus case `tests/corpus/language/dead-code-patterns.js`. Known gaps: none
found; dead code is still emitted (only its handlers go).
