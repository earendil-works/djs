# 0037: P2 Promises, job queue, async host calls, suspendable frames

status: open
agent: -
branch: issue/0037-p2-promises-jobs
depends-on: 0035
touches: crates/djs-core/src/interp/, crates/djs-core/src/vm/, crates/djs-core/src/heap/ (additive cells), builtins/promise.ts (new) + intrinsics.d.ts additions + gen, crates/djs-wasm/ + js/src/sandbox.ts (async host calls), docs/host.md

## Goal
ARCH.md "Interpreter" (generators and async functions, promises and jobs) and "Host
Interface" (async host functions, the pending host call table, `Idle`), phase P2. This is
the core of codemode: every `tools.*` call is an async host call, and scripts are async
function bodies. This issue builds the **runtime**: the compiler side of
`async`/`await` (and generators) comes after 0036 from the lexer agent, and will target
the ops defined here.

## Acceptance criteria
- [ ] Promise in the builtin language (`builtins/promise.ts`): the constructor,
      `then/catch/finally`, `resolve/reject/all/allSettled/any/race/withResolvers/try`,
      thenable adoption, species. Leaf intrinsics for the internal slots (state,
      result, reaction lists) and HostPromiseRejectionTracker-style unhandled rejection
      tracking (qjs reports unhandled rejections: check what codemode/difftest expects).
- [ ] The microtask job queue as a heap structure (a root, ARCH "Roots"). Running a job
      pushes a frame. `run(fuel)` drains jobs after the main frame finishes or parks
      (spec order), and returns `Idle` when only pending host calls remain.
- [ ] Async host functions: calling one creates a promise and a pending host call
      `{call_id, fn_id, args, promise}` in the heap table. Execution continues. The
      embedder reads new pending calls after `run`, resolves or rejects them
      (`resolve_host_call(id, ok|err, value)`), which enqueues the reactions. Multiple
      outstanding calls at once (`Promise.all` over tools runs them concurrently: the
      0023 follow-up). Sync host calls stay as they are.
- [ ] Suspendable frames for await and yield: ops that save the current frame's slice of
      the value stack (frame header, locals, operand stack, pc) into a heap
      generator/async object, and resume it later (ARCH "Generators and async
      functions"). Async function objects: the return promise, resuming on settlement
      via a job. Define the ops in the ops! table and document them in
      docs/compiler.md for the compiler side, and test them with hand-assembled bytecode
      (as 0015 did).
- [ ] Snapshots: a VM parked with pending host calls and queued jobs round-trips through
      heap images (ARCH "Durability"); a test covers it.
- [ ] djs-wasm + js/src/sandbox.ts: async host functions end to end (a JS host function
      returning a promise → a pending call → resolved later → the VM continues),
      several at once.
- [ ] e2e and difftest cases using Promise directly (`Promise.resolve(1).then(...)`,
      ordering of microtasks vs host calls), checked against qjs. Ratchets updated.
- [ ] Also, from the 0035 review: `tests/corpus/dquickts/builtin/test_rope.js` became
      very slow under `--gc-stress` (a 600 s run timed out). Find out why (repeated
      flattening, long rope chains copied at every safepoint?) and fix it or log it.

## Notes
- The lexer agent does 0036 (destructuring) in compiler/ right now. Do not change the
  compiler beyond what the new ops strictly need (nothing, ideally).
- Land in parts: Promise + job queue first; then async host calls; then frames.

## Log

## Follow-ups

## Result
