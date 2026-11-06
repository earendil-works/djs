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

## Follow-ups

## Result
