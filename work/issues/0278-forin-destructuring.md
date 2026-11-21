# 0278: for-in and destructuring, size-neutral

status: done
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

2026-11-21 (pidle)
- Re-measured with `--wasm-sync-tier-up` (main 93bcde52; wasm instructions per op vs
  QuickJS 2026): destructuring is fine: {a,b} 1.2k vs 2.1k, {a,b=1} 2.5k vs 2.9k,
  {a:{b}} 1.5k vs 2.5k, [a,b] 5.1k vs 4.5k, [a,,b] 5.7k vs 4.9k, param destructuring
  1.7k vs 2.0k, with defaults 3.6k vs 2.8k, for-of [k,v] 29k vs 21k, swap 6.8k vs 5.1k
  (0270's 1.8x came from `--no-liftoff`). The outliers: object rest 48k vs 9.5k and
  for-in (5 keys 20.5k vs 8.6k, an array of 10 60k vs 17k).
- for-in (D3490): enumerable keys only, read from the slot flags; shadowing checked by
  walking the chain; steps look up existing atoms only. 5 keys 20.5k → 13.2k, 20 keys
  51.3k → 37.7k, array of 10 60.5k → 29.9k. −50 B gz.
- Object rest (D3491): Rust path, 48k → 13.6k, ~+400 B gz: parked as
  work/research/0278/object-rest.patch on the coordinator's call.
- Size experiments: gzip of the module moves ±50 B with layout; merging monomorphized
  copies (an `object_of` helper for 12 `.filter(class ==)` closures: −243 raw, +101 gz)
  can lose gzip.
- Tests: lib, e2e plain, active test262, difftest plain/stress/plain-dumps --lazy on
  language, builtins, codemode; corpus case language/for-in-keys.js. Guide: none
  affected (no codegen change).

## Follow-ups
- Object rest (D3491) when there is room.
- for-in is still 1.5x: the snapshot allocates a ValueArray and a Vec per object of the
  chain; Object.prototype's keys are still walked (a pristine-builtins skip cost 100 B).

## Result
for-in 1.5-1.75x faster (D3490) at −50 B gz; destructuring needed nothing (the sweep's
1.8x was a measuring artifact); object rest parked (D3491). 0270's sweep table corrected.
