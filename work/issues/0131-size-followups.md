# 0131: Size follow-ups: message builder, decomposition stream, composition tables

status: open
agent: -
branch: issue/0131-size-followups
depends-on: 0126
guide: - (unless a page describes the tables)
touches: crates/djs-core/src/unicode/ (normalization tables + generator), message construction sites (interp/run.rs, date.rs, errors: small edits, announce to perf who works in interp/ in 0129), scripts/size

## Goal
bloat's 0126 follow-ups, speed-free cuts toward the D020 budget (main 460.9 KB, 1.608x):
1. A small non-formatting string builder for the ~100 `format!` message sites (~2–3 KB gz).
2. QuickJS-style recoding of the normalization decomposition stream (~2–3 KB).
3. Derived composition tables with a runtime cache keeping localeCompare/normalize fast
   (~3 KB).

## Acceptance criteria
- [ ] Each cut measured; no workload slower than +1%; size ratchet lowered.
- [ ] Error messages byte-identical (difftest/test262 outputs unchanged).

## Log

## Follow-ups

## Result
