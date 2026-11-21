# 0266: Promise construction, then, async functions without await, new Error

status: in-progress
agent: pidle
branch: issue/0266-async-promise-error
depends-on: -
touches: builtins/promise.ts and its generated/native parts, crates/djs-core/src/interp (async function call/suspend: state allocation; promise jobs), crates/djs-core/src/compiler (only if the async state allocation moves to the first await), builtin closure creation (lazy length/name), Error construction and CaptureStack, tests/corpus
guide: suspension.md, interpreter.md if the async story changes
decisions: D3370-D3379

## Goal
Codemode bodies are async and make tool calls through promises (22% of bodies fan out
with Promise.all). Left over from 0232 and 0236 (their Follow-ups):
1. `new Promise(executor)` and `p.then(f)` cost ~4-6x QuickJS in instructions:
   PromiseConstructor and Promise.prototype.then are TS (SpeciesConstructor,
   NewPromiseCapability), and every builtin closure (resolving functions, element
   functions) defines `length` and `name` through define_own_property (an allocation and
   two validated defines each). Lazy length/name for builtin closures (like closures,
   D2445's flags) and a Rust fast path for `then` with a plain species.
2. An async function that never awaits still allocates its 7-slot state object next to
   its promise: allocate it at the first await (`async f()` without await ~1.25x QuickJS
   → parity).
3. allSettled/any element jobs still run the TS path.
4. `new Error`: 30.1k vs 18.7k wasm instructions; ~60% is CaptureStack (trace_frames
   converts file and function names to Rust Strings with decode_utf16 and malloc), and
   DefineDataPropertyOrThrow/CaptureStack go through step.
Measure each with wmicro against QuickJS 2026 and on the workloads (fanout, heavy,
sequential, loop use promises).

## Acceptance criteria
- [ ] per-op instructions vs QuickJS before/after; workload instructions
- [ ] exact semantics: corpus + test262 built-ins/Promise, AsyncFunction, Error; suspension and dumps of pending promises and async frames unchanged in meaning (difftest --plain-dumps, --suspend-every 97, --gc-stress)
- [ ] size reported (room ~3 KB under the ceiling, shared); gate green; decisions recorded

## Notes
- pcall (0259) owns the builtin call path (callee lookup, fast_leaf_call, decline_fast);
  pgc (0258) the collector. pscan (0264) adds a stop status in interp for missing tables.

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
