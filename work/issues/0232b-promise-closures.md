# 0232b: Performance: builtin closures, async state at first await, allSettled/any jobs

status: in-progress
agent: pasync
branch: issue/0232b-promise-closures
depends-on: 0232
touches: heap/object.rs (lazy length/name), interp/run (intrinsics, promise, suspend), compiler (async_start) if needed, builtins/promise.ts
guide: suspension.md / heap.md if the async or closure story changes

## Goal
Follow-ups of 0232 (D3130-D3133). Decision IDs D3155-D3159.
1. `new Promise(executor)` and `p.then(f)` are 4-6x QuickJS in instructions: every builtin
   closure (resolving functions, element functions) defines `length` and `name` through
   define_own_property. Make them cheap (lazy, like closures' D2445, or a precomputed shape),
   with exact property order and observable behavior, and dumps working.
2. Allocate an async call's state object at its first await (an async call without await is
   ~1.25x QuickJS).
3. allSettled/any per-element jobs in Rust, if cheap.
Measure with wmicro (wasm instructions); keep the size small. Dumps: plain-dump difftest leg
and fuzz regress with and without --lazy.

## Acceptance criteria
- [ ] new Promise / then within ~1.5x QuickJS, or the remaining cause documented
- [ ] async call without await at QuickJS parity, or the remaining cause documented
- [ ] test262, difftest (stress), plain dumps and fuzz regress (both modes) green
- [ ] size cost measured and recorded

## Log
- 2026-11-06 pasync: builtin closures' `length`/`name` lazy (D3155): `new Promise` 25.6 k ->
  21.7 k instructions; the rest was the TS resolving functions and then. @fast entries (D3156):
  then 30.3 k -> 12.1 k (qjs 10.8 k), new Promise -> 14.1 k (qjs 5.2 k). Async state at the
  first await (D3157): async call without await 4.3 k -> 3.6 k (qjs 7.3 k, numbers at N=100k).
  allSettled/any element jobs (D3158): allSettled of 20 662 k -> 233 k (qjs 333 k), any of 20
  rejected 544 k -> 285 k (qjs 279 k). @fast Promise.resolve.
- Tried and dropped: `@native` PromiseConstructor: -10% for +1.6 KB raw.
- `new Promise` stays ~2.7x QuickJS: the general construct path is. `new C()` of a user class
  is 2.4x (5.9 k vs 2.5 k) and `new Map()` 4.6x (9.9 k vs 2.1 k) on the same build; `new
  Promise(() => {})` 12.5 k vs 5.1 k. Not promise code: listed as a follow-up.
- scripts/perf/instr.py anchors updated again (heap/gc.rs, compactor.rs moved).
- Tests: tests/corpus/builtins/promise-fast-paths-2.js (closure keys/attributes/redefine/delete/
  freeze, resolve twice/self/thenable, then species reads, subclass, newTarget proxy/plain/
  primitive prototype, async without await incl. order and thenables, allSettled with a
  Object.prototype.status setter, any's AggregateError).

## Follow-ups
- The construct path: `new C()` of a user class 2.4x QuickJS, `new Map()` 4.6x, `new Promise`
  2.7x (D3155/D3156 removed the promise-specific part). Interpreter construct/call path and
  builtin constructors, not promises.
- Fuzz seeds 344676 and 208872 (--lazy) fail on main too (dumper; given to pjr).

## Result
