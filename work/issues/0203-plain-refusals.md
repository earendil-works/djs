# 0203: Plain encodings for the remaining refusals (using, yield*, destructuring/assignment/case sites, finally by break, for-await close, iterator helpers, private fields path)

status: in-progress
agent: pjr
branch: issue/0203-plain-refusals
depends-on: 0202
guide: -
touches: vm/dump*.rs, compiler/ (records if needed), tests/dump goldens

## Goal
docs/plain-js-dumps.md section 8 "Recommendation and plan", step 4.

## Context
docs/plain-js-dumps.md (the design, results and plan: read all of it), the prototype in
work/research/0199/ (plain.mjs, normalize.mjs, freevars.mjs, the harness and chain checks),
decisions D2830-D2833, and today's format 2 exporter (vm/dump*.rs, D2610-D2628, D2730-D2735,
D2800-D2802). Acceptance at every merge: dumps run as plain Scripts in djs **and node** and
continue identically to the uninterrupted run over every corpus idle point they cover,
re-dumps stay bounded (cap chains at 50 links), and the existing gate checks stay green.
djs is unreleased (D024): no compatibility code for format 2 once it is retired.

## Log
- pjx (before handing over): cef48dc2, plain dumps of the v2 and golden programs at every
  idle point with the ratchet tests/dump/v2/plain-refusals.txt
  (`DJS_UPDATE_PLAIN_REFUSALS=1 cargo test -p djs-core --lib plain_dump`).
- pjr: `super` and `new.target` in continuations. A continuation of a method that uses
  `super` is a method of `{ __proto__: <home's prototype> }` (`Act::super_proto`); a site
  inside `super[...]` writes `super[` again; the exporter refuses when a `super.base`/
  `super.this` read before the site is not what K's `super`/`this` give (prototype changed
  since). `new.target` is undefined in K as in the async function/generator (never
  constructed); in an arrow it is refused. Also: a site in the value of a computed key
  takes the key from `object.key@`. v2 refusals 88 -> 68; corpus plain-node unchanged
  (1187 links, 0 differ).
- pjr: the first merge's gate caught a chain bug the djs-core chain test let through (a
  method continuation's `segs`/`scaf` counted from the `({ __proto__: ` wrapper, not the
  method's own text). Fixed; the chain test's refusals are now a ratchet too,
  tests/dump/v2/plain-chain-refusals.txt (same update variable), so a chain that stops
  earlier fails. It lists 4 origin-tracking gaps for 0203: a frame with no value for
  `call.callee`/`array.element` (stopped again inside its rewritten statement) and
  `for-in.iterator` (for-in scaffold).

## Follow-ups

## Result
