# 0263: Compile speed for small bodies; static data audit

status: open
agent: pstart
branch: issue/0263-compile-speed
depends-on: -
touches: crates/djs-core/src/lexer, crates/djs-core/src/compiler (parser, resolve, func state, completion), crates/djs-core/src/bytecode (code buffers, stack.rs walk), Rust-side scratch allocation of the compiler; static data tables (read-only data) for item 2
guide: compiler.md if the pipeline changes
decisions: D3340-D3349

## Goal
1. Compiling is 14-17% of the small bodies (sequential, inspect; BRIEF.md) and Rust-side
   malloc/Vec growth ~10%, much of it the compiler's (code buffers, func state, blocks).
   Ideas (IDEAS.md idea 10): fold the stack-depth pass (`bytecode::stack::walk`) into the
   resolver's re-emit; a resettable scratch zone / reused buffers instead of per-compile
   Vec growth; anything the profile of a compile of the codemode bodies shows. Measure
   compile-only instructions (scripts/perf/wcompile.py, wasm-compile.ts) and the small
   workloads.
2. Static data audit (0261's follow-up): is the 8.2 KB intrinsic name table needed in the
   shipped module? What is the unidentified 4.3 KB constant? Every KB out of the data
   segment brings the stack + data closer to the two-page line (needs ≤131,072 B, now
   164,659) and shrinks the module.

## Acceptance criteria
- [ ] compile instructions per byte and small-workload times, main vs branch
- [ ] identical bytecode (or justified differences) — the corpus, test262, golden fixtures and lazy/eager equality (vm::lazy_tests) stay green
- [ ] size reported (prefer shrinking); gate green; decisions recorded

## Notes
- 0259 (pcall) changes the compiler to recognize builtin call sites (resolve / call
  emission) and adds ops: coordinate through the coordinator before touching call
  emission in resolve.rs; rebase often.

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
