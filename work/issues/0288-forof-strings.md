# 0288: for-of over strings without a trip through `step`

status: done
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
- [x] re-measured vs QuickJS 2026; run_hot unchanged
- [x] size recorded; gate green; decision recorded

## Log

2026-11-21 (pidle)
- Applied cleanly on main 7323532f. slope.py (`--qjs2`): for-of over 10 ASCII units 22.3k →
  18.3k; for-of over an Array +2.9%: its declined last step now planned twice. Finished the
  last step in the same path (result object first): Array 11.6k → 11.5k, ASCII → 17.9k
  (QuickJS 2026 13.2k), Latin-1 23.0k → 19.3k, wide 23.8k → 20.1k.
- hotcode.sh: run_hot's summary identical to main's but for addresses.
- +301 B gz (431,736), slim +291 (365,655); ceilings raised (D3570).
- Tests: lib, active test262, difftest stress + determinism and plain-dumps --lazy on
  language/builtins/codemode; corpus case language/for-of-strings.js.

## Result
for-of over strings 20% fewer instructions (D3570), arrays unchanged; +301 B gz from the
allowance.
