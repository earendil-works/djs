# 0137: Per-element cost of callbacks from natives (map/filter/forEach) without size growth

status: done
agent: perf
branch: issue/0137-callback-cost
depends-on: 0129
guide: docs/guide/builtins.md / interpreter.md if the call mechanism changes
touches: crates/djs-core/src/vm/ (native→bytecode call path, run_hot re-entry), crates/djs-bgen/ (native accessor glue), crates/djs-core/src/builtins/ (array iteration natives); heap (0136) is in heap/

## Goal
After 0129, pi-like is 0.94x quickjs-wasi but callback-heavy code is not: higher-order
2.6x, foreach 1.36x per element. About a third is the out-of-line native accessors
(D1634); inlining them costs +4.6 to +10.7 KB gz (owner trade, pending). Find wins that
don't grow the module: e.g. a dedicated callback loop (the native sets up one frame and
re-enters the bytecode function per element without the generic call machinery), fewer
accessor calls per element (hoisting checks out of the loop), cheaper argument passing
for 1–3 arg callbacks, or moving the hot iteration loops into bytecode intrinsics if that
measures better.

## Acceptance criteria
- [x] Profile of the per-element cost in wasm, split by component.
- [x] Wins landed within the size ceiling (or offset), no workload slower than +1%.
- [x] Updated numbers for the inlining trade on top of whatever lands, for the owner.

## Log
### Per-element profile (wasm, main 7c631680, node --cpu-prof, `a.forEach(x => { t += x })`)
Per element: djs 2176 vs quickjs-wasi 1013 instructions. Engine samples:

| component | share |
| --- | ---: |
| run_hot (the callee's ops, plus the loop's exit and re-entry) | 30% |
| `n_call`/`n_call_closure` (template, two `ensure`s, args, frame head) | 21% |
| out-of-line native accessors (n_slot, n_set_slot, n_arg, n_result, f64_of, num) | 27% |
| the native's state machine and prologue | 10% |
| step_once/native_step re-entry | 7% |
| n_dense_pre | 6% |

The other callback builtins, per element: map 2551, filter 2824, reduce 2083,
some/every 3725 (bytecode since D2254), find 2199. The bytecode loop calling `f` was
2042, against quickjs-wasi 2666.

### What landed (D2303)
- `%EachDense` with the `Each` continuation: one callback frame, rewritten in place per
  element by `each_return`. That runs from the hot loop (`Exit::Each`) and from
  `do_return`, so the two paths are one.
- Modes: forEach, map, filter, some, every, find, findIndex, reduce (forward).
  findLast and reduceRight keep their loops.
- No longer `@native`: forEach, map, filter, Reduce, reduce, FindViaPredicate, find,
  CollectPresent. CollectPresent copies a dense array with one `%ArraySliceDense`.
- Per-call fixes for the bytecode versions: LengthOfArrayLike `@fast`, hot
  `%ToObject`/`%IsCallable`, inline callability checks, reduce's accumulator in its
  rest array.
- After: forEach per element is 1156; run_hot (the callee) 38%, `each_return` and its
  helpers ~39%.
- Tests:
  - corpus builtins/array-callback-loops.js: fresh closures per element, arguments,
    defaults, assigned parameters, nesting, thisArg, sloppy/bound/proxy callbacks,
    mutation mid-loop, early exits, holes, reduce errors, throws at each position,
    generator and async callbacks, species incl. a leaked filter target;
  - `native_frames_are_used` now uses sort;
  - bgen's behavior stand-in declines EachDense.
- Guide: builtins.md (the Rust backend list and excerpt, now push's loop; the
  EachDense paragraph) and interpreter.md (the `Each` continuation, native frames).

### Numbers (wasm, against main 5d66ef74)
- Size: 457,604 → 457,408 gzip (−196). The ceiling is lowered to 458,444.
- Workloads (n=3; the >1% ones rechecked at n=5, all within ±0.5%):
  - pi-like 6791 → 6271 (−7.7%, 0.87x quickjs-wasi);
  - foreach −27.5% (0.99x);
  - higher-order −30.9% (1.78x; the rest is sort's comparator at ~5x);
  - arrays −1.2%;
  - the others ±0.5%.
- Per element (main → now, quickjs-wasi in brackets):
  - forEach 2176 → 1156 (1007);
  - map 2487 → 1265 (1052);
  - filter 2764 → 1379 (1019);
  - reduce 2070 → 1392 (869);
  - some/every ~3720 → ~1000 (785);
  - find/findIndex ~2205 → 1000 (796).
- Per call on 0- and 1-element arrays it costs more than the natives did:
  - forEach +2.2k / +1.9k;
  - map +1.6k / +1.2k;
  - filter +2.9k / +3.5k;
  - reduce +0.8k / +3.8k.

  The bytecode builtins' intrinsic calls go through the general path. Before the
  per-call fixes this was up to +9k.

### Inlining trade, refreshed on top of this
- n_slot and n_arg inline in wasm: +2.9 KB gzip, for higher-order −4.8%, json-replacer
  −2.7%, pi-like −0.4%, foreach 0.
- The whole accessor set: +6.5 KB, for −8.7%, −3.4%, pi-like +0.4%.

With the callback loops gone from the natives, only sort and the JSON slow path gain.
Recommendation: drop the trade.

### Tried
- Keeping forEach/map/filter `@native` around EachDense: per call as before, but +1.7
  to +2.6 KB gzip.
- A leaner `each_return` variant: it was larger after gzip and slower for forEach, so it
  was dropped.

## Follow-ups
- Per-call cost of bytecode builtins (+1–4k on tiny arrays, above): leaf intrinsic
  calls from bytecode go through step_once → step → call_intrinsic, ~500 instructions
  each. A run_hot path for more leaf intrinsics, or for `%EachDense`'s entry, would
  close most of it. filter also runs `plain_species` twice (ArraySpeciesCreate, then
  EachDense).
- sort's comparator: 24.6k vs 4.97k instructions per element in quickjs-wasi. This is
  now the main callback gap (higher-order 1.78x). SortList is native; a merge sort
  driven like EachDense (one comparator frame reused) would be the analogue.
- findLast/findLastIndex/reduceRight could get a backward EachDense mode.
- TypedArray callback builtins (TAReduce etc.) still use their own loops.
- The native-API inlining trade: recommend dropping it (numbers above).

## Result
The callback builtins run their dense loops in `%EachDense`, with one callback frame
reused per element (D2303).
- The module is 196 B smaller.
- In wasm: pi-like −7.7% (0.87x quickjs-wasi), foreach −27.5% (now at parity),
  higher-order −30.9% (1.78x). Per element, forEach and map are within 15–20% of
  quickjs-wasi (were 2.2–2.4x).
- Cost: calls on empty or one-element arrays are 1–4k instructions dearer than the
  natives were (Follow-ups).
- The native-API inlining trade, re-measured: +2.9 KB for higher-order −4.8% and
  pi-like −0.4%. Recommend dropping it.
