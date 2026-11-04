# 0211: Plain dump fuzzing, round 2: wider generator, new seeds

status: open
agent: -
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

## Follow-ups

## Result
