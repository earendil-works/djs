# 0154: Slow spots found by pico's benchmarks

status: done
agent: perf
branch: issue/0154-slow-spots
depends-on: 0137, 0152
guide: -
touches: crates/djs-core/src/interp/run/iter.rs, crates/djs-core/src/builtins/ (Array.from, callback loops), crates/djs-core/src/vm/ (leaf intrinsic call path); bloat (0151 L4) changes bytecode operand encoding in compiler/ and interp/ run_hot: coordinate via the coordinator

## Goal
docs/lessons-from-pico.md lesson 1:
1. `[...Array(n).keys()]` / `Array.from(Array(n).keys())` are 26x slower than quickjs-wasi
   (562 vs 21.6 ms for 1M): the D1800 fast iteration path gives up on holes even for keys
   iterators, which never read the element.
2. `Array.from({length: n}, f)` 2.9x slower.
3. `map`/`reduce` on 0–2 element arrays 2x slower (0137 follow-up 1: leaf intrinsic calls
   from bytecode go step_once → step → call_intrinsic; filter checks species twice).

## Acceptance criteria
- [x] Each fixed or explained, measured in wasm vs quickjs-wasi (pico's cases rewritten in
      work/scratch/0152), corpus cases added for the fixed paths.
- [x] No workload slower than +1%; size ratchet respected.

## Log
- work/scratch/0152/w did not exist; pico's cases rewritten in work/scratch/0154/w
  (keys-spread, keys-from, from-length, push-loop, tree-children, tree-filter, tiny-*),
  wins.py for instruction counts, cpu.sh for profiles.
- 1 (D2305): `iter_plan` produces keys over holes; new leaf `%IteratorFastDrain` stores
  keys/values runs in one call (spread, IterableToList, IteratorRest, Array.from iterable
  path without mapfn). 593 → 44 ms (keys spread), 642 → 45 ms (Array.from keys);
  quickjs-wasi 55 / 54. Registry: tests/builtins/registry.tsv +1 intrinsic.
- 2 (D2306): `%EachDense` mode 8 for Array.from(arrayLike, f): 168 → 101 ms (qjs 80). A first
  version with the element read inlined into `each_element` cost foreach +1.0% and
  higher-order +1.4%; out of line it is ±0.2%.
- 3 (D2307): exits from run_hot through step_once/step/call_intrinsic cost ~2.3k wasm
  instructions; `@fast` closure calls' exits are cheap. Empty arrays skip `%EachDense`;
  `%EachDense` runs from run_hot's Decline exit directly (`each_dense_decline`; coordinated,
  new code in the Decline handler only). Per call: forEach [] 5.5k → 3.2k, map [] 9.1k →
  6.8k, map [x] 11.8k → 11.0k, filter [] 10.6k → 7.1k, reduce [x] 14.0k → 13.0k (qjs 1.6k,
  5.0k, 6.6k, 5.0k, 2.5k). Tree (children + map/reduce) 700 → 665 ms vs qjs 310: about 20% is
  the collector tracing the live tree, the rest per-call bytecode in the builtins.
  Tried and dropped (both slower on every workload or useless): `@fast` builtin
  intrinsics in `intrinsic_hit` (never reached: privileged code calls them as closures),
  `@fast` calls inside `call_closure`.
- Filter's "double species check" (ArraySpeciesCreate's fast entry, then `plain_species` in
  `each_ok`): the second is two own-property lookups (not measured separately, an estimate of well under 1% of a filter call); left as is.
- All 22 workloads vs the branch start (wasm instructions, n=3): −2.5% to +0.41%
  (json-indent +0.41%). Size 454,744 B gzip (+905), ceiling 454,863.
- Corpus: builtins/iterator-drain.js, builtins/array-from-arraylike.js,
  builtins/array-callback-tiny.js (qjs expected). test262 Array/from, ArrayIteratorPrototype,
  spread/destructuring dirs, the callback builtins: no new failures.
- Guide: builtins.md (the `%EachDense` paragraph); generated parts regenerated.

## Follow-ups
- reduce: the `...rest` parameter (an array per call) and the second frame (`Reduce`) cost
  ~3k instructions per call; an "argument count" intrinsic would let reduce take
  `initialValue?` directly.
- Callback returns on short arrays still leave run_hot (`Exit::Each` → `each_return`) once
  per element, plus `each_finish`; handling the last element's return inside the loop
  is the next lever (hot.rs, after 0151 L4).
- Tree-shaped data: the collector's tracing of the live tree is ~20% (QuickJS counts
  references); generational old-region promotion of long-lived trees would cut it.

## Result
Keys iterators over holes and `%IteratorFastDrain` (D2305): `[...Array(n).keys()]` 13x
faster, now 0.8x quickjs-wasi. Array.from(arrayLike, f) through `%EachDense` (D2306): 1.7x
faster, 1.26x qjs. Short-array callback builtins (D2307): 8–33% fewer instructions per
call; pico's tree case 1.05x faster, still 2.1x qjs (collector + per-call bytecode; see
Follow-ups). No workload above +0.41%; size within the ratchet.
