# 0288: for-of over strings without a trip through `step`

status: in-progress
agent: pidle
branch: issue/0288-forof-strings
depends-on: 0287, 0286
touches: crates/djs-core/src/interp/run/construct.rs (the declined-instruction path), crates/djs-core/src/interp/run/iter.rs, tests/corpus
guide: -
decisions: D3570-D3579

## Goal
Land 0287's parked patch (work/research/0287/for-of-strings.patch, D3560's note): the tier-0
loop declines `iter_next_loc` for String Iterators; finish that step out of line (one
`iter_plan`, ASCII units from the one-unit cache) instead of in `step`. 0287 measured a
for-of over a 10-character string at 22.9k → 17.3k wasm instructions (QuickJS 2026 11.2k),
+243 B gz. Re-measure on current main (slope.py --qjs2), check run_hot's code
(hotcode.sh), raise the ceiling from the owner's allowance (D3510) by what it needs.

## Acceptance criteria
- [ ] re-measured vs QuickJS 2026; run_hot unchanged
- [ ] size recorded; gate green; decision recorded

## Log

## Result
