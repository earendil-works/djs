# 0266: Promise construction, then, async functions without await, new Error

status: done
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
- Measuring: work/scratch/0266/opsins.py (as 0262's: wasm instructions per op, node
  --no-liftoff, (2R − R reps) / ops) against quickjs-wasi and the QuickJS 2026 / NG
  builds; wl.py for workloads ((41 − 1 runs)/40, ±2-4% noise; a build with another
  heap layout runs with its own checkout's reader, `X.wasm@SRCROOT`).
- Items 1-3 were already done by 0232b (D3155-D3158): builtin closures' lazy
  length/name, @fast resolving functions/then/Promise.resolve, the async state at the
  first await, allSettled/any element jobs in Rust. Measured on main 5f5434e3 (instr per
  op, djs vs QuickJS 2026 / NG): `p.then(f)` + job 9.3k vs 9.0k / 8.6k; async call
  without await 2.5k vs 8.5k; await of an async call 6.5k vs 10.1k; Promise.all of 20
  13.2k vs 21.0k; allSettled 15.0k vs 24.3k; any of 20 rejected 23.6k vs 20.7k (1.14x);
  `new Promise(() => {})` 7.8k vs 6.0k; `new Error("x")` 28.0k vs 24.2k / 23.4k
  (quickjs-wasi 19.1k).
- new Error (D3370): with Error.stackTraceLimit = 0 it still cost 18.1k, so the
  constructor was most of it: five prelude frames (ErrorConstructor, ErrorConstruct,
  GetPrototypeFromConstructor, InstallErrorCause, RecordStack), two trips through `step`
  (DefineDataPropertyOrThrow for `message`, CaptureStack), and `atom_str` of
  "stackTraceLimit" and "prepareStackTrace" (keyed hashing) per error. The trace itself:
  a Rust String per name and file (decode_utf16, malloc), `cat_to!`, a UTF-8 round trip,
  and `lookup_line` through the generic iterator. Fixed each; see D3370.
- Promise constructor: GetPrototypeFromConstructor's leaf path written out (−0.3-0.5k).
  The rest of `new Promise`'s gap is the construct path (`new C()` itself 2.9k vs 1.9k)
  and the intrinsic dispatch of the prelude's leaves, not promise code.
- Results, main 5f5434e3 → branch (instr per op; QuickJS 2026, NG, quickjs-wasi):
  `new Error("x")` 28.0k → 15.4k (24.2k, 23.4k, 19.1k); `new Error()` 24.5k → 14.1k;
  one more frame 35.2k → 20.5k (30.8k); throw/catch of an Error 28.9k → 16.2k (24.5k);
  `new Promise(r => r(1))` 9.3k → 8.8k (6.3k); `new Promise(() => {})` 7.8k → 7.6k (6.0k);
  the others unchanged. Workloads (wl.py): fanout −4%, loop −2%, sequential +4%, heavy,
  text, json ±1% (all within the noise).
- Size: shipped 427,455 → 427,616 B gzip (+161) on main 5f5434e3.
- Tests: corpus errors/error-construct-order.js (the constructor's order with a proxy
  new.target and getters, message/cause attributes, subclasses, AggregateError, frames
  of named, anonymous, index-named, getter and callback functions, stackTraceLimit,
  prepareStackTrace CallSites); test262 Promise, AsyncFunction, Error, NativeErrors,
  AggregateError, SuppressedError; difftest builtins/errors/codemode with
  --suspend-every 97 --gc-stress and --plain-dumps --lazy.
- Guide: no page describes the pieces changed (checked interpreter.md, builtins.md,
  suspension.md).

## Follow-ups
- Stack frames of methods with computed or symbol keys show `<anonymous>` (qjs shows the
  name): the trace reads the template's name, which SetFunctionName sets only on the
  function object. Same on main.
- With Error.prepareStackTrace, qjs's CallSites include one more (harness) frame than
  djs's at a codemode body's top level. Same on main.
- `new Promise` 1.3-1.4x and Promise.any 1.14x QuickJS: the construct path and the
  prelude's intrinsic dispatch (0236/0259 areas).

## Result
Items 1-3 had landed with 0232b (measured: then 1.04x, async without await 0.3x,
allSettled 0.6x, any 1.14x QuickJS). New here (D3370): `new Error` with one prelude frame
instead of five, `message` defined with the object, `CaptureStack` as a pure leaf, the
two Error constructor properties as well-known atoms, traces written from heap strings
and a faster line lookup: `new Error("x")` 28.0k → 15.4k wasm instructions (QuickJS
2026 24.2k), throw/catch 28.9k → 16.2k; `new Promise` −3-6%. +161 B gzip.
