# 0080: for-of / iteration fast paths, natives wave 2

status: in-progress
agent: infra
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
- Design (D1800): a new op `iter_next L` (`it next -> r`) replaces `swap; call 0` in the
  for-of step and in array destructuring (compiler/for_in_of.rs, pattern.rs; the
  compiler is outside `touches:` but the change is two emit sites). The fast path keys on
  the identity of the cached `next`, not on @@iterator: that is exactly what the spec
  calls. `%GetIterator`/`%IteratorClose` fast paths in Rust for arrays/strings
  (interp/run/iter.rs). Corpus case language/for-of-fast.js (holes via proto, getters,
  mutation, entries/keys, strings, TAs incl. BigInt and resizable, proxies, replacing
  next mid-loop, `return` on the chain, finished iterators).
- Numbers (load ~25, noisy). Native release, s: for-of 200k ×20 1.06 → 0.13 (qjs 0.05);
  string ×200 0.77 → 0.14; TA 1.17 → 0.16; entries+destructuring 1.33 → 0.27 (qjs 0.17);
  `const [a,b] = [i,1]` ×1M 1.21 → 0.43 (qjs 0.13). wasm (node, median ms): for-of
  1831 → 196, string 1243 → ~0, TA 2426 → 642, entries 4555 → 378, destructuring
  1546 → 382, short for-of 1446 → 627. wasm size 1,396,260 → 1,398,951 (+2.7 KB).
  Scripts: work/scratch/0080/{mb.sh (native), wmb.ts + wsuite.sh (wasm; base.wasm built
  from ee5b575 in the scratch worktree work/scratch/0080/base)}.

## Follow-ups
- (coordinator, from 0082) 3 test262 tests fail on typed arrays over resizable buffers
  that shrink during `fill`/`freeze`/`subarray` (length-tracking views).

## Result
