# 0261: Instantiation and sandbox setup costs

status: in-progress
agent: pstart
branch: issue/0261-instantiation-cost
depends-on: 0256
touches: crates/djs-wasm (exports, the ABI, data segment layout), js/src (sandbox.ts, base.ts, the ABI wrappers, layout of host setup), js/pi/vm.ts and js/pi/engine.ts (entropy pool, setup calls), js/test
guide: host.md, js-api.md if the ABI or setup changes
decisions: D3320-D3329

## Goal
0256 found that ~100 µs of a ~140 µs new sandbox is V8 instantiating the module, and
named the parts (its Result and Follow-ups):
1. Exports: ~0.3-0.8 µs each per instantiation, 65 of them; a stripped-exports module cut
   instantiation by about a quarter. djs is unreleased (D024): change the ABI in place.
   Fewer exports (e.g. rarely used ones behind one dispatch export, or folded calls),
   with js/src, js/pi and the tests updated.
2. The 97 KB data segment: about another quarter of instantiation. Find what in it a
   typical run never reads (cold Unicode / case / regexp property tables, panic and format
   strings) and keep it out of the segment that every instantiation copies, e.g. coded in
   a custom section and materialized on first use, the way the base image is. Measure
   instantiation, idle memory and the first use's cost. Coordinate with 0257 (pidle),
   which changed the stack/data layout and dlmalloc's first segment.
3. Sandbox setup (~30 µs): batch the console host-function setup (~9 small wasm calls)
   into one; the arena's growth by single pages costs ~2 µs per memory.grow call (4-10
   per run): grow in larger steps where it does not add touched pages.
4. js/pi/vm.ts: use the entropy pool for its per-VM seeds (as 0256 did for Sandbox).
5. Rope interning compares through per-unit iterators (~0.8% of sequential): flatten or
   compare by slices.
Target: a new sandbox well under 100 µs at a quiet load, idle memory not worse.

## Acceptance criteria
- [ ] new-sandbox time and small-workload run times, main vs branch (interleaved, as in 0256)
- [ ] the ABI change carried through js/src, js/pi, djs-embed if affected, and tests; layout/ABI version bumped as the repo requires
- [ ] size reported (the module is near the D020 budget: prefer changes that shrink it)
- [ ] gate green; decisions recorded; guide pages updated

## Notes
- 0257 (pidle) merges an elastic stack (a new `djs_stack` export, relocation in
  sandbox.ts/trap.ts) and the data-page layout: rebase onto it before touching those files.
- 0259 (pcall) and 0260 (pscan) work in djs-core; 0258 (pgc) in the heap.

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
