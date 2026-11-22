# 0289: Builtins charge fuel for the work they do (GitHub issue #3)

status: done
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
- [x] Every operation listed above charges fuel that grows with its input or output
      (elements visited, code units scanned or copied, keys enumerated, bytes allocated),
      at rates in line with the existing ones (D1561: 1 per 16 code units; dense loops 1
      per 4 elements), on every path: @fast entries, @rust leaves (including the tier-0
      loop's leaf call, D3551), leaf intrinsics, and the hot loop's own arms.
- [x] The charge is made where the work is done (shared primitives: copy, search,
      compare, key enumeration, array/string allocation, memmove), not per builtin, so a
      new builtin built on them is metered without thought. Preferred design: a work
      counter in the heap that primitives add to and the interpreter drains into
      `Exec::fuel` after each leaf/fast call; decide and record it.
- [x] Fuel stays deterministic: independent of fuel slicing, GC timing, images/restores,
      native vs wasm (D1710, D1740, D1711). The existing determinism and suspend-every
      stress modes stay green.
- [x] A test (gate leg if needed) checks input scaling for a broad table of builtins (the
      survey's list at least, plus the reporter's cases), so a new flat builtin fails it.
      The reporter's failing tests pass.
- [x] Wall-clock sanity: re-run the 1M-fuel slice measurements above; each is within a
      small factor (say 20x) of the tight loop's time. Record the numbers.
- [x] Performance: the fast paths stay fast (bench/slope numbers for the touched ops;
      wasm size within budget). Record in a decision.
- [x] ARCH.md's "@fast entries cost a flat 2" and guide limits.md updated to the truth.
- [x] Decision recorded (fuel model change; COMPAT_VERSION if image meaning changes).

## Notes
- Large leaf calls still run to completion (they are not interruptible); the overshoot
  becomes FUEL_DEBT (D1740). That is acceptable; bounded by the heap cap.
- If the scope proves much larger (e.g. instructions with O(n) operands need their own
  design), land the builtin/leaf part and list the rest under Follow-ups.

## Log
- Baseline survey (scratch issue3/fuel_survey.rs at main 389d83ca): 40+ operations flat
  (work/scratch/0289/survey-before.txt).
- Design (D3580): `Heap::work`, a counter in sixteenths of a unit, charged by the heap's
  primitives (allocation past 64 B, string search/compare/scan/hash, number parsing,
  element/key loops and moves, values made into arrays). Taken into `Exec::fuel` at
  safepoints; the fraction carries over in `FUEL_DEBT` (COMPAT_VERSION 14). GC,
  suspension records, host work and declined fast paths charge nothing.
- First version drained after every heap call in the tier-0 loop: correct, but +10-18%
  instructions on every loop in V8 (slope.py). Draining at safepoints instead (the loop
  leaves at its next safepoint when a unit is owed, through `collect`) costs ~1%. To keep
  D1740's "a budget stops at the same instruction for any slicing", a run out of fuel
  goes on to a wanted safepoint before it stops (the loop with its fuel biased from the
  run loop, `hot_bias`; a check in the loop itself cost 5-8%).
- Determinism bugs found on the way, each by an existing test: floor-per-drain made fuel
  depend on drain granularity (hot vs non-hot) → carry fractions; a declined fast
  attempt that charged and then stopped the run looped forever at small slices
  (generated_builtins_collect_at_the_cap) → declined attempts charge nothing and a
  declined instruction completes before the run stops; debug-only double compares in the
  loop charged; host OOM with owed work never stopped; huge credits lost the fraction in
  f64 (props_tests) → debt clamped to 2^48.
- Small work is free (scan ≤64 units, loop ≤16 elements, compare/hash ≤256 units):
  without it a fraction per call made the loop leave for a safepoint every few calls
  (+10% on `includes`).
- Size: +1,163 B gz shipped, +617 slim after slimming (charges out of line, string
  compares shared through `Heap::str_cmp`/`view_cmp`, counter as the heap's last field).
  The owner raised the allowance by 1,024 B (D3581).
- Guide: limits.md (fuel section rewritten), builtins.md (fuel per transition),
  determinism.md (fuel bullets). ARCH "Fuel" and the GC/fuel rules. TESTING.md budget.
- Numbers: work/scratch/0289/{survey-after.txt,wall-after.txt,slope-default.txt,
  slope-ops.txt}; summary in D3580.

## Follow-ups
- A Map/Set or atom lookup with a long string key whose hash is cached, and the content
  comparison of the equal key it finds, are not charged (the hash is charged when
  computed). Bounded by the hash charge of making the key, but a loop over one long key
  object is a flat lookup per call.
- Under GC stress a run stops at the end of its fuel even with work owed (every
  safepoint is wanted there); stress modes compare transcripts, not stops, so nothing
  checks it, but fuel stops under stress are not slice-exact for owed work.
- The wall-clock table was measured natively; a wasm run of `slice_wall_clock`'s cases
  would confirm the factors there.
- `split('')` of a long string is the slowest per unit of fuel (20x the tight loop):
  50 ns per one-character piece is slow in itself (substring + unit string per char).

## Result
Builtins and leaf operations now charge fuel in proportion to their work, at the heap's
primitives (`crates/djs-core/src/heap/work.rs`), on every path (`@fast` entries, `@rust`
leaves in and out of the tier-0 loop, leaf intrinsics, the loop's own string compares).
One slice of a million units now takes at most ~43 ms natively (split('') of 1M; tight
loop 2 ms) where it took up to 128 s. Fuel stays slice-, GC- and loop-independent
(D1740 tests, `slices::fuel_independent_of_collections`, the hot-loop invisibility
tests). `crates/djs-prelude/tests/builtin_fuel.rs` fails for a new flat builtin; its
ignored `slice_wall_clock` prints the wall-clock table. Decisions D3580, D3581.
