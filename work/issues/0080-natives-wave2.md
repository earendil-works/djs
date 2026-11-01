# 0080: for-of / iteration fast paths, natives wave 2

status: open
agent: -
branch: issue/0080-natives-wave2
depends-on: 0063
touches: crates/djs-core/src/interp/ (an unobservable for-of fast path for unmodified Array/String/TypedArray iteration, D1351-style checks; isolated), builtins/ (@native tags), crates/djs-bgen/ (try/finally and constructors in @native), crates/djs-core/src/builtins/gen/ (regenerated), crates/djs-core/src/heap/ (a dense truncate entry point for pop/length shrink: small, additive, coordinate with heap), crates/djs-bench/

## Goal
0063's follow-ups: after native builtins, for-of over arrays is mostly iterator-result
allocation (10–15x qjs), `new TA(array)` walks the full iterator protocol (devflow: ~31 ms
per 10k elements in debug), and the next hot builtins are still bytecode.

## Acceptance criteria
- [ ] for-of over an Array (and String, TypedArray) whose iteration is unmodified
      (`Array.prototype[@@iterator]` and `%ArrayIteratorPrototype%.next` are the
      originals; checked like D1351) iterates without allocating iterator results.
      Observable behavior identical (the spec's iteration semantics hold because the check
      makes it unobservable), including mutation during iteration and holes.
- [ ] The same fast path for the iterable-consuming builtins (`new TA(array)`, Array.from,
      spread, Promise.all over arrays, Map/Set constructors) where it applies.
- [ ] bgen: try/finally and constructors in @native builtins.
- [ ] Natives wave 2, chosen by profiling: JSON.stringify's slow path, Object.keys/
      values/entries, Array.from, Map/Set iteration and forEach, and a faster `pop`
      (a dense truncate entry point in heap).
- [ ] Numbers (on a quiet machine): the engine track, the pi bench, for-of
      microbenchmarks, and the size per native (scripts/native-sizes.sh).
- [ ] Both backends identical (filtered runs while iterating, one full comparison before
      merging).

## Log

## Follow-ups

## Result
