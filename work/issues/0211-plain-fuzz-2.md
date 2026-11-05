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
  (plain-xfail, pjr), finally that awaits then always `continue`s (refused; pjs's
  finally-dead-end covers it, mine dropped). Batch 4: parser bug, a 2+ character regex
  literal after a block inside `try` (pjp/0217 adds the case with its fix, mine dropped).
  Seeds 4401–4800: only the nested for-in shape again.

## Follow-ups

## Result
