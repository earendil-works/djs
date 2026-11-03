# 0181: Should djs provide atob/btoa and performance.now? (research)

status: open
agent: -
branch: issue/0181-atob-performance
depends-on: -
guide: -
touches: docs/ (a short report section, e.g. docs/codemode-feature-usage.md addendum or a new note), work/scratch/0181/; implementation only if the coordinator approves after the report

## Owner
"Investigate if we should add atob/btoa and performance.now. I think the existing pi
codemode has access to those."

## Facts to start from
- djs already defines `atob` and `btoa` (typeof is "function"; base-families lists them);
  check they behave like the web spec (Latin-1 only, InvalidCharacterError as DOMException,
  whitespace handling, padding) against node and qjs.
- `performance` is undefined in djs. 0130's analyzer listed `performance` among
  QJS_ONLY_GLOBALS (quickjs-wasi has it). pi's host code uses performance.now() on the host
  side (packages/codemode/src/runtime/execution.ts); check what pi's *sandbox* exposes to
  scripts today (QuickJS, prelude-source.ts, the wasm host's globals).

## Questions
1. What do pi's code mode scripts see today for atob/btoa/performance (QuickJS + pi's
   prelude), and do any harvested scripts use them (work/scratch/0130, 0149 data;
   scripts/harvest/features.py)?
2. atob/btoa: conformance gaps in djs vs the spec/node; tests to add.
3. performance.now: semantics under determinism (D1680 deterministic mode: the clock moves
   only with host results; Date.now's handling), fuel, snapshots/dumps (its time origin);
   whether it should be a host-provided global (like Date's clock) or an engine builtin;
   what `performance` object members to provide (now, timeOrigin?, mark/measure no);
   cost in size; the slim build.
4. Recommendation with a small implementation plan.

## Deliverable
A short report (in the issue Result and a docs note); no engine changes until approved.

## Log

## Result
