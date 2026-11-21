# 0271: A smaller base image: idle at QuickJS parity (320 KiB)

status: open
agent: pstart
branch: issue/0271-base-image-diet
depends-on: 0265
touches: research first; then the base image's composition (crates/djs-prelude, builtins/ build options), the template layout for privileged code (heap/cells.rs FuncTemplate and its readers, bytecode/template.rs), the js reader layout (regenerated); coordinate every interp reader change through the coordinator
guide: heap.md, snapshots.md, builtins.md if the realm changes
decisions: D3420-D3429

## Goal
Idle is 384 KiB (stack 64 + data in 2 pages total 128 KiB; dlmalloc's primed page 64 KiB;
the arena 3 pages for a ~184 KB base). QuickJS is 320. The arena fits in 2 pages if the base
(plus the per-VM tail and the first session cells) fits in 128 KiB: ~56 KB less base.
Where it goes (IDEAS.md 0.3 and 6): 863 templates with 48-byte headers (41 KB) over 39.5 KB
of bytecode, 863 function objects (20.7 KB), 25.7 KB of name strings, 15 KB of PropTables.
Options: a compact template kind for privileged code (16 B header: −27.6 KB), function
objects reified on first read (−20.7 KB, IDEAS idea 3 stage 2: needs a tag test on prefix
reads), families no codemode run touches left out of the shipped base or loaded as a second
chunk on first use (typed arrays, iterator helpers, Proxy/Reflect, DisposableStack, WeakRef,
...; docs/empty-heap.md and docs/frozen-builtins.md 6), shorter name strings.
Research first: a script over base.img computing the image size under each rule, the per-row
memory effect (does the arena really drop a page, given the arena's start and growth rules,
D3140/D3141), the CPU cost of each (wmeasure), the size cost; report the ranked options to
the coordinator before building.

## Acceptance criteria
- [ ] the ranked options with numbers in the issue
- [ ] built: idle and every codemode row −64 KiB (or the reason it can't be done) at no measurable CPU cost; the codemode-memory leg updated
- [ ] semantics unchanged for scripts (frozen builtins mode included); images and dumps consistent; size reported
- [ ] gate green; decisions recorded; guide updated

## Notes
- pcall (0268) works in run_hot and the call path, which read FuncTemplate fields at
  frame entry: any layout change there goes through the coordinator and their twin check.
- pidle (0270) adds ops and changes class codegen; pgc (0269) strings and atoms.

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
