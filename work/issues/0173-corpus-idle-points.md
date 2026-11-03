# 0173: Idle points in the difftest corpus (`host.later`)

status: open
agent: -
branch: issue/0173-corpus-idle-points
depends-on: -
touches: tests/corpus/prelude.js, scripts/harvest/ (the importer's fake tools) and the regenerated tests/corpus/codemode-harvest, crates/djs-difftest (djs and qjs glue), TESTING.md
guide: -

## Goal
The corpus's fake tools resolve through promise jobs, so a djs run never becomes idle and a
dump-at-every-idle-point mode (0172) would have nothing to test. Make every fake tool call a
real async host call in djs, without changing what qjs records.

## Acceptance criteria
- [ ] An async host function `host.later(v)`: the djs runner delivers `v` at the next idle
      point (one per idle point, in call order); the qjs driver makes it `Promise.resolve(v)`.
- [ ] The corpus prelude's fake tools and the harvest importer's in-script fakes return
      through it; the harvested cases are regenerated.
- [ ] All .expected files unchanged (qjs), the ratchets unchanged, and the existing stress
      modes still pass; if a case's output depends on job vs. idle ordering, list it.
- [ ] Count of idle points per case reported (the 0141 prototype found 1,057 in 300 cases).

## Notes
Parallel with 0169 and 0170.

## Log

## Follow-ups

## Result
