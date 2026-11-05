# 0232: Performance: Promise.all, await, async calls

status: done
agent: pasync
branch: issue/0232-perf-async
depends-on: -
guide: suspension.md / interpreter.md if the async story changes
touches: builtins (promise), interp (await, async function call/resume), job queue

## Goal
Measured by benchpage (0229) against the best QuickJS wasm build (QuickJS 2026-06-04 / QuickJS-NG 0.18, wasi-sdk -O2), node 26; scripts in .worktrees/benchpage/work/scratch/0229/ops.ts, ops2.ts (need vendor/engines from scripts/bench-engines.sh --setup on that branch).
- Promise.all over 20 promises 3.1-3.4x slower; await of a resolved value 1.9x; async function
  call + await 1.7x.
Codemode is async-heavy (fan-out over tools is 22% of real bodies), so this matters. Find the root
causes (reaction records, job queue, frame suspend/resume, Promise.all's per-element closures)
and fix; target parity with QuickJS. Dumps must keep working (the plain-dump difftest leg and
fuzz regress: node scripts/plain-fuzz/fuzz.mjs regress, with and without --lazy).
Constraints: exact semantics (test262 and the difftest stay green); measure in wasm instructions on the shipped module before/after and report size cost (the shipped module has ~15 KB gzip headroom shared by 0230-0234: keep each change small, report bytes; the coordinator arbitrates); several agents work in djs-core at once (0230 builtin calls, 0231 arrays/strings/collections, 0232 async, 0233 regexp, 0234 memory): stay in your touches, ask the coordinator before changing shared paths (the interpreter's call path, the heap). Batch merges, gate checks locally, niced builds, benchmarks only at load < ~4.

## Log
- 2026-11-06 pasync: measured with scratch work/scratch/0232/amicro.py (machine instructions per
  op of `node --no-liftoff` running the shipped-profile module, body x N minus x 0; N large, small
  N is dominated by V8 noise) and instr.py (bytecodes per builtin). Root causes: an `await` ran
  ~133 bytecodes in 9 builtin frames (AsyncAwait, PromiseResolve, NewPromiseCapability,
  ResolveCapability ... PromiseReactionJob) plus 2 allocations; `RealmIntrinsic` allocated a Rust
  String per call; an async return ran AsyncFunctionReturn + ResolvePromise (29 bytecodes);
  Promise.all ran ~440 bytecodes per element (Promise.resolve, then, SpeciesConstructor,
  NewElementRecord, closures) plus ~180 in its reaction job.
- Fixes (D3130-D3132): await / async_return / async_reject subscribe and settle in the op when no
  user code can run, and an await's reaction job resumes the function as the job's bottom frame;
  PromiseCombinatorRun does Promise.all/allSettled/any/race elements of an Array iterator in Rust;
  next_job settles Promise.all resolve-element jobs in Rust; RealmIntrinsic reads its name in place.
  Heap state after every fast path equals the builtins' (dumps unchanged). %AsyncFunctionThrow
  removed (registry tombstone).
- scripts/perf/instr.py: anchors and two API uses brought up to date (it no longer built).
- Tests: tests/corpus/builtins/promise-fast-paths.js makes each skipped step observable (then
  getters, Proxy, own constructor, patched then/resolve/species, Array.prototype.then, subclass,
  job order). test262 active group, difftest (plain, stress --lazy --suspend-every 97 --gc-stress
  --determinism), plain dumps in djs and node, fuzz regress with and without --lazy: see Result.
- Verification: test262 active group (ratchet OK), difftest plain and stress (--lazy
  --suspend-every 97 --gc-stress --determinism) OK, plain dumps leg (1521 links, node 0 differ)
  OK. fuzz regress: regress/ check 17 failures, the same on main (cf826382); fresh seeds found
  seed 344676 ("cannot read property 'prototype' of undefined" at plain load) and with --lazy seed
  208872 ("not an object" at plain load), both failing identically with main's difftest binary:
  pre-existing dumper bugs, not this change (reported to the coordinator).
- Guide: suspension.md (await without %AsyncAwait in the common cases). interpreter.md unaffected.
- Outside touches: interp/run.rs next_job (job loop, now a loop with `job_failed`), the
  REALM_INTRINSIC arm in intrinsics.rs, iter.rs `iter_next_peek`, vm/tests.rs
  vm_call_never_collects (its body needs a call to reach a safepoint now that a plain async
  return enters no builtin frame). Not the call path.

## Follow-ups
- `new Promise(executor)` and `p.then(f)` stay ~4-6x QuickJS in instructions: PromiseConstructor
  and Promise.prototype.then are TS (SpeciesConstructor, NewPromiseCapability), and every builtin
  closure (resolving functions, element functions) defines `length` and `name` through
  define_own_property, an allocation and two validated defines each. Builtin closures could get
  lazy `length`/`name` like closures (heap change, D2445's flags), and `then` a Rust fast path for
  a plain species.
- An async function that never awaits still allocates its state object (7 slots) next to its
  promise; allocating the state at the first await would make `async f()` without await
  ~1.25x QuickJS → parity (compiler + suspend.rs).
- allSettled/any element jobs still run the TS path (only Promise.all's resolve element is done
  in next_job).

## Result
Await, async return/reject and await resumption need no bytecode builtin when no user code can
run (D3130); Promise combinators do the elements of an Array in Rust and Promise.all's element
jobs settle in next_job (D3131); %RealmIntrinsic stops allocating (D3132). Exact semantics: the
fast paths leave the heap as the builtins would and decline wherever a spec step could be
observed (tests/corpus/builtins/promise-fast-paths.js).

Machine instructions per op, wasm module under `node --no-liftoff` (work/scratch/0232/amicro.py,
large N), main → this branch, quickjs-2026-06-04 / quickjs-ng-0.18 (wasi-sdk -O2):
| op | main | now | quickjs | quickjs-ng |
| --- | ---: | ---: | ---: | ---: |
| `n += await 1` | 26.2 k | 3.8 k | 9.3 k | 8.8 k |
| `n += await f(1)`, f async | 20.5 k | 7.2 k | 10.1 k | 10.3 k |
| `f(1)` async, no await | 14.0 k | 9.9 k | 7.9 k | 6.4 k |
| Promise.all of 20 async calls (incl. Array.from) | 1529 k | 338 k | 424 k | 407 k |
| Promise.all of 20 resolved promises | 1323 k | 220 k | 257 k | 254 k |

js/bench/engines/ops.ts (djs/best QuickJS, load 110-180, noisy): await value 50k 2.14 → 0.16,
async call+await 20k 2.86 → 0.12, Promise.all 20 x500 4.58 → 1.17 (that op also times its
Array.from callback loop).

Size (D3133, vs main 09b8cb8f): shipped module 964,175 → 969,696 bytes (+5,521), gzip -9 420,075 → 422,256 (+2,181); slim +2,327 gzip.
Most of it is the combinator step (~2.1 KB raw) and the next_job fast paths (~1 KB).
