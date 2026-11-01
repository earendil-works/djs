# 0034: Core builtins wave 2: Array and String in full, 0031 leftovers

status: open
agent: -
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

## Follow-ups

## Result
