# 0264: Cold Unicode tables out of the data segment: stack + data in two pages

status: in-progress
agent: pscan
branch: issue/0264-cold-unicode-tables
depends-on: 0261, 0263 (batch 1)
touches: crates/djs-core/src/unicode (or wherever the \p property and normalization tables live), crates/djs-core/src/regexp (compile of \p classes: a stop before any effect), crates/djs-core/src/interp (the instruction re-run on the stop), crates/djs-wasm (the custom section, the op to hand it over, the status), js/src (sandbox.ts/trap.ts: hand-over on the status, like elasticStack), js/pi/engine.ts, tests
guide: limits.md, host.md, arena.md
decisions: D3350-D3359

## Goal
Every workload row pays one 64 KiB page because stack (64 KiB) + static data (~90.9 KB
after 0263's batch 1) do not fit in two pages (≤131,072 B: now ~25.4 KB over). The regexp
`\p{…}` property tables (~24.5 KB: props DATA/GC_RUNS/SCRIPTS/BINARY) are read only when a
regexp with a property class is compiled; normalization tables (~22.8 KB) only by
`String.prototype.normalize`. Move the \p tables (and normalization if needed, or ~1 KB
of other cold data) out of the data segment into a custom section that the host hands over
on first need, so idle drops 448 → 384 KiB and every row by 64 KiB.

pstart's design (work/issues/0261-instantiation-cost.md, Follow-ups, "Protocol
NEED_TABLES"): modelled on 0257's NEED_STACK. The reader finds its family missing, the
compile/instruction stops before any effect with a distinct status, the host copies the
family from the module's custom section into memory through an op, and the instruction
runs again (as the elastic stack's retry). djs-core gets the bytes as a slice through the
heap/VM (no new unsafe); native, slim (if it has \p) and test builds keep the statics.
Estimated +1.0-1.6 KB gz code; the module has ~4 KB of room under the ceiling now.

## Acceptance criteria
- [ ] idle and per-workload memory, main vs branch (expect −64 KiB everywhere)
- [ ] identical results and fuel with and without the hand-over (a test like 0257's moved-stack equality), for regexp literals, `new RegExp`, lazy functions, eval/Function, normalize if moved
- [ ] hosts without the hand-over get a clear error or the statics (decide, record)
- [ ] the dump module and djs-embed unaffected or handled
- [ ] size reported; gate green; decisions recorded; guide updated

## Notes
- First confirm the arithmetic on the current build: where `__heap_base` lands, what
  exactly must leave the segment to reach the two-page line, and whether `\p` alone plus
  a small trim suffices (pstart suggested packing `bytecode::ops::OPS`, 4.3 KB, but it is
  on hot paths: only with a measured zero cost).
- pstart (0263) works on the compiler and djs-wasm's allocator (a compile-scoped bump
  zone): coordinate allocator/djs-wasm lines through the coordinator.

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
