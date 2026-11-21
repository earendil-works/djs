# 0257: Idle memory: elastic shadow stack, data padding, dlmalloc's first segment

status: in-progress
agent: pidle
branch: issue/0257-idle-memory
depends-on: -
touches: crates/djs-wasm (build.rs, linking, allocator's first segment, stack exports), crates/djs-core/src/compiler/parser.rs and regexp/parser.rs (depth limits as run-time values, a retry status), js/src/sandbox.ts and js/src/trap.ts (stack relocation), js/pi/engine.ts (if its instance reuse needs it)
guide: limits.md, arena.md, host.md
decisions: D3280-D3289

## Goal
The memory gap to QuickJS is mostly the idle gap (IDEAS.md 0.1): idle 576 KiB vs 320.
Each 64 KiB page off idle is ~6% of the memory geomean. Anatomy (0234): 128 KiB shadow
stack (QuickJS 64), 99 KB rodata + bss padded to 4 pages, dlmalloc's first segment 128 KiB
(~45 KiB used), the arena (base 182 KB + slack).
1. Elastic shadow stack (IDEAS.md idea 2): link with 64 KiB; the parser's nesting limit
   becomes a run-time value; a compile that would exceed the small stack's budget returns a
   distinct status before any effect, the host relocates `__stack_pointer` to a larger
   block and compiles again with the full limit (550). Same limit and SyntaxError as today:
   the semantics must not change (0234 declined halving the limit). Cover run-time compiles
   (`new RegExp`, `Function`, eval) too.
2. The static data's padding and dlmalloc's first segment: fit stack + data in fewer pages
   (e.g. seed dlmalloc with the leftover of the data page), shrink the first segment if
   the arena no longer needs it.
3. Whatever else idle memory holds that a fresh VM does not need (measure page by page).
Target: idle ≤ 448 KiB (two pages), with no run-time cost.

## Acceptance criteria
- [ ] idle and per-workload linear memory, main vs branch (run.ts, mem-perturb)
- [ ] js/test nesting tests and test262's deep-nesting cases pass unchanged; a test for the relocation path
- [ ] no measurable run-time cost on the workloads
- [ ] size reported; gate green; decisions recorded; guide pages updated

## Notes
- Overlap: 0256 (pstart) adds a grow-without-zeroing hook to djs-wasm's allocator and
  changes base injection in djs-wasm/src/lib.rs. Coordinate with them on allocator lines.
- trap.ts's stack-overflow report relies on the stack wrapping below 0 (D2543): keep the
  report meaningful or replace it.

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
