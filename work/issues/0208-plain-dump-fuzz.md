# 0208: Fuzz plain dumps: generated codemode-like programs, every idle point, djs and node

status: done
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
- Tool: scripts/plain-fuzz/{gen,fuzz}.mjs (D2940); difftest `--plain-from K` (a chain
  starting at idle point K, so sites are dumped from original functions too);
  `// difftest: plain-xfail <reason>` (D2941) with stale reporting in the difftest and
  plain-node.mjs. plain-node prints the reason line after an unexpected `=== throw/crash`.
- First run: 300 programs (seeds 1..300), ~4,800 idle points, ~30 signatures. The two
  known shapes: nested literals (fixed) do not show; `get: ,` does (~60 of 300 programs).
- First batch reduced by hand into tests/corpus/codemode/plain-fuzz/ (reported to the
  coordinator, routed): async-generator-for-await (plain-xfail, pjs), getter-capturing-
  frame-variable (refusal, pjd/0209), binary-two-awaits, binary-three-awaits,
  template-two-awaits, function-declared-in-loop, await-using-redump (refusals, pjr),
  await-in-assignment-target / await-in-pattern-target (deliberate refusals), and
  language/unreachable-destructuring-default.js (compiler bug, ratchet, pjk/0210).
- Batches 2 and 3 (reported, routed by the coordinator): accessor-pair-after-site,
  class-instance-loop-closure, generator-capturing-for-of, class-instance-in-idle-function
  (plain-xfail); callee-and-argument-awaits, assign-member-nested-call,
  finally-await-in-loop, class-from-waiting-closure, class-static-in-promise-all,
  integer-key-equal-value, language/plain-fuzz/await-using-hoisted-function (refusals).
- `using` cases live in language/plain-fuzz/: the slim build (codemode corpus in the slim
  gate leg) has no `using`.
- 0210 fixed the compiler bug: language/unreachable-destructuring-default.js passes now.
- After rebasing onto 0203/0205/0207 work: function-declared-in-loop, finally-await-in-loop
  and both await-using cases no longer refuse (refusals ratchet updated).
- Batch 4: class-in-switch-block (two classes in one continuation block with one temporary
  name) and for-await-break (missing slot for-await.next); both fixed on main by pjs before
  this merge (pjs added its own for-await-break case, which replaced mine).
- Fuzzer throttled (owner's rule): 4 niced jobs by default, waits while load > 30;
  `--known FILE` folds reported signatures into one line. Seeds 1001..1400 with the known
  list: only the accessor-pair bug (fixed since).
- Corpus dump-size score raised for the new cases' dumps (+25 KB raw).
- Guide: none affected.

## Follow-ups
- Rerun the fuzzer after the open fixes land (the refusal family of re-dumps, class
  placement): `node scripts/plain-fuzz/fuzz.mjs --seed 2001 --cases 400 --time 900`;
  most of its signatures were masked by the big ones (`get: ,`, the async generator
  ordering) in the first runs.
- The generator has no private fields (refused by design, D2878), TypeScript, `eval`,
  Proxies, Maps/Sets, Dates or regexps; adding Map/Set/Date values and labeled blocks
  would widen the data side.
- `reduce` is line- and bracket-based and slow (it runs every chain per candidate);
  hand minimizing was faster for most findings.

## Result
- `node scripts/plain-fuzz/fuzz.mjs [--seed S] [--cases N] [--time SECS] [--known FILE]`:
  seeded codemode-like programs (gen.mjs) through the difftest's plain-dump variant with a
  chain starting at every idle point (`djs-difftest --plain-from K`, D2940) and node
  (plain-node.mjs); distinct failure signatures with the smallest seed. `fuzz.mjs check
  FILE...`, `fuzz.mjs reduce FILE --sig TEXT`, `fuzz.mjs gen S`. TESTING.md documents it.
- `// difftest: plain-xfail <reason>` (D2941) marks a known plain-dump bug in a corpus case;
  stale marks are reported by the difftest / plain-node.mjs.
- 26 corpus cases in tests/corpus/codemode/plain-fuzz/ and language/plain-fuzz/ (+ the
  compiler case language/unreachable-destructuring-default.js), from ~25 distinct
  signatures over ~1,100 programs / ~19,000 idle points, all reported and routed. All
  but two are fixed on main; the two left are deliberate refusals (await in an assignment
  target / in a pattern around the site), listed in tests/plain-refusals.txt.
