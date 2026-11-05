# 0211: Plain dump fuzzing, round 2: wider generator, new seeds

status: in-progress
agent: pjz
branch: issue/0211-plain-fuzz-2
depends-on: 0208
guide: -
touches: scripts/plain-fuzz/, tests/corpus (new cases), tests/plain-refusals.txt, TESTING.md

## Goal
0208's follow-ups. Widen the generator to what codemode really holds across awaits: Map,
Set, Date, RegExp (incl. lastIndex), typed arrays/ArrayBuffer, Error objects, symbols as
keys, getters/setters on classes, static blocks and static fields (after 0209 item 7 lands),
TypeScript bodies (types, enums, parameter properties, `as`/`satisfies`), JSON round trips,
string-heavy data. Run new seeds (e.g. `--seed 2001 --cases 400 --time 900`, `--known` with
the reported signatures), reduce, report batches to the coordinator grouped by area, add
corpus cases (plain-xfail for wrong results until fixed). Same throttling as 0208 (4 niced
jobs, wait while load > 30) and gate rules (batch merges, check slim/test262-slim locally).

## Log
- gen.mjs widened (D2942): Map/Set/Date/RegExp/typed arrays/Errors, symbol keys, tricky
  strings, JSON round trips, class accessors, static fields/blocks (default on since
  0209's D2950; `--no-statics`), TypeScript bodies for seeds divisible by 3. fuzz.mjs: .ts
  cases, `--known`, reduce keeps the extension and parse-checks TS with node's stripper.
- Seeds 2001–3600 (~22k idle points), every plain run ends normally. Batch 1 (instance
  before its callback, for-await + Promise.all, async method of a temporary, template with a
  plain first part, static field reading a loop variable, class with an async method in a
  loop, TS parameter property): all fixed on main by the time of the rebase. Batch 2:
  sync generator + Promise.all body (refused, pjs).
- Cases: codemode/plain-fuzz/{async-method-of-temporary, static-field-reads-loop-variable,
  template-plain-part-then-awaits, class-with-async-method-in-loop,
  generator-promise-all-body}, ts/plain-fuzz-parameter-property.ts. Mine for instance-
  before-callback and for-await + Promise.all were dropped for main's own versions.
- Seeds 3601–4800 (~23k idle points). Batch 3: nested for-in re-dump loses iterations
  (pjr fixed it; plain case), finally that awaits then always `continue`s (refused; pjs's
  finally-dead-end covers it, mine dropped). Batch 4: parser bug, a 2+ character regex
  literal after a block inside `try` (pjp/0217 adds the case with its fix, mine dropped).
  Seeds 4401–4800: only the nested for-in shape again.
- Seeds 4801–6200 (+ 200 `--big`): batch 5, an await in a case test then one in a later
  case body (refused "its temporaries differ", pjs) and `await using` + `continue` after a
  try/catch in a for-await body (completion.kind, pjr), both fixed on main before this
  merge (plain cases); batch 6, a class with an async method in a nested loop's body
  (refused "class in a nested scope", pjr; refusals ratchet). `fuzz.mjs --big` for longer
  programs.
- Regression set: scripts/plain-fuzz/regress/ (23 programs that found round-2 bugs, frozen
  as text) and `fuzz.mjs regress [--cases N]`: those, then N fresh seeds. To run after each
  pjs or lazy merge.
- gen.mjs: Proxies with traps capturing frame variables, arrays with getter properties
  and index getters, BigInt, iterator helpers live across awaits, `Array.fromAsync`.
  Seeds 7001–8060: no wrong results; two refusals: `Array.fromAsync` in flight (to be
  fixed, pjr; refusals ratchet for now) and for-in over a Proxy (deliberate, noted in
  docs/plain-js-dumps.md §10).
- `fuzz.mjs --lazy` (difftest --lazy, D2990): regress + 300 seeds clean with and without
  lazy functions after lazy batch 3 and 0215's merges. Generator: optional calls, tagged
  templates, async methods with arrows over `this`, labelled blocks, `delete`.
- Seeds 9001–9800: batch 8, an arrow capturing `arguments` of a waiting async function
  (wrong result, plain-xfail) and one capturing `this` in a waiting method (refused);
  batch 9, an array's accessor element before a data element written as a hole
  (plain-xfail) and `continue` out of a try whose finally awaits, re-dumped (refused).
- Regress after 0207's batch (bdd7871b), seeds 9801–10100: three regressions, bisected to
  b4ce8159 (D3003: Promise.all closure calling a body function refused, case
  promise-all-calls-body-function in the refusals ratchet), 356b7194 (D3006: wrong
  results, seeds 9922/9828) and 05e7a4ad (D3005: re-dump text refusal, seed 10014). Their
  programs are in regress/ (s12-*). `regress` now takes its fresh seeds from a random,
  printed start.
- pjs fixed all three (d881a8c4). Seeds 10101–10700 (lazy and --big): a new refusal,
  "crossing callbacks" (for of over a generator's iterator helpers + nested Promise.all
  in an async function; case language/plain-fuzz/crossing-callbacks.js, language/ since
  the slim build has no iterator helpers). regress ignores deliberate refusals
  (regress/deliberate.txt).
- Generator: subclasses with `super` across awaits, null-prototype/frozen/iterable
  objects, sparse arrays, -0/NaN/Infinity, lone surrogates and NUL. Seeds 10701–12000:
  batch 11, a subclass published into a factory before its declaration (load crash,
  plain-xfail) and `super` in an async method re-dumped ("no `<home>`", refused).
  plain-node reads lone surrogates as U+FFFD; the generator deletes with
  Reflect.deleteProperty (frozen objects).
- plain-node maps node's "Cannot assign to read only property 'x' of …" to djs's "'x' is
  read-only" (false alarms at seeds 184922, 4586, from pjr). regress/ gains s15-* (those
  and pjr's D3026 seeds 668041, 82489).
- Housekeeping: dropped the stale refusal entry codemode/plain-class-static-private.js
  (fixed by D2883/D2950). The gate's stale entry in tests/pi/expected-failures.txt was
  "'Node' process lifetime exits promptly after timeout with a real worker": the six
  process-lifetime tests cancel a workload djs's indexOf finishes in milliseconds, so they
  failed on a quiet machine (3 of 3 local runs) and passed now and then under load. Fix
  (coordinator's call): js/pi/pi-tests.ts gives the copied fixture an endless loop
  (failing loudly if pi's workload text changes); all six pass, the entries are gone from
  both pi ratchets (next mode, slim). Checked: next mode twice and slim six times, four of
  them with 6–8 busy processes; one early slim run under load reported one new failure
  that the output did not keep and five more runs did not repeat. docs/host.md updated.

## Follow-ups

## Result
