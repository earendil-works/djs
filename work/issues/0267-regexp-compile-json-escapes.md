# 0267: Regexp compile speed; escape-dense JSON

status: done
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
- Measuring (work/scratch/0267): rx-cm.js compiles 16 codemode-like patterns distinct
  (a unique `|@i` alternative) 80K times; rx-cm-base.js the same loop without `new
  RegExp`; wmeasure difference = the create path. main 5f5434e3: djs 41.6K wasm
  instructions per pattern, QuickJS (quickjs-wasi) 22.1K (1.88x); time 2.4 vs 1.2 µs.
  The corpus patterns (182, collect.mjs) are dominated by `\p{RGI_Emoji}` and `\p{…}`
  with `i` (5-9x QuickJS: case closure of large sets), not codemode-typical.
- Profiles: native (samply on a release test loop) parse 46%, compile 34%, tree drop 17%,
  malloc ~35%; wasm (create path) parser ~15%, compiler ~14%, first/follow sets ~10%,
  drop ~4%, allocator ~15%. Allocations per pattern: 6-27 in parse, 4-14 in compile.
- D3380: literal runs as one node: −7% (38.8K), +225 B gz. Tried and dropped: one-term
  alternatives without a Vec (−10.5% total but +422 B), bulk program_cell copy (0).
- JSON escapes: ~183 machine instructions per escape (scan + write; inner 80 KB text with
  3,266 escapes: 990K instr vs 393K with one escape). Escape positions recorded by the
  scan (writer copies 32 B unconditionally per run): inner −13% time / −1.5% instr, outer
  (double-encoded) +4% / +8%, heavy −1%, json/text −3% (noise), +525 B gz: not kept
  (work/scratch/0267/json-escape-positions.patch).

## Follow-ups
- Regexp compile at QuickJS speed needs a single-pass compiler (no tree) or an
  allocation-free tree: 1-2 days and 1-2 KB (the coordinator declined for now).
- Run-time compiles in the compile zone: a host switch costs ~380 B because a new Heap field
  moves offsets in every VM function; a cheaper carrier (a field already passed, or the
  zone keyed by something djs-wasm sees) would make it worth −8% of `new RegExp`.

## Result
Landed (D3380): regexp literal runs are one parser node (same matcher program), their
characters in one buffer of the parse, the compiler's buffer sized from the pattern.
Distinct-pattern compile (wasm instructions of `new RegExp`, 16 codemode-like patterns, on
main 2fec34fb): 41.0K → 36.9K per pattern (−10%; QuickJS 22.1K, 1.85x → 1.67x); size
+275 B gz (428,410 → 428,685). Workloads (instructions, n=3): no
change beyond noise (±3-5% on small bodies). test262 built-ins/RegExp, the regexp unit
tests (new: literal runs forward, backward, case-insensitive, escaped, quantified), the
corpus pass.

Not landed, measured (patches in work/scratch/0267/):
- JSON escapes cost ~183 machine instructions per escape over both passes (an 80 KB text
  with 3,266 escapes: 990K vs 393K with one), most of it the SWAR words of the run plus a
  mispredicted run end per pass, not the escape itself. Escape positions recorded by the
  scan: one-layer text −13% time, double-encoded +4%, workloads within noise, +525 B. A
  raw-position escaped-string scan: −4% instructions on the escape kernels, workloads
  within noise, +181 B.
- Run-time `new RegExp` in the compile zone (D3361) through a host switch: −8% more of the
  create path, +383 B.
