# 0272: Per-element cost of callback array builtins

status: in-progress
agent: pscan
branch: issue/0272-callback-builtins
depends-on: 0268 (batch 1)
touches: crates/djs-core/src/interp/run/each.rs and dense.rs (EachDense and its callers), the @native state machines of map/filter/forEach/reduce/find/some/every/flatMap/findIndex (builtins/*.ts and their generated Rust), crates/djs-core/src/interp/run/iter.rs (only for for-of over arrays if the profile points there), tests/corpus
guide: builtins.md, interpreter.md if the each story changes
decisions: D3430-D3439

## Goal
heavy's profile has ~22% in "other interpreter/builtins", with each_return, each_element,
each_args, decline_fast among the top functions: its bodies run `batch.map`, `forEach`,
`filter`, `reduce` with small closures over every line of 96 files. The native track's
higher-order and foreach cases (1.73x and 0.93x of quickjs-wasi at 0223; re-measure after
0268's batch 1) and scripts/perf/perelem.py measure instructions per element. Find what a
callback round trip costs per element now (frame setup and teardown, argument copies,
result handling, the decline back into the native frame) and cut it: frame reuse across
elements (0262's D3330 did this for the comparator sort), cheaper argument passing for
1-3 argument callbacks, filter/map result building, reduce's accumulator path.

## Acceptance criteria
- [ ] perelem.py numbers per builtin (djs vs quickjs-wasi) before/after; heavy/text/json workload instructions
- [ ] exact semantics: callbacks that throw, suspend (await is not possible but host calls and fuel slices are), mutate the array, are not plain closures; corpus with --suspend-every 97 --gc-stress --plain-dumps
- [ ] size reported (~1.3 KB room, shared); gate green; decisions recorded

## Notes
- pcall (0268) owns run_hot, decline_fast and the call path; each.rs's interaction with
  them goes through the coordinator. pidle (0270) changes construct paths and adds ops.

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
- Measured (main 8197d88e, slope.py N/2N on 100-element arrays, machine instructions per
  element, work/scratch/0272/s/*.js; quickjs-wasi by the same method, qslope.py): djs
  forEach 799, map 868, filter 979, reduce 896, some 721, find 733, every 721, for-of 736,
  map (3 args) 951, flatMap 9,420; quickjs-wasi 920, 996, 951, 769, 712, 696, 709, 1,394,
  1,226, 2,570. perelem.py's single runs include V8's tier-up (900-1,400 per element on
  either engine): not usable here. flatMap is generic bytecode (FlattenIntoArray) and
  appears in one codemode corpus file (275 .map, 39 .filter, 37 .forEach, 7 .reduce).
- Profiles (forEach, wasm): run_hot 48% (callback body ~125 instructions, the rest the
  Each exit, load_frame and re-entry), each_return 27%, each_element 10%; map adds the
  store (create_element_fast + dense_put ~16%).
- D3430 (batch 1): forEach -6.2%, map -6.0%, filter -9.4%, reduce -21.5%, some/find/every
  -12.6%; heavy -2.3%. A frame rewrite through the stack's raw bytes was +3-4% (dropped).

## Follow-ups

## Result
