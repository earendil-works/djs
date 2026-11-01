# 0051: Remaining builtins: iterator helpers, WeakRef/FinalizationRegistry, disposables, Array.fromAsync, upsert

status: in-progress
agent: heap
branch: issue/0051-remaining-builtins
depends-on: 0046
touches: builtins/ (iterator.ts, weakref.ts, disposable.ts, …), builtins/gen/, crates/djs-core/src/builtins/, crates/djs-core/src/heap/ (weak references in the GC, additive), crates/djs-core/src/interp/realm.rs (additive), crates/djs-core/src/vm/ (the host hook for FinalizationRegistry cleanup scheduling, additive), tests/test262.toml (promotion), ratchets

## Goal
Most of what is left in test262's later group is builtins on top of machinery that now
exists (generators/iterators from 0029/0037/0042, the ephemeron GC from 0038):

## Acceptance criteria
- [ ] Iterator helpers (ES2025): the global `Iterator` constructor (abstract),
      `Iterator.from`, `Iterator.prototype.{map, filter, take, drop, flatMap, reduce,
      toArray, forEach, some, every, find}`, `%IteratorHelperPrototype%`,
      `%WrapForValidIteratorPrototype%`, `[Symbol.toStringTag]` and `constructor`
      accessors, and the `iterator-sequencing` / `Iterator.concat` features if they are
      in the test262 commit. Codemode scripts will use these on arrays' iterators.
- [ ] `Array.fromAsync` (the async-iteration builtin).
- [ ] `Map.prototype.getOrInsert / getOrInsertComputed` (and WeakMap) (upsert).
- [ ] WeakRef and FinalizationRegistry: weak references in the Cheney GC (cleared when the
      target dies; `deref` per spec, with the KeepDuringJob set cleared at the end of each
      job), and FinalizationRegistry cleanup callbacks scheduled as jobs after a
      collection. A host hook decides when (ARCH: the embedder controls
      nondeterminism, so with the default, cleanup runs as a job after a collect). The
      GC must stay deterministic (D1090).
- [ ] DisposableStack, AsyncDisposableStack, SuppressedError, and
      `Symbol.dispose/asyncDispose` (the runtime parts of explicit resource management;
      the `using` syntax is a later compiler issue).
- [ ] Promote everything implemented from later to active (and `__proto__` if 0044's
      accessor covers the tests). Report the numbers before and after the move.
- [ ] behavior.js / difftest / e2e (three modes) and the ratchets.

## Log

- Baseline (test262 @7ab7faf, scratch groups in work/scratch/0051/test262.toml), djs vs
  qjs: iterator (built-ins/Iterator + iterator-* features) 18/655 vs 655; Array.fromAsync
  0/93 vs 93; upsert 23/70 vs 70; WeakRef+FinalizationRegistry 0/80 vs 80;
  explicit-resource-management 59/395 vs 395 (170 are `using` syntax in
  language/statements: stay later); `__proto__` 18/20 vs 20.
- Ids (told the coordinator): ObjectClass 28 IteratorHelper, 31 WeakRef, 32
  FinalizationRegistry, 33 DisposableStack.
- Part 1, iterator helpers (D1510): builtins/iterator.ts, ObjectClass 28, realm
  intrinsics IteratorHelperProto/WrapForValidIteratorProto, leaves in
  interp/run/late.rs. x-iter 449/655; every core and concat test passes. The rest are
  the proposals (chunks/windows, includes, join, zip/zipKeyed: the coordinator wants them
  in this issue) and Iterator.prototype[Symbol.dispose] (with DisposableStack).
  qjs deviations (followed the spec + node): reduce's empty-iterator TypeError closes
  the iterator in qjs; qjs closes an exhausted flatMap inner iterator. Unit test in
  vm/tests.rs (outside `touches`, additive).
- Promoted built-ins/Iterator, iterator-helpers, iterator-sequencing: active
  33,854 → 34,283 tests, 33,450 → 33,879 passing (+429, no new failures). Merged
  (main ca3f95e).
- Part 2, the iterator proposals qjs has (coordinator: in this issue): chunks, windows,
  includes, join, Iterator.zip/zipKeyed (kinds 7-9 of D1510). x-iter 649/655 (the 6
  left are Iterator.prototype[Symbol.dispose], with the disposables). Promoted
  iterator-chunking, iterator-includes, Iterator.prototype.join, joint-iteration:
  active 34,283 → 34,503 tests, 34,099 passing (+220, no new failures).

## Follow-ups

## Result
