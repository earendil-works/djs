# 0276: Call path and cache follow-ups from 0268 and the 0270 sweep

status: open
agent: pcall
branch: issue/0276-call-path-followups
depends-on: 0268
touches: crates/djs-core/src/interp/run (run_hot arms, ic, decline paths, fastpath, iter for collection iterators, spread calls), builtins/fast.rs (FAST flag tests only)
guide: interpreter.md
decisions: D3470-D3479

## Goal
0268's ranked follow-ups and pidle's sweep (re-measure each on current main first):
1. `typeof x === "string"` (1,082 instr) as a loop arm; `typeof o.name === 'string'` 2.1x QuickJS.
2. `Map.get`, `Set.has`, `a.push(i)` (1.2-1.5x): test the template's FAST flag before building a frame.
3. for-of over a Set or Map (3x per element: iter_next_loc declines for collection iterators into step) and `f(...args)` (1.35x).
4. A cache kind for absent properties (`o.missing ?? 1`, 2.1x) without slowing accessor reads (0268 measured a plain miss lookup at +14-25% on accessors: find another shape).
5. The add cache for a key shadowing a writable inherited data property (`e.name = 'X'`, ~10x per store).
6. Optional chaining `DEEP?.a?.b?.c` (1.35x), Math.floor + Math.round (1.5x) if cheap.
Superinstructions (a dispatch is 18 of a simple instruction's 21-29) only as a measured
experiment with the hotcode/slope procedure; report before adopting.

## Acceptance criteria
- [ ] per-op instructions vs QuickJS 2026 before/after; workload instructions with the noise control
- [ ] the hotcode.sh shared-path check for every run_hot change
- [ ] size: ~950 B room under the ceiling, shared (pidle's Set/Map work is waiting for ~865 B of it): prefer size-neutral; report bytes
- [ ] gate green; decisions recorded

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
