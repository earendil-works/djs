# 0252: Size headroom: generated builtins' conversions out of line

status: done
agent: owner session
branch: issue/0252-size-headroom
depends-on: 0251
touches: crates/djs-bgen/src/emit_leaf.rs, crates/djs-core/src/builtins/gen/leaves.rs, crates/djs-core/src/interp/run/fastpath.rs, crates/djs-core/src/builtins/string.rs, tests/scores/size.tsv
guide: -

## Goal
0251 left the shipped module 289 B under the D020 budget. Find 1-3 KB gz without
slowing anything down, so the next performance work (decimal number formatting,
toFixed, small allocations, rope flattening) has room. Also record the result of the
separate-call-instruction experiment.

## Acceptance criteria
- [x] ≥1 KB gz saved on the shipped module, speed neutral (wmicro)
- [x] ceilings lowered; decisions recorded

## Log
- Prototyped `call_builtin` (an opcode for calls of builtin names): −55/−75 machine
  instructions per builtin call, but +1-2% on loops and plain calls in both variants.
  Not adopted (D3265); the patch is in work/scratch/0252-call-op of the primary checkout.
- Size by function against main before 0251: the generated shapes were 24 KB raw, most
  of it the inlined conversion sites (ToString/ToNumber of a possible object, twice per
  builtin) and IsRegExp in includes/startsWith/endsWith.
- ToString/ToNumber as one call with the primitive case inline: −805 B gz. IsRegExp of an
  object out of line: −146 B (the first try used `iid` at run time and pulled in the
  intrinsic name table, +5 KB: now a const). `to_case`'s two-byte ASCII path: −117 B.
  ToIntegerOrInfinity as a call: +18 B gz, dropped.
- wmicro of 19 string/number/Math cases against main: within ±2% (work/scratch/0252).
- Checked: djs-bgen tests, djs-core lib, difftest full corpus (ratchet) and builtins/
  stressed, test262 active, String case/startsWith/endsWith/includes dirs, djs-docs check.
- Guide: none affected (the builtins page describes sites, not how they are emitted).

## Follow-ups
- Intrinsics that no bytecode calls any more (BufferDetach, PromiseState, PromiseResult;
  the dispatch arms of StrTrim, StrLastIndexOf, StrRepeat, NumberToFixed, F64ToU32): ~100 B,
  each removal renumbers intrinsics and needs registry tombstones.
- The four speed fixes (decimal number formatting, toFixed in u128, small allocations at
  the arena end, rope flattening check) and a cheaper `fast_leaf_call`.

## Result
Shipped module 429,692 → 428,624 B gz (1,358 B under the D020 budget), slim 363,372 →
362,325; ceilings lowered (D3266). Generated builtins call their conversions out of line
(D3264). The separate builtin-call instruction is recorded as measured and dropped (D3265).
