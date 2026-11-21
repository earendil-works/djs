# 0265: Allocator placement effects; the resolver's re-emit

status: in-progress
agent: pstart
branch: issue/0265-placement-resolver
depends-on: 0263; part 2 after 0259 merges
touches: crates/djs-wasm/src/allocator (placement only, part 1), crates/djs-core/src/heap/arena.rs (growth/placement, part 1), crates/djs-core/src/compiler/resolve.rs and bytecode emission (part 2, after pcall's 0259 merge)
guide: arena.md (part 1), compiler.md (part 2)
decisions: D3360-D3369

## Goal
1. 0263 saw media's whole run get 15-17% faster from a change that only shrank the stack
   checker's buffers (repeatable, both orders), and D3342's compile zone raised memory
   2x by shifting where dlmalloc places blocks (the arena stops growing in place). Where
   the arena and the big Rust buffers land seems to move both time (first-touch faults
   ~6.6 µs/page, memory.grow ~2 µs, arena moves = copies) and memory. Explain the media
   effect (count page touches, grows, arena moves per run on both builds) and, if it is a
   real lever, make placement robust: e.g. Rust-side transient buffers never placed where
   the arena wants to grow, the arena at memory's end always, big transient buffers
   returned to the page allocator. Measure all workloads (time and mem-perturb).
2. After 0259 merges: the resolver's re-emit is ~30% of compile inclusive. Cheaper
   constant-pool hashing (~2%), folding the depth calculation into the emitter (~4%, keep
   a debug-build check), and fewer passes over each instruction where cheap.

## Acceptance criteria
- [ ] part 1: an explanation with counts; a change only if it helps time or memory without hurting the other (mem-perturb distributions, D2047/D2048)
- [ ] part 2: compile instructions per body, identical bytecode (golden fixtures, lazy/eager equality)
- [ ] size reported; gate green; decisions recorded

## Notes
- pcall (0259) is about to merge changes in resolve.rs: don't start part 2 before it's on
  main (I'll tell you).
- pscan (0264) adds an op and a custom-section hand-over in djs-wasm; pgc (0258) owns GC
  side tables and growth thresholds in heap/gc.rs.

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
- Instrumented builds (work/scratch/0265/instr.patch: memory.grow calls and pages, page
  runs, huge reallocs that move and bytes copied, dlmalloc segments, arena moves,
  collections; an event log of page-run allocations): main and the pre-D3341 build count
  exactly the same for every codemode workload, warm or first instance. No arena move in
  any warm instance; memory.grow 1-11 per run, collections 0-4.
- The media +15-17% (0263): not placement. It shows only between two particular shipped
  binaries: the same sources relinked with padded relocations (--keep-names) or with the
  counters compiled in run equally fast; with V8's wasm inlining off and eager TurboFan
  (--no-wasm-inlining --no-wasm-dynamic-tiering) the gap stays, so it lives in the
  module's code bytes: the pre-D3341 shipped binary is the slow outlier (its profile has
  the JSON parser's loop 4x heavier). Code-layout sensitivity of LLVM/V8, a lottery a
  one-line change can win or lose; not a lever, and a reason to confirm one-workload
  surprises on a relinked twin.
- First instances are where placement bites (D3360): the coded base held in dlmalloc.
  Dropped when the first VM takes the base: first-instance memory equals warm instances.
- With D3360 the compile zone of D3342 no longer moves memory (first and warm instances,
  all workloads): its regressions were first instances (and the variant that kept the
  zone forever). Measured again on top: compile −2..−5%, sequential run −3.6%, inspect
  −1.8%, +~220 B gz.
- Landed as D3361 (supersedes D3342) with a native allocator test; memory verified with
  memfirst.ts (first instances, 6 perturbed runs × 8 workloads), mem.ts (warm instances)
  and mem-perturb (8 workloads × 6): unchanged. Size: 425,611 (main) → 425,640 (D3360) →
  425,900 (D3361) B gz.

## Follow-ups

## Result
