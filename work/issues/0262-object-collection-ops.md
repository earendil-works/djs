# 0262: Object and collection operations: Map/Set string keys, spread, entries, comparator sort

status: in-progress
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

## Follow-ups

## Result
