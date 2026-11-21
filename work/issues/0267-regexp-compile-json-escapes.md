# 0267: Regexp compile speed; escape-dense JSON

status: open
agent: pscan
branch: issue/0267-regexp-compile-json-escapes
depends-on: 0264
touches: crates/djs-core/src/regexp (parser, compiler to the matcher program), crates/djs-core/src/json (stringify's escape writer, the scan's record of escape positions), crates/djs-core/src/swar.rs, tests/corpus
guide: builtins.md if a fast-path story changes
decisions: D3380-D3389

## Goal
1. Compiling distinct regexp patterns is ~2x QuickJS (0233's follow-up): the parser builds
   a tree with a node per character, dropped right after. Literal runs as one node, or
   emitting without a tree. Codemode bodies compile each literal regexp once per run
   (regexp-compile in the native engine track; loop/text/heavy bodies hold several), so
   this is per-run latency on small bodies. Measure compile instructions per pattern over
   the patterns of the codemode corpus (tests/corpus, tests/bench/codemode).
2. Escape-dense JSON still costs ~12 ns per escape (0260's follow-up): let the scan
   record escape positions for the writer (or another way to cut per-escape work);
   roughly halves it by your estimate.

## Acceptance criteria
- [ ] per-pattern compile instructions vs QuickJS before/after; per-escape JSON costs; workload instructions
- [ ] identical matching (test262 built-ins/RegExp, the corpus, the regexp unit tests; the matcher program may change, its behavior may not)
- [ ] size reported (room ~2.5 KB under the ceiling, shared); gate green; decisions recorded

## Notes
- pstart (0265) works on dlmalloc's needs and the GC side tables; pgc (0258) on GC
  thresholds; pidle (0266) on promises/Error; pcall (0259) on builtin calls.

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
