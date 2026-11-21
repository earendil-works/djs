# 0270: The construct path, and a sweep of everyday codemode operations vs QuickJS

status: in-progress
agent: pidle
branch: issue/0270-construct-op-sweep
depends-on: 0266
touches: crates/djs-core/src/interp/run/construct.rs (decline_construct, super calls), compiler (only a super_call op if adopted), builtins for the operations the sweep finds (coordinate: string ops are pgc's 0269, regexp/JSON pscan's 0267, call path/run_hot pcall's 0268), js/bench/engines/ops.ts (new rows), tests/corpus
guide: interpreter.md, benchmarks.md (new ops rows are generated: regen, not by hand)
decisions: D3410-D3419

## Goal
1. The construct path: `new C()` 2.9k vs 1.9k QuickJS instructions, derived classes
   1.2-1.6x (0236's follow-up: ~10 cheap declines per `new D()`; a `super_call` that reads
   the active function and new.target from its frame would remove three); `new Promise`
   1.3-1.4x rests on it.
2. A sweep: the ops table was written for 0229 and the workloads are nine synthetic
   bodies. Take the operations real codemode bodies use most (tests/corpus/codemode and
   codemode-harvest: count builtin calls and syntax kinds over the corpus), measure the top
   ~40 per op in wasm against QuickJS 2026 (wmicro or an ops.ts extension), and list every
   one above 1.2x with its cause. Fix the cheap ones in your areas; route the others to the
   coordinator with numbers. Candidates nobody measured yet: Date (now, toISOString,
   new Date(string)), Object.keys/values/entries over plain objects, for-in, destructuring
   with defaults, optional chaining, spread of arrays into calls, Array.prototype.flatMap,
   find/findIndex/some/every, string comparison and localeCompare, Number parsing,
   JSON.stringify with indentation, console formatting of objects.

## Acceptance criteria
- [ ] the sweep's table (op, djs, QuickJS, ratio, cause) in the issue; ops.ts rows added for the important ones
- [ ] construct-path numbers before/after; the fixes landed with tests
- [ ] size reported (~2 KB room, shared); gate green; decisions recorded

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
