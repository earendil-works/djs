# 0270: The construct path, and a sweep of everyday codemode operations vs QuickJS

status: done
agent: pidle
branch: issue/0270-construct-op-sweep
depends-on: 0266
touches: crates/djs-core/src/interp/run/construct.rs (decline_construct, super calls), compiler (only a super_call op if adopted), builtins for the operations the sweep finds (coordinate: string ops are pgc's 0269, regexp/JSON pscan's 0267, call path/run_hot pcall's 0268), js/bench/engines/ops.ts (new rows), tests/corpus
guide: interpreter.md, benchmarks.md (new ops rows are generated: regen, not by hand)
decisions: D3410-D3419

## Goal
1. The construct path: `new C()` 2.9k vs 1.9k QuickJS instructions, derived classes
   1.2-1.6x (0236's follow-up: ~10 cheap declines per `new D()`; a `super_call` that reads
   the active function and new.target from its frame would remove three); `new Promise`
   1.3-1.4x rests on it.
2. A sweep: the ops table was written for 0229 and the workloads are nine synthetic
   bodies. Take the operations real codemode bodies use most (tests/corpus/codemode and
   codemode-harvest: count builtin calls and syntax kinds over the corpus), measure the top
   ~40 per op in wasm against QuickJS 2026 (wmicro or an ops.ts extension), and list every
   one above 1.2x with its cause. Fix the cheap ones in your areas; route the others to the
   coordinator with numbers. Candidates nobody measured yet: Date (now, toISOString,
   new Date(string)), Object.keys/values/entries over plain objects, for-in, destructuring
   with defaults, optional chaining, spread of arrays into calls, Array.prototype.flatMap,
   find/findIndex/some/every, string comparison and localeCompare, Number parsing,
   JSON.stringify with indentation, console formatting of objects.

## Acceptance criteria
- [ ] the sweep's table (op, djs, QuickJS, ratio, cause) in the issue; ops.ts rows added for the important ones
- [ ] construct-path numbers before/after; the fixes landed with tests
- [ ] size reported (~2 KB room, shared); gate green; decisions recorded

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

2026-11-21 (pidle)
- Tools: opsins.py (wasm instructions per op = (2R − R runs)/ops, `/usr/bin/time -l node
  --no-liftoff`), with sweep op lists in work/scratch/0270/sweep/ (ops.py: 70 ops,
  iter.py, date.py); count.py counts builtin calls and syntax over the 240 harvested
  bodies of tests/corpus/codemode-harvest (top: tools.read/bash, arrow functions, map,
  Promise.allSettled/all, join, template literals, ===, for-of, ternaries, regex
  literals, forEach, spread, slice, ??, split, push, typeof, replace, trim, String(),
  filter, JSON.stringify/parse, ?., startsWith, throw new Error, includes, endsWith,
  Object.entries, indexOf, sort, Date.now, Array.from, Object.keys, localeCompare, Map/Set).
- Construct path (D3410): no check_ctor (calls refused at the call), class/builtin
  constructors return their object (PushResult frames), super() from the frame
  (super_call_frame/forward), this checks elided after a first unconditional super(),
  class fields via define_field. new C() 2.9k → 2.3k (QuickJS 2026 1.8k), class with a
  field 7.1k → 4.7k (4.8k), derived default 8.6k → 4.5k (3.3k), super(a) 8.2k → 5.0k
  (4.4k), three levels 11.3k → 6.4k (5.8k), new Promise 7.6k → 7.2k (6.0k); +424 B gz.
  PARKED by the coordinator (no room; classes appear in none of 2,535 codemode scripts):
  work/research/0270/construct-path.patch, branch issue/0270-construct-parked.
- Map/Set (D3411): Map/Set Iterators drain into arrays; new Set(array)/new Map(pairs)
  in the `new` entries. Date.parse/new Date(ISO) (D3412) and a wire transcode offset.
- Size: main 428,728 → 429,545 gz (+817; ceiling 429,648). Tried and dropped: a direct
  `[...set]` in array_spread plus GetIterator for Set/Map (+520 gz: [...set] 24k → 6.9k,
  for-of Set 20k → 14k); peek/commit Map iterator cursors (+356 gz); Set/Map drain from
  the constructors' JS loops (−340 gz but 2.5x slower than the entries).
- Tests: lib, e2e plain, bgen, active test262, difftest plain/stress/plain-dumps --lazy
  over builtins, language, codemode; new corpus case builtins/collection-drain.js.
  Guide: builtins.md (the Set/Map `new` entries), interpreter.md (the drain).

### The sweep (wasm instructions per op, incl. a ~350 (djs) / ~450 (QuickJS) loop; main builds of 2026-11-21)

| op | djs | QuickJS 2026 | ratio | cause / owner |
| --- | ---: | ---: | ---: | --- |
| flatMap per element | 11.7k | 2.1k | 5.7 | FlattenIntoArray per element: HasProperty/Get/Call intrinsics and a recursive frame per result array; an %EachDense mode with a flatten step would fix it (follow-up) |
| x.replace("TODO","DONE") | 14.9k | 3.6k | 4.2 | strings, pgc 0269 |
| Date.parse(ISO) | 8.3k → 4.4k | 2.0k | 2.2 | fixed partly (D3412); the rest: builtin frame + two intrinsics through step |
| Math.max(...10) | 32.6k | 8.8k | 3.7 | spread into a builtin call: spread_args + generic call path, pcall 0268 |
| for-in over 5 keys | 21.4k | 6.5k | 3.3 | for_in_next through step per key, re-interning each key string and has_property; keys could be kept as atoms (format change; follow-up) |
| LINE.match(/^([^:]+):(\d+):/) | 37.9k | 11.6k | 3.3 | regexp exec, pscan 0267 |
| localeCompare | 7.0k | 2.5k | 2.8 | strings, pgc 0269 (sort of 10 with it 199k vs 75k) |
| replace(/\s+/g,' ') | 40.4k | 16.3k | 2.5 | regexp, pscan 0267 |
| o.missing ?? 1 | 1.65k | 0.77k | 2.1 | absent-property read misses the IC, pcall 0268 |
| typeof o.name === 'string' | 1.88k | 0.91k | 2.1 | typeof + compare with a string, pcall 0268 |
| startsWith+endsWith | 4.7k | 2.3k | 2.1 | strings, pgc 0269 |
| Array.from({length:10}, fn) | 29.7k | 15.5k | 1.9 | EachDense FROM mode setup + array-like path; follow-up |
| for-of over a Set | 1.9k/elem | 0.7k | 2.7 | iter_next_loc declines collection iterators to step, pcall 0268 (a GetIterator fast path for Set/Map: −30%, +gz) |
| sort 10 numbers ([...N10].sort) | 55k | 30k | 1.8 | comparator calls via SortCompareCall; spread copy |
| destructure {a, b, c = 1} | 2.9k | 1.7k | 1.8 | per-property get_field + default check; compiler (pstart) |
| destructure [a, b] | 6.1k | 3.4k | 1.8 | array destructuring builds an iterator (GetIterator fast) |
| template `${a}-${i}` | 4.4k | 2.7k | 1.6 | strings, pgc 0269 |
| < on strings / === non-interned | 1.4k / 1.3k | 0.9k / 0.9k | 1.5 / 1.4 | strings, pgc 0269 |
| Math.floor + Math.round | 2.5k | 1.6k | 1.5 | builtin calls, pcall 0268 |
| for-of Object.entries(o5) | 46k | 31k | 1.5 | entries arrays + pair destructuring |
| push | 1.26k | 0.86k | 1.5 | method call path, pcall 0268 |
| Object.values / keys of 5 | 10.3k / 9.2k | 7.0k / 6.8k | 1.5 / 1.4 | builtin frame + key list |
| find (per element) | 1.17k | 0.82k | 1.4 | EachDense per-element cost |
| includes / indexOf in 10 strings | 3.6k / 3.1k | 2.6k / 2.4k | 1.4 / 1.3 | string compare per element |
| slice string / indexOf(':') | 2.1k / 1.9k | 1.4k / 1.4k | 1.4 / 1.3 | strings, pgc 0269 |
| JSON.parse small object | 21.8k | 16.0k | 1.4 | pscan 0267 |
| f(...10) | 12.8k | 9.4k | 1.35 | call spread, pcall 0268 |
| reduce per element | 0.98k | 0.73k | 1.3 | EachDense REDUCE |
| ?.?.? chain | 1.56k | 1.16k | 1.3 | pcall 0268 |
| toFixed / Set.has / String(i) | 2.8k / 1.7k / 1.8k | 2.2k / 1.3k / 1.4k | 1.3 | builtin call path |
| Map.get | 1.73k | 1.42k | 1.2 | call path |
| new Error + throw/catch | 16.7k | 25.5k | 0.65 | fine |
| JSON.stringify (indent) | 19.6k / 24.6k | 23.3k / 27.4k | 0.84 / 0.9 | fine |
| object spread, join, slice array, toLowerCase, split, Date.now, toISOString, parseFloat, Number(s), Promise.all/allSettled, async call | | | ≤ 1.15 | fine |
| console.log(object) | 67k | (no console in the QuickJS runners) | – | not compared |

Map/Set (fixed here, D3411): [...set] 122k → 24k (9.6k), Array.from(set) 137k → 30k
(11.2k), new Set(arr10) 71k → 19k (16.4k), [...new Set(arr)] 192k → 39k (25.4k),
new Map(pairs10) 126k → 21k (18.0k).

## Follow-ups
- flatMap (5.7x): an %EachDense FLAT_MAP mode (per result: append a dense Array's
  elements, else the value) needs a target index slot; or a dense append leaf.
- for-in (3.3x): keep the snapshot's keys as atoms (no re-interning per step) and
  skip has_property while the target's shape is unchanged; for_in_next out of step.
- Array.from({length}, fn) (1.9x), destructuring (1.8x, compiler).
- The parked construct path (D3410), when there is room.
- Property IC: a store that shadows a writable inherited data property (`e.name = "X"` on
  an Error) misses the add cache (+2.5k per store).
- Size: json::parse's const-generic `next`/`key_colon` (3 copies, ~9 KB raw) and
  `vec_with_capacity` (11 copies) are the largest duplicate monomorphizations, but
  duplicates compress well (wire transcode: −763 raw, −58 gz).

## Result
Landed: Map/Set iteration drains (D3411), new Set(array)/new Map(pairs) in Rust (D3411),
Date.parse/new Date(ISO) 2x faster and a wire transcode offset (D3412), the sweep table
with owners above. Parked: the construct path (D3410, patch in work/research/0270).
Size +817 B gz (428,728 → 429,545, ceiling 429,648).
