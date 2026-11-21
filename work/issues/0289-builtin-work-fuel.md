# 0289: Builtins charge fuel for the work they do (GitHub issue #3)

status: in-progress
agent: pidle
branch: issue/0289-builtin-work-fuel
depends-on: -
touches: crates/djs-core/src/interp/run/fastpath.rs, crates/djs-core/src/builtins/fast.rs, crates/djs-core/src/builtins/{string,dense,object,collection,...}.rs (leaf primitives), crates/djs-core/src/heap (a work counter), crates/djs-core/src/interp/run/{intrinsics,hot,dense}.rs, a new fuel-scaling test, ARCH.md "Fuel" line, docs/guide/limits.md
guide: docs/guide/limits.md, docs/guide/builtins.md, docs/guide/determinism.md

## Goal
Fuel must bound the CPU work of a run (ARCH.md "Fuel", guide limits.md: "every native
operation must charge for its work, or it becomes a way to stall the host"). It does not:
https://github.com/earendil-works/djs/issues/3 (reported against 88c5fd9b, reproduced on
main c34b2a59). Many builtins charge a flat amount whatever their input size.

Coordinator's survey (work/scratch/issue3/fuel_survey.rs, the reporter's test
work/scratch/issue3/builtin_fuel.rs; both are djs-prelude integration tests, copy them to
crates/djs-prelude/tests/ to run): fuel at n = 30 / 3,000 / 30,000 is FLAT for
- strings: split (any separator), indexOf, lastIndexOf, includes, startsWith, endsWith,
  toUpperCase/toLowerCase, repeat, padStart/padEnd, trim, concat, replace/replaceAll
  (string pattern), normalize, localeCompare, atob/btoa, parseInt/Number of long strings,
  `===` / `<` of long strings (instructions);
- arrays: join (and toString / String(arr)), indexOf, includes, slice (also via
  Array.prototype.slice.call), splice, shift, unshift, `length = 0`, `[...a]`;
- objects: Object.keys, getOwnPropertyNames, Reflect.ownKeys, `{...o}`, Object.freeze,
  Object.isFrozen;
- new Set(arr), new Uint8Array(n).

Wall clock of one 1M-fuel slice (release, native): `for(;;) i++` 2.2 ms; `a.join('-')` of
30k elements 7.9 s; `s.indexOf('!')` over 1M chars 6.0 s; `Object.keys(o)` of 30k keys
12.8 s; `new Set(a)` of 30k elements 128 s. A one-line script stretches a slice ~57,000x.

Causes found so far: @fast entries (D1850) get `&mut Heap` and no fuel, and charge a flat
`FAST_FUEL = 2` (fastpath.rs); `array_index_of` drops `dense::search`'s scanned count;
@rust generated leaves (`own_call`, the in-loop leaf call) charge flat `FAST_FUEL` and the
leaf intrinsics they call (string search, OwnPropertyKeys, ...) charge nothing; VM
instructions comparing strings charge per instruction.

## Acceptance criteria
- [ ] Every operation listed above charges fuel that grows with its input or output
      (elements visited, code units scanned or copied, keys enumerated, bytes allocated),
      at rates in line with the existing ones (D1561: 1 per 16 code units; dense loops 1
      per 4 elements), on every path: @fast entries, @rust leaves (including the tier-0
      loop's leaf call, D3551), leaf intrinsics, and the hot loop's own arms.
- [ ] The charge is made where the work is done (shared primitives: copy, search,
      compare, key enumeration, array/string allocation, memmove), not per builtin, so a
      new builtin built on them is metered without thought. Preferred design: a work
      counter in the heap that primitives add to and the interpreter drains into
      `Exec::fuel` after each leaf/fast call; decide and record it.
- [ ] Fuel stays deterministic: independent of fuel slicing, GC timing, images/restores,
      native vs wasm (D1710, D1740, D1711). The existing determinism and suspend-every
      stress modes stay green.
- [ ] A test (gate leg if needed) checks input scaling for a broad table of builtins (the
      survey's list at least, plus the reporter's cases), so a new flat builtin fails it.
      The reporter's failing tests pass.
- [ ] Wall-clock sanity: re-run the 1M-fuel slice measurements above; each is within a
      small factor (say 20x) of the tight loop's time. Record the numbers.
- [ ] Performance: the fast paths stay fast (bench/slope numbers for the touched ops;
      wasm size within budget). Record in a decision.
- [ ] ARCH.md's "@fast entries cost a flat 2" and guide limits.md updated to the truth.
- [ ] Decision recorded (fuel model change; COMPAT_VERSION if image meaning changes).

## Notes
- Large leaf calls still run to completion (they are not interruptible); the overshoot
  becomes FUEL_DEBT (D1740). That is acceptable; bounded by the heap cap.
- If the scope proves much larger (e.g. instructions with O(n) operands need their own
  design), land the builtin/leaf part and list the rest under Follow-ups.

## Log

## Follow-ups

## Result
