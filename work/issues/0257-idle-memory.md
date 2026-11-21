# 0257: Idle memory: elastic shadow stack, data padding, dlmalloc's first segment

status: done
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
- Measured (main a0d85b06, shipped wasm): idle 576 KiB = stack 2 pages + data 99,043 +
  bss 1,240 (4 pages with the stack) + dlmalloc 2 + arena 3. Stack high-water of the
  workloads ~6.5 KiB (11 KiB for the first instance's base decode); per nesting unit
  ~112–160 B release, ~225 B debug; regexp groups ~512 B/level. Codemode corpus (339
  scripts) nests at most 44 parser units.
- Batch 1 (D3280, D3281, D3282): elastic stack (64 KiB linked; small limits 200 units /
  32 regexp levels; NeedStack retry, host moves the stack to a 128 KiB block via
  `djs_stack`, js/src/trap.ts `elasticStack`), dlmalloc's first segment seeded with the
  data page's rest, PRIME 1. Idle 576 → 448, every workload −128 KiB but heavy; slim
  idle 512 → 384. +1,051 B gz; ceiling +182 approved by the coordinator.
- Size: inlining `Parser::enter` (once its check moved out of line) made LLVM merge
  parse_object/array/… into parse_primary (+1.1 KB, bigger frame): enter is
  `#[inline(never)]`.
- Guide pages updated: limits.md (nesting in source), host.md (the stack protocol,
  exports, status 6), arena.md (dlmalloc's first memory). docs/host.md and
  docs/compiler.md 2.4 too. New gate leg: nesting test on the shipped module.
- Merged batch 1 (86b42954). Batch 2, size offsets (D3283): regexp `err` and
  `Parser::unexpected` out of line, djs-wasm exports borrow staged text: −427 B gz
  (429,780 → 429,353); ceiling back to 429,648.

## Follow-ups
- Stack + data in two pages (another −64 KiB): needs ~2 KB less data or a ~30 KiB stack
  (then small limits ~100 units); idea 11 (Unicode tables out of linear memory) would
  make room. Not done.
- heavy's peak is unchanged (4736 KiB): pgc's territory.

## Result
Idle linear memory of the shipped module 576 → 448 KiB (target met), every codemode
workload −128 KiB except heavy; slim idle 512 → 384. No measurable run-time cost. Same
nesting limits and errors; a compile deeper than the small stack allows runs again on a
128 KiB block the host provides (hosts other than the Sandbox/pi adapter: call
`djs_elastic_stack(1)` and wrap with `elasticStack`, or keep it off and get the old
behaviour on a 64 KiB stack). Size +1,051 B gz (D3282), then −427 B gz of offsets
(D3283): net +624 B over main before 0257; ceiling back at 429,648.
