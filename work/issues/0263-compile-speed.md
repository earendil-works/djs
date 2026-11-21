# 0263: Compile speed for small bodies; static data audit

status: done
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
- [x] compile instructions per byte and small-workload times, main vs branch
- [x] identical bytecode (or justified differences) — the corpus, test262, golden fixtures and lazy/eager equality (vm::lazy_tests) stay green
- [x] size reported (prefer shrinking); gate green; decisions recorded

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
- Static data (0261's map): the 4,280 B anonymous constant is `bytecode::ops::OPS`, 214
  `OpInfo` records of 20 B (imms slice 8, size, pops/pushes with their Option<u8>s, cost,
  flags); essential at run time. Packing it (imms as an index into one table, the Options
  as a byte) would save ~1.5 KB but it sits on hot paths (`OPS[op].cost`, `size`): not now.
  The intrinsic names: D3340 (−8.2 KB data, −4.1 KB gz).
- Compile profile (wasm keep-names, steady state, sequential+inspect bodies): parse 38%,
  `finish_unit` 38% (emit_function 28%, alloc_template_in 9% of which the stack walk 8%),
  lexer 8%, dlmalloc+Vec growth ~10%. Natively (samply, cbench in work/scratch/0263): the same
  shape, malloc 22% there. A compile allocates 425-605 times (alloc-stats module), peak
  ~40 KB above the arena.
- Merged batch 1 (eba51b55).
- D3341 stack walk in code order: compile −3..−4%, +126 B gz.
- D3342 compile bump zone: −2..−7% compile but +320 KiB (sequential) / +1.3 MiB (fanout)
  wasm memory through dlmalloc placement: dropped (zone.patch).
- D3343 string literals without copies: compile −2..−5%, −59 B gz.
- EarlyStop of pattern scans in every build: no change, +390 B: dropped.
- A/B (work/scratch/0263/cab.ts: compile only, interleaved, 2,000 compiles per body, load
  ~10; main eba51b55 vs branch): sequential 57.9 → 53.3 µs (−7.8%), inspect −5.3%, media
  −6.0%, fanout −6.7%, json −4.7%. Whole runs (ab2.ts, 800 interleaved fresh sandboxes):
  sequential −2.1..−3.0%, inspect −1.7%, fanout −2.1%; media −15..−17% (repeated three
  times, both orders) from D3341 alone, though media's compile is only 6% faster: the
  profile of media is JSON and GC, so the walk's smaller scratch buffers likely change
  dlmalloc placement and the arena's growth; not pinned, may not hold under other changes.
  Memory per workload unchanged (mem.ts).

## Follow-ups
- What is left of a compile is spread thin (wasm, steady state, after this issue):
  `finish_unit` 38% (emit_function 30% incl., 8.6% self; alloc_template_in 9%, the stack
  walk ~4% of it), lexing 9%, pattern scans 4%, dlmalloc + Vec growth ~10%, UnitPool's
  keyed hashing of constants 2%. The larger remaining items are in compiler/resolve.rs
  (0259's file): `UnitPool::intern` could hash with a per-compile unkeyed hash (the pool
  never reaches a heap table), ~2%; folding the depth computation into the re-emit (idea
  10) would remove the walk's decode, ~4%, but trades the independent check for one in
  the emitter. A structural win (fewer passes over each instruction: phase-1 encode,
  decode, peephole, re-encode, jump shortening, walk) is a larger project.
- Allocation churn per compile (~500 allocations) stays: a bump zone was faster but moved
  dlmalloc's placement (D3342). Fewer, pre-sized Vecs per function would get part of it
  without the placement risk.
- `bytecode::ops::OPS` (4,280 B, 214 × 20 B) could pack to ~13 B per op (−1.5 KB data) if
  the hot reads (`cost`, `size`) stay one load; with the `\p` tables it reaches the
  two-page line.
- Media's run got 15-17% faster with D3341 alone though its compile is 6% faster: not
  pinned (dlmalloc placement after the walk's smaller scratch, most likely).

## Result
Landed (main eba51b55, 26d5de4a): D3340 (intrinsic names only where read: −4,145 B gz
shipped, −4,027 slim, data segment −8.2 KB), D3341 (stack walk in code order: compile
−3..−4%, +126 B), D3343 (string literals without copies: compile −2..−5%, −59 B). Compile
of the codemode bodies −4.7..−7.8% (wasm, interleaved); whole small runs −2..−3%; memory
unchanged; net size −4,078 B gz. Measured and dropped: a compile bump zone (D3342: faster
but moved wasm memory), early pattern-scan stops in every build (no gain, +390 B).
