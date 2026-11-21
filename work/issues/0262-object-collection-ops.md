# 0262: Object and collection operations: Map/Set string keys, spread, entries, comparator sort

status: done
agent: pidle
branch: issue/0262-object-collection-ops
depends-on: -
touches: crates/djs-core/src/heap/collection.rs and Map/Set fast entries, object spread / CopyDataProperties paths, Object.entries/fromEntries/keys/values builtins, comparator sort (%Sort* leaves), padStart/padEnd, builtins/*.ts for these, tests/corpus
guide: builtins.md if a fast-path story changes
decisions: D3330-D3339

## Goal
The per-op gaps to the best QuickJS (docs/guide/benchmarks.md, ops table) that are not
the cost of finding the builtin (that is 0259's): Object.entries + fromEntries 2.38x,
padStart 2.25x, sort with a comparator 1.84x, Map.set with string keys 1.81x, Set.add
1.52x, object spread 1.45x. Profile each in wasm (wmicro, js/bench/engines/ops.ts), find
where the instructions go beyond the call, and fix what is cheap: hashing and probing of
string keys (hash caching on strings? atoms vs content hashes), allocation per entry,
generic property-definition paths taken where a plain data property is known, the
comparator's re-entry per comparison (0231 follow-up: n_call's frame write + re-entering
the native frame).

## Acceptance criteria
- [ ] per-op instructions and ops.ts ratios before/after, and the workloads (json, inspect, text use these)
- [ ] exact semantics (corpus + test262 for the touched builtins), suspension inside callbacks still works
- [ ] size: the module is ~600 B under the D020 budget with two other issues waiting on the owner for room; prefer size-neutral or shrinking changes, report bytes
- [ ] gate green; decisions recorded

## Notes
- 0259 (pcall) owns the builtin call path (finding the callee, fast_leaf_call's
  dispatch, decline_fast). Agree with pcall through the coordinator before touching it.
- 0258 (pgc) owns heap/compactor.rs and gc.rs (collections are rehashed by the collector:
  tell pgc if you change the table layout).
- 0260 (pscan) owns byte kernels (string search, JSON scanning).

## Shared context (all of 0256-0260)
The owner asked to push both memory and speed as far as they go (2026-11-21). Background,
fresh profiles and the ranked idea list: work/scratch/perfreview/ of the primary checkout
(BRIEF.md, IDEAS.md; prof.ts and cat.py profile the codemode workloads in wasm). Read
IDEAS.md's section for your issue first; its numbers are estimates, not measurements.

Five issues run in parallel (0256 per-VM/per-byte costs, 0257 idle memory, 0258 GC,
0259 builtin calls, 0260 byte kernels). Stay in your `touches:`; announced overlaps are in
each issue's Notes.

Measuring (D016: wasm32 under node is the yardstick):
- run time and memory per workload: build the shipped module in your worktree
  (`scripts/build-wasm.sh`, or `--keep-names` for profiles) and run
  `DJS_WASM=$PWD/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm node js/bench/engines/run.ts --engines djs --reps 25`
  before and after, on main's build and yours, at load below ~4 (`uptime`; say the load).
  Memory claims: scripts/perf/mem-perturb.sh (D2048).
- instructions: scripts/perf/wmeasure.py / wmicro.py (TESTING.md "Instruction ratios").
- Report per workload: ms and MiB, main vs branch, and the module's gzip bytes.

Size: the shipped module is ~1.3 KB gzip under the D020 budget (ceilings, D3266). Report
the bytes of every change; find offsets where you can. If a measured win needs more room
than that, stop and tell the coordinator with the numbers: do not raise ceilings yourself.

Never weaken determinism (collection points a function of the program, D1710), suspension
anywhere, images, or the unsafe rules (AGENTS.md).

## Log
- Measuring: work/scratch/0262/opsins.py counts wasm instructions per op ((setup + loop at
  2R reps) − (at R reps), node --no-liftoff, /usr/bin/time -l), for djs builds against
  quickjs-wasi and the QuickJS 2026 / QuickJS-NG builds of js/bench/engines (qb-run.ts).
  Without --no-liftoff and with short loops the numbers carried V8's tier-up (10x off).
  Workloads: wl.py (fresh VMs, (41 runs − 1 run)/40, ±2% noise). Profiles: prof.sh (node
  --cpu-prof on a keep-names build).
- Where the instructions went (main): padStart/padEnd interned the literal " " on every
  call (the generated leaf's `l_str`) and built the result in a Vec; Map/Set hashed a new
  key three times and read each slot through `object_slot`'s count check; comparator
  sort re-entered the native SortList frame per comparison (step_once → native_step →
  state machine → n_call_closure's frame write); Object.fromEntries ran the TS loop
  (~13k instructions per entry: iterator steps, Gets with string keys, ToPropertyKey,
  CreateDataPropertyOrThrow, try); object spread did three property lookups per key plus
  a shape transition per key.
- Landed (bytes are gzip of the shipped module, measured one by one):
  - padStart/padEnd: StrPad takes undefined for spaces; a one-unit one-byte fill is written
    into the result cell (`alloc_latin1_pieces` + fill). `zero_to` (heap/arena.rs, outside
    touches, one attribute) went out of line: dropping pad's Vec::resize let LLVM inline
    resize at the arena's three sites (+0.7 KB raw). −95 B.
  - Map/Set: one hash per set, the same key value matches before the content comparison,
    direct slot reads (table layout unchanged, so nothing for pgc). −12 B.
  - Comparator sort: `%SortCompareCall` + EachDense's sort mode (D3330). +367 B.
  - Object.fromEntries: `%ObjectFromEntriesDense` (D3331, room approved). +374 B.
  - Object spread: `Heap::spread_clone` shares the source's shape (D3332). +174 B.
  Total 425,457 → 426,260 B (+803, main 6a83498c); slim 858,849 → 860,907 raw (under
  its ceiling).
- Tried and dropped: pad writing through `contiguous_data` (+135 B, same speed); four
  shapes of the fromEntries leaf (Vecs of keys/values +480, dense_range per entry
  +362 but 22.6k instructions, kept: two passes over an out-of-line `entry_pair`);
  sort through `EachDense(9, ...)` called from the native (+601 B raw in the generated
  state machine: a 7-argument resume point), so the 2-argument `%SortCompareCall`.
- Instructions per op, main → branch [QuickJS 2026, QuickJS-NG] (r5-ops.txt): padStart
  4868 → 3728 [2412, 3058]; padEnd 3788 → 2368 [2175, 3231]; sort comparator per element
  14560 → 8867 [5708, 6282]; Map.set string keys (fresh maps) 2707 → 2489 [1771, 1894],
  with the concat 4693 → 4496 [2430]; Set.add 2863 → 2775 [1890]; Map.set update 2088 →
  1931 [1528]; Map.get 2293 → 2239 [1497]; object spread 16009 → 3961 [12255, 13589];
  Object.fromEntries (8 keys) 105180 → 15056 [14819, 16864]; entries+fromEntries+keys
  132733 → 43072 [45152].
- ops.ts (wall, load 11.6, 15 reps, indicative), djs/best main → branch: padStart 2.44 →
  1.35, sort comparator 2.07 → 1.49, Map string keys 1.82 → 1.70, Set 1.63 → 1.33, Map
  group rows 1.67 → 1.12, object spread 1.68 → 0.18, entries/fromEntries (0.26 → 0.22 ms;
  0.53 of QuickJS).
- Workloads (wl.py instructions per run, main → branch): json 42.5 → 41.8M (−1.8%),
  text 33.5 → 32.9M (−1.7%), inspect 6.31 → 6.33M, sequential 5.89 → 5.97M (both noise).
- Tests: corpus builtins/sort-comparator-frames.js (mixed result types, other callables,
  nested sorts, throws, mutation), builtins/from-entries.js (every decline: Maps,
  generators, getters, Proxies, holes, modified iterators, non-string keys),
  language/object-spread-clone.js (symbols among strings, frozen/sealed/read-only/
  non-enumerable/getter/index/dictionary/null-proto sources, later parts), pad cases in
  builtins/fast-entries-string.js (slices, ropes, append ropes, one-unit fillers); all
  with --suspend-every 97 --gc-stress. test262 Array sort/toSorted/forEach/map/reduce/
  filter/from, TypedArray sort, Object fromEntries/entries/assign, language/expressions/
  object, String padStart/padEnd, Map/Set/WeakMap/WeakSet: no new failures.
- Guide: builtins.md (native sort and EachDense), interpreter.md (the comparator's
  frame), heap.md (spread shares shapes). benchmarks.md's prose still holds; its table
  is regenerated from records.

## Follow-ups
- Map/Set: what is left of the string-key gap (1.4-1.5x) is mostly the call (run_hot's
  decline, try_fast_call, the method cache) and the `r.s + r.u` concat (1.5x): 0259's.
- Map/Set with number or object keys pay the keyed SipHash round per key (D2433): integer
  keys 1.9x QuickJS. A cheaper keyed mix for numbers needs a decision on flooding.
- Object.fromEntries of a Map, a generator or non-string keys still runs the TS loop
  (~13k instructions per entry); a per-entry leaf would serve them at a small cost.
- Object spread that is not the literal's first part, or several spreads, takes the
  per-key path (three lookups and a transition per key).

## Result
Landed D3330-D3332 and two smaller changes. padStart/padEnd no longer intern " " per call
and write one-unit fills in place; Map/Set hash a key once per set; a comparator sort runs
its comparisons in one reused frame (`%SortCompareCall`, EachDense's sort mode); an Array
of entry arrays with string keys goes through Object.fromEntries in one leaf; `{...src, ..}`
of a plain source shares its shape. Instructions per op vs QuickJS 2026 main → branch:
padStart 2.02x → 1.55x, padEnd 1.74x → 1.09x, sort comparator 2.55x → 1.55x, Map.set
string keys 1.53x → 1.41x, Set.add 1.51x → 1.47x, object spread 1.31x → 0.32x,
Object.fromEntries 7.10x → 1.02x. +803 B gzip (fromEntries' room approved). Known gaps:
Follow-ups.
