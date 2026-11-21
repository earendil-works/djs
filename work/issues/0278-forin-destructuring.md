# 0278: for-in and destructuring, size-neutral

status: in-progress
agent: pidle
branch: issue/0278-forin-destructuring
depends-on: 0270
touches: crates/djs-core/src/interp/run/iter.rs (for-in enumeration), heap key enumeration helpers, crates/djs-core/src/compiler (destructuring codegen: patterns, defaults), tests/corpus
guide: compiler.md if destructuring codegen changes
decisions: D3490-D3499

## Goal
From 0270's sweep: for-in over 5 keys 3.3x QuickJS (it re-interns each key on every step);
destructuring `{a, b = 1}` 1.8x and `[a, b]` 1.8x (codegen); both are everyday codemode
syntax (destructuring of tool results is everywhere). The module has ~100 B of room
(owner decisions pending), so this issue is size-neutral: every change must pay for itself
in bytes (better codegen often shrinks the compiler's output paths, and removing the
re-intern may remove code).

## Acceptance criteria
- [ ] per-op instructions vs QuickJS 2026 before/after; workload instructions
- [ ] exact semantics (test262 language/statements/for-in, destructuring; corpus with --suspend-every 97 --gc-stress); bytecode changes reviewed against lazy/eager equality and dumps
- [ ] shipped size ≤ main's (net ≤ 0 B gz); gate green; decisions recorded

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
