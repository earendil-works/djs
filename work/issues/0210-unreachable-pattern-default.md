# 0210: Compiler: array destructuring with a default in unreachable code

status: open
agent: -
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

## Follow-ups

## Result
