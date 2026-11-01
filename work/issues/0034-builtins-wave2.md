# 0034: Core builtins wave 2: Array and String in full, 0031 leftovers

status: in-progress
agent: infra
branch: issue/0034-builtins-wave2
depends-on: 0031
touches: builtins/ (array.ts, string.ts, object.ts, error.ts, …), builtins/gen/, crates/djs-core/src/builtins/, crates/djs-core/src/bytecode/intrinsics.rs (regenerated), crates/djs-bgen/, crates/djs-core/src/compiler/ (only: an option to drop source text for the prelude), crates/djs-prelude/, ratchets

## Goal
Finish the everyday builtins that codemode scripts and test262 lean on most, and close
the 0031 leftovers.

## Acceptance criteria
- [ ] Array.prototype complete (ES2025): `at concat copyWithin entries every fill filter
      find findIndex findLast findLastIndex flat flatMap forEach includes indexOf join
      keys lastIndexOf map pop push reduce reduceRight reverse shift slice some sort
      (stable; a leaf fast path for primitives without a comparator, per ARCH) splice
      toLocaleString toReversed toSorted toSpliced toString unshift values with`, plus
      `Array.from` (iterables need 0029/0030; land the array-like path first), `Array.of`
      and `Symbol.species` handling. Use 0028's iterator objects for keys/values/entries
      once 0030 lands (coordinate: heap owns the ArrayIterator runtime).
- [ ] String.prototype, except num's six (0033: match matchAll replace replaceAll search
      split): `at charAt charCodeAt codePointAt concat endsWith includes indexOf
      isWellFormed lastIndexOf localeCompare (simple code-unit compare, no Intl;
      document it) normalize (NFC/NFD/NFKC/NFKD need tables: check their size and
      decide; a follow-up is fine) padEnd padStart repeat slice startsWith substring
      toLowerCase toUpperCase (0011's case tables, including final sigma)
      toLocaleLowerCase/UpperCase toWellFormed trim trimEnd trimStart [Symbol.iterator]`
      (the iterator after 0030), plus `String.fromCharCode`, `fromCodePoint` and `raw`.
      Leaf intrinsics on the heap string representation (one-byte/two-byte, ropes:
      avoid flattening where cheap).
- [ ] 0031 leftovers once iteration is on main: `Object.fromEntries`, `Object.groupBy`,
      `AggregateError` (and `Map.groupBy` only if Map exists by then).
- [ ] `instanceof` honours `Symbol.hasInstance` (the slow path through
      `builtins/function.ts`'s InstanceofOperator), and the throwing `caller`/`arguments`
      function's name is "".
- [ ] Prelude source text: an option to compile the prelude without keeping its source
      (prelude functions print as native code anyway), so the base drops ~46 KB. This
      also speeds up the e2e suspension mode, which copies the whole heap image (24 s →
      47 s in debug after 0031). Check the gate time before and after, and report it.
- [ ] difftest + test262 numbers before and after, per directory (Array, String,
      Object), and the ratchets updated.

## Notes
- Land in parts (for example: prelude source + Array; then String; then the leftovers).
- num (0033) does JSON/RegExp and the six regex String methods in parallel, also in
  builtins/. Keep to separate files and regenerate the intrinsic table on rebase.

## Log

### Baseline (main 7ebc262)
- difftest 74 / 171. test262 active: built-ins 3,617, language 4,027, harness 43.
  Array 809 / 2,680, String 156 / 1,040, Object 1,838 / 3,197, AggregateError 0 / 19.
- Gate 1:43 wall; e2e test 49.7 s (debug).

### Part 1: prelude source, Array
- Prelude source (D1340): privileged compiles keep no source; base 174 → 127 KB,
  `new_vm` 2.3 → 1.4 µs. e2e only 49.7 → 48.1 s: the cost is the verified
  `Vm::from_bytes` per suspension stop (1.1 ms in debug, 49 µs in release) and gc-stress
  copying all cells, both per cell. Reported; the coordinator set djs-core to
  opt-level 1 in dev (D010): e2e 3.4 s, gate ~46 s.
- Array (D1341): all of Array.prototype except keys/values/entries/[Symbol.iterator]
  (need 0029's iterator intrinsics), Array.of, Array.from (array-likes, arrays and
  strings; other iterables throw "not supported yet" until %GetIterator), get
  Array[Symbol.species], Array.prototype[Symbol.unscopables]. Leaves: SortPrimitives,
  CompareStrings, StringCodePoints, ArraySpliceDense (heap: `array_splice_dense`,
  additive in heap/object.rs). Element fast paths in %Get / %CreateDataPropertyOrThrow.
  bgen: a conditional of two integer literals types as a literal.
- behavior.js compares every Array method with qjs on the subject matrix (results only
  for the methods where qjs probes with HasProperty, see D1341): no mismatches.
- difftest builtins/array-methods.js (with stress).
- Results: difftest 79 / 172. test262 Array 2,429 / 2,680 (from 809); built-ins 5,264,
  language 4,033, harness 43. Remaining Array failures: propertyHelper's for-in (133),
  RegExp/Date/JSON globals (61), iterators (keys/values/entries, Array.from iterables),
  and `arr.length = true` (the interpreter's length assignment does not convert).
- Perf: correct but builtin-language speed. 200k-element `sort` with a comparator 1.8 s
  (qjs 0.04 s); the interpreter's call path dominates (3.4M plain calls 0.6 s vs qjs
  0.06 s). shift on a dense 3,000-element queue went from 1.1 s to instant.

## Follow-ups
- Interpreter: `arr.length = v` with a non-Number v (ToNumber/ToUint32 in the set path),
  3 test262 tests.
- Interpreter call speed (3.4M calls 0.6 s vs qjs 0.06 s) bounds every builtin-language
  loop; candidates for `@native` once the Rust backend exists: sort with comparator,
  map/filter/forEach.

## Result
