# 0208: Fuzz plain dumps: generated codemode-like programs, every idle point, djs and node

status: in-progress
agent: pjz
branch: issue/0208-plain-dump-fuzz
depends-on: 0201, 0202 (on main)
guide: -
touches: crates/djs-difftest/ (a generator mode or a new small tool crate), scripts/, tests/corpus (new cases that reproduce findings), work/research/0208/

## Goal
The corpus gate leg (every idle point, chains of 50, djs and node) is green, but real bugs
still slip through because the corpus lacks the shape: pi crashed on
`return { calls: [await a(), await a(), await a()] }` (fixed in 0203 with
codemode/nested-literals.js), and pjs saw a getter of an object literal that captures a
variable (`{ get m() { n++; … } }`) written as `get: ,`. Find these systematically.

1. Build a generator of small async programs in the style of pi codemode: awaits of host
   tool calls nested in every expression and statement position (literals, calls, member
   targets, destructuring, template literals, conditionals, logical operators, loops of
   every kind, try/catch/finally, switch, labeled break/continue, closures and classes
   capturing frame variables, getters/setters, generators and async generators, Promise.all
   over mapped async functions, `using`). Deterministic from a seed; programs stay short and
   readable.
2. Run each with frozen builtins (D2836) through the existing machinery: uninterrupted run vs
   a plain dump at every idle point and chains of dumps, in djs and node (difftest
   `--plain-out`, scripts/plain-node.mjs). Classify each failure: wrong result, crash,
   refusal (expected per tests/plain-refusals.txt / D2878, or a new one).
3. Minimize each distinct failure to a small case and add it to tests/corpus (so the gate
   covers it) — mark it as an expected failure where the gate supports that, or hand it to
   the coordinator to route the fix (the exporter is being changed by pjs/0207 and
   pjr/0205; do not fix exporter code yourself unless the coordinator says so).
4. Keep the fuzzer runnable as a script with a seed and a time/case cap (no unbounded
   loops), documented in TESTING.md; not a gate leg unless it is fast and deterministic.

## Log

## Follow-ups

## Result
