# 0254: Number formatting, toFixed, small allocations, flattened ropes

status: done
agent: owner session
branch: issue/0254-builtin-speed
depends-on: 0252
touches: crates/djs-core/src/num/format.rs, crates/djs-core/src/interp/conv.rs, crates/djs-core/src/heap/arena.rs, crates/djs-core/src/heap/string.rs, tests/corpus/builtins/number-format-fast.*
guide: -

## Goal
After 0251, the slowest common operations against QuickJS were `String(i)` and `toFixed`
(1.8-1.9x), whose time is number formatting and the allocation of a short string, and
string builtins on ropes that re-walked the rope on every call. Fix those without
touching the interpreter loop (D3265).

## Acceptance criteria
- [x] faster on the targeted operations, workloads neutral, size within the ceiling
- [x] tests for the new paths

## Log
- Profile (0251 work): `String(i)` ~15% digits (64-bit division per digit), ~20-25%
  allocation; `toFixed` ~37% big-integer arithmetic; charCodeAt on a `repeat` result
  ~9% in rope checks and `flatten` on every call.
- Four changes (D3267). Wasm instructions: String(i) 1963 → 1500, toFixed 3500 → 2679,
  (i).toString() 2185 → 1912, 'a' + i 1969 → 1773, template 2852 → 2661; flat-string
  builtins unchanged once the rope case moved out of line (inlined, it cost charCodeAt
  +1.7%). ops.ts (interleaved, load 16-18): String(i) 6.3 → 4.6 ms, toFixed 6.0 → 4.1,
  padStart 8.8 → 7.2, template 5.7 → 5.35. wmeasure: within ±0.8%, strbuild −3.9%.
- `fast_leaf_call` looked at and left alone: three loads of callee checks, a table lookup
  and the indirect call; the rest is leaving the loop (D3265).
- Tests: `to_fixed_small_path_matches_reference`, `u32_decimal_digits`, corpus case
  builtins/number-format-fast (from qjs; stressed), djs-core lib, difftest ratchet,
  test262 active.
- Guide: none affected (the benchmarks page's numbers are from before this change; its
  prose names the causes, which still hold, less strongly).

## Follow-ups
- Re-record the benchmarks page on a quiet machine after this (String(i) and toFixed
  should drop to about 1.2-1.3x of QuickJS).

## Result
D3267: ints format without the float formatter, toFixed in u128 for ordinary values,
small cells zero ahead, flattened ropes read directly. +162 B gz.
