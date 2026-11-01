# 0080: for-of / iteration fast paths, natives wave 2

status: done
agent: infra
branch: issue/0080-natives-wave2
depends-on: 0063
touches: crates/djs-core/src/interp/ (an unobservable for-of fast path for unmodified Array/String/TypedArray iteration, D1351-style checks; isolated), builtins/ (@native tags), crates/djs-bgen/ (try/finally and constructors in @native), crates/djs-core/src/builtins/gen/ (regenerated), crates/djs-core/src/heap/ (a dense truncate entry point for pop/length shrink: small, additive, coordinate with heap), crates/djs-bench/

## Goal
0063's follow-ups: after native builtins, for-of over arrays is mostly iterator-result
allocation (10–15x qjs), `new TA(array)` walks the full iterator protocol (devflow: ~31 ms
per 10k elements in debug), and the next hot builtins are still bytecode.

## Acceptance criteria
- [x] for-of over an Array (and String, TypedArray) whose iteration is unmodified
      (`Array.prototype[@@iterator]` and `%ArrayIteratorPrototype%.next` are the
      originals; checked like D1351) iterates without allocating iterator results.
      Observable behavior identical (the spec's iteration semantics hold because the check
      makes it unobservable), including mutation during iteration and holes.
- [x] The same fast path for the iterable-consuming builtins (`new TA(array)`, Array.from,
      spread, Promise.all over arrays, Map/Set constructors) where it applies.
- [x] bgen: try/finally and constructors in @native builtins.
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

- Slice 1 merged (main 491db8e): iter_next, D1800, constant intrinsic ids in native.rs.
- D1801 dense-array leaves (interp/run/dense.rs) for indexOf/lastIndexOf/includes/
  slice/join/push(v)/pop, Object.values/entries, typed arrays from dense arrays. Heap:
  `array_pop_dense`/`array_can_pop_dense` (additive, elements section; cleared with heap
  via the coordinator). `fast_step` takes `dup2` and `iter_next` over array values.
- D1802 iterable consumers (`%IteratorFastReady/Value`), Map/Set Iterators in
  `iter_next`, direct CollectionSet in the Map/Set constructors with the original adder;
  Map/Set forEach `@native`.
- D1803 bgen try/finally (copies on every way out) and `@native` constructors; Set/Map
  constructors, GetPrototypeFromConstructor, AddEntriesFromIterable native.
- JSON.stringify's slow path: not done. Profile (0081 F0, measure.py): json 0.53x and
  jsonh 0.57x qjs instructions already (fast path for plain data); the slow path only
  runs with a replacer, toJSON or getters, and a subtree fast path would need an indent
  parameter in json/stringify.rs (outside touches). Left as a follow-up.
- Load was 17-47 the whole session, so no `bench.sh --record`. Instruction counts
  (scripts/perf/measure.py, load-independent, native release, base = main before 0080 →
  this branch, Minstr): forof 25,126 → 4,725 (qjs 1,781; peak 21.6 → 10.4 MB), indexof
  1,052 → 124 (qjs 75), pushpop 6,778 → 4,951 (qjs 1,265), push-pop 4,095 → 3,144,
  arrays 14,692 → 12,855, pi-like 13,815 → 12,839, jsonh 941 → 739, higher-order
  unchanged. Per element (perelem.py): for-of 5,433 → 738 (qjs 386), indexOf 490 → 45
  (28), includes 653 → 48 (33), join 2,424 → 390 (469), slice 945 → 371 (20).
- wasm (node, median ms, base.wasm vs new.wasm, loaded machine so ±20%): for-of 200k×20
  1666 → 163; string for-of 1820 → 410; TA for-of 9040 → 1256; entries+destructuring
  4538 → 1409; `[a,b]=[i,1]` 1652 → 498; new Float64Array(xs) 4982 → 268; new Set(xs)
  945 → 366; new Map(pairs) 807 → 433; Set forEach + for-of 1317 → 216; pop loop 1608 →
  744; indexOf+includes 3588 → 308; slice 836 → 361; join 487 → 67; Object.values+entries
  248 → 38; Map for-of 1130 → 343; 100k `new Set([1,2,3])` 1375 → 250. pi bench
  (tool-heavy, image reused) 16.5 → 12.9 ms (quickjs-wasi 11.1 / 9.2 in the same runs).
- Size (wasm, names kept, scratch sizes.sh): code 829,416 → 849,875 (+20.5 KB):
  generated natives 50,521 → 61,238 (new: SetConstructor 2,909, AddEntriesFromIterable
  2,703, MapConstructor 1,114, GetPrototypeFromConstructor 816, Map/Set forEach 959/928;
  indexOf/includes/join/slice/pop +190..+280 each for the leaf calls), iter.rs 1,972,
  dense.rs 4,265. Shipped (stripped) module 1,411,761 → 1,435,717.
- Slice 2 merged (main 03c79ff); `backends_agree` sampling (main ffc1811): the default
  test samples 5 modes (~2 s), the exhaustive one is `#[ignore]`d (TESTING.md).
- fill: `ArrayFillDense` (plain stores via `set_element_fast`, then the general loop from
  where it stopped). All O(n) dense leaves charge one fuel unit per four elements
  (lastIndexOf's charge counts the elements compared: charging the range first made a
  sparse 2^32 array run out of fuel). copyWithin "detached" tests: their 10,000-element
  `array.fill` was the cost (debug runner: 1.35 s now incl. startup).
- decodeURI A2.5_T1: the time is in the harness loop's builtin calls, not the URI code:
  per iteration decodeURI ~5.5k instructions and String.fromCharCode(H, L) ~12k (rest
  array, CoerceNumbers frame, leaf), in a Script whose `var`s are globals. The cheap fix
  is `@fast` entries for fromCharCode and the URI functions once 0085 lands (debug
  runner: 3.9 s for the test today).

## Follow-ups
- After 0085 lands: move the dense.rs paths (push/pop/indexOf/lastIndexOf/includes/slice/
  join) to `@fast` entries (no native frame; push+pop is still ~4x qjs because of the two
  frames). Agreed with the coordinator: mine.
- slice's copy goes through `set_element_fast` per element (prototype walk each time,
  371 instr/elem vs 20): a heap bulk append for a fresh array would fix it.
- JSON.stringify slow path: a subtree fast path (needs an indent parameter in
  json/stringify.rs).
- Record the engine track / for-of benches on a quiet machine.
- `@fast` entries for String.fromCharCode and decodeURI/encodeURI(Component) (with the
  dense.rs move above).
- (coordinator, from 0082) 3 test262 tests fail on typed arrays over resizable buffers
  that shrink during `fill`/`freeze`/`subarray` (length-tracking views).

## Result

## Result (coordinator)
Merged in four steps (491db8e, 03c79ff, ffc1811, fd5a6d1); see the Log. The remaining items moved to new issues: the @fast move and the fromCharCode/URI entries -> 0091 (after 0085); native collect-and-retry, resizable TA, the JSON.stringify slow path -> 0090.
