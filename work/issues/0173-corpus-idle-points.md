# 0173: Idle points in the difftest corpus (`host.later`)

status: done
agent: infra
branch: issue/0173-corpus-idle-points
depends-on: -
touches: tests/corpus/prelude.js, scripts/harvest/ (the importer's fake tools) and the regenerated tests/corpus/codemode-harvest, crates/djs-difftest (djs and qjs glue), TESTING.md
guide: -

## Goal
The corpus's fake tools resolve through promise jobs, so a djs run never becomes idle and a
dump-at-every-idle-point mode (0172) would have nothing to test. Make every fake tool call a
real async host call in djs, without changing what qjs records.

## Acceptance criteria
- [x] An async host function `host.later(v)`: the djs runner delivers `v` at the next idle
      point (one per idle point, in call order); the qjs driver makes it `Promise.resolve(v)`.
- [x] The corpus prelude's fake tools and the harvest importer's in-script fakes return
      through it; the harvested cases are regenerated.
- [x] All .expected files unchanged (qjs), the ratchets unchanged, and the existing stress
      modes still pass; if a case's output depends on job vs. idle ordering, list it.
- [x] Count of idle points per case reported (the 0141 prototype found 1,057 in 300 cases).

## Notes
Parallel with 0169 and 0170.

## Log

- tests/corpus/prelude.js: `host.later(value, rejected)`; echo/fail answer through it;
  non-enumerable global `later` for cases with their own fakes; a host without `later`
  falls back to Promise.resolve/reject (djs-bench, the portable research crate).
- djs glue (crates/djs-difftest/src/djs.rs): `later` is an async host function; on
  `Status::Idle` the oldest pending call is settled with its argument (rejected if the
  second argument is `true`), one per idle point, counted. qjs driver: Promise.resolve /
  Promise.reject.
- Harvest: build.py's templates use `later` (tool fakes are plain functions returning
  `later(...)`, models fakes via `answer()`, so qjs takes the same jobs as before);
  `build.py --refresh DIR` re-renders the committed cases from their parts (answers,
  code, stored values). All 300 rewritten; scripts byte-identical, no privacy re-review
  needed.
- Runner: `RunResult.idle_points`/`Outcome.idle_points`; a stress variant with the same
  transcript but a different idle count is a stress mismatch; the total prints after the
  summary, `--idle-points` lists cases.
- Results: qjs over the whole corpus matches every .expected (h271 once differed by
  `ms: 1` vs `0` under load: a wall-clock flake of qjs, passes on rerun, pre-existing).
  djs: same 5 known failures (qjs string spread), ratchet unchanged. Stress
  (--suspend-every 97/1009, --gc-stress, --determinism) OK on both backends for
  codemode, codemode-harvest, result, console.
- Idle points: 1,194 in 294 cases (codemode 30 in 6, codemode-harvest 1,164 in 288; 12
  harvested cases call no tool). 0141's prototype counted 1,057 in 300.
- No case's output depends on job vs. idle ordering. The glue test's `later.js` shows
  the difference deliberately (djs-only assertion).
- Docs: TESTING.md (codemode model, djs glue, --idle-points), scripts/harvest/README.md,
  D2590. Guide: none affected.

## Follow-ups

- h271 measures `Date.now()` deltas; under qjs the wall clock makes it flaky under load
  (`ms` 0 vs 1). Harmless for djs (fixed clock), but `--update` on a loaded machine could
  record 1. Could normalize in the case or the fake.

## Result

Every fake tool call in the corpus (prelude tools, harvested tools and `models`) is an
async host call in djs, answered at the next idle point; under qjs nothing changed. The
runner counts idle points per case and checks them across stress modes, ready for
0172's `--dump-every-idle`. Re-render harvested cases with `build.py --refresh`.
