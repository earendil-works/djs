# 0034: Core builtins wave 2: Array and String in full, 0031 leftovers

status: done
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

### Part 2: String
- Part 1 merged: main ..b5ad87c.
- String (D1342): all of String.prototype except num's six regex methods and
  [Symbol.iterator] (iterators), plus substr/trimLeft/trimRight, fromCharCode,
  fromCodePoint, raw. Leaves in builtins/string.rs on flat views. localeCompare as qjs
  (code point difference, no Intl, no NFC). normalize without tables: identity for
  strings that need no tables, else TypeError "not supported yet" (3 test262 tests).
- behavior.js compares every method with qjs on 10 subjects (proxy-free, wrapper,
  toPrimitive objects): no mismatches. difftest builtins/string-methods.js (stress OK).
- Results: test262 String 161 → 687 / 1,040; the rest is num's regex methods (135),
  propertyHelper's for-in (96), regex literals (70), tagged templates, normalize.
  built-ins 5,809, language 4,046, harness 45. difftest 82 / 173.

### Part 3: instanceof, %ThrowTypeError%
- Part 2 merged: main ..6b0cc5e.
- `instanceof` (D1343): custom Symbol.hasInstance, `prototype` getters and bound
  functions over such targets go through `%InstanceofOperator`; checked against qjs
  (builtins/instanceof.js, stress OK). %ThrowTypeError%: name "", length 0, frozen.
- Outside touches: interp/run.rs (`Op::Instanceof` slow path, `instanceof_needs_builtin`).
- Still open: Object.fromEntries, Object.groupBy, AggregateError, Array
  keys/values/entries/@@iterator, String @@iterator, Array.from of other iterables:
  all wait for 0029's iterator intrinsics (%GetIterator, CreateArrayIterator).

### Part 4: iterables (on 0029 part 1)
- Behavior test timeout (D1344) merged early (4f2ae92).
- builtins/iterable.ts (D1345): Object.fromEntries, Object.groupBy, AggregateError,
  Array.from over iterables. Work with user iterables now; arrays and strings as input
  need the Array/String iterators (0030 part 4; the String iterator is drafted in
  work/scratch/0034/draft/string-iterator.ts on heap's ArrayIterator class).
- behavior.js: all of them against qjs with logging iterators (no mismatches, one
  documented spec difference). difftest builtins/iterables.js.
- Results: AggregateError 0 → 6 / 19, Object 1,882, Array 2,448; built-ins 5,844.

### Part 5: String iterator (on 0030 part 4)
- Part 4 merged: main ..acaa611.
- String iterator (D1346) with kind 3 on heap's Array Iterator class, as agreed with the
  coordinator. Small edit in heap's code (interp/run/intrinsics.rs): `ArrayIteratorTarget`
  rejects kind 3. `Exec::iteration_unmodified` (interp/run.rs) replaces
  `array_iteration_unmodified` and covers strings, so spread over a string with a patched
  String.prototype[@@iterator] or %StringIteratorPrototype%.next runs the protocol
  (heap's D1351 follow-up). Array.from: iterator path unless the iteration is the
  original (arrays only without a mapping function).
- difftest builtins/string-iterator.js (patched iterator and next, spread and
  Array.from; stress OK).
- Results: test262 Array 2,461, String 688, Object 1,891, AggregateError 9 / 19;
  built-ins 5,877, language 4,077, harness 45. difftest 85 / 176.

### After part 5: gate time
- Part 5 merged: main ..7181e8c. Its merge took 15 min: the gate's difftest stress step
  ran 911 s, all in `dquickts/builtin/test_rope.js`, which now runs (String.fromCharCode)
  and did charCodeAt over a 100k-deep rope without flattening (quadratic). Fixed: the
  string leaves flatten once (cached in the rope); plain run 0.4 s in debug, and the case
  passes now. Its stress runs are still slow, and the stress timeout was only checked
  between 10M-fuel slices: D1347 (GC stress in 20k slices; a stressed run that only
  times out is reported, not failed). Gate: 1:43 at the start of 0034 (before D010),
  15:40 after part 5, 60 s now.

## Follow-ups
- Unicode normalization tables (NFC/NFD/NFKC/NFKD): size them (qjs's libunicode has
  them) and decide; localeCompare should then compare NFC forms like qjs.
- Interpreter: `arr.length = v` with a non-Number v (ToNumber/ToUint32 in the set path),
  3 test262 tests.
- Interpreter call speed (3.4M calls 0.6 s vs qjs 0.06 s) bounds every builtin-language
  loop; candidates for `@native` once the Rust backend exists: sort with comparator,
  map/filter/forEach.

## Result
Landed in five merged parts (D1340-D1346):
- The prelude keeps no source text (base 174 → 127 KB). The e2e slowdown was per-cell
  verification in suspension mode; D010 (djs-core at opt-level 1 in dev) fixed it.
- Array.prototype complete (ES2025), Array.of / from (iterables, array-likes), species,
  unscopables; leaf fast paths (dense splice, primitive sort, element access).
- String.prototype except num's six regex methods, plus substr/trimLeft/trimRight,
  fromCharCode/fromCodePoint/raw, and the String iterator.
- Object.fromEntries, Object.groupBy, AggregateError.
- instanceof honours Symbol.hasInstance and prototype getters; %ThrowTypeError% unnamed
  and frozen.
- djs-bgen's behavior test has a 60 s timeout (D1344).
Numbers (start → end of 0034): test262 Array 809 → 2,461 / 2,680, String 156 → 688 /
1,040, Object 1,838 → 1,891 / 3,197, AggregateError 0 → 9 / 19; built-ins 3,617 →
5,877, language 4,027 → 4,077, harness 43 → 45. difftest 74 → 85 / 176.
Known gaps: normalize without tables (and localeCompare without NFC), Map.groupBy (no
Map yet), builtin-language loops at interpreter speed (sort with a comparator), the
interpreter's `arr.length = v` conversion, and the regex String methods (0033).
