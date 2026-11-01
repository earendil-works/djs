# 0030: P1-5 runtime: for-in ops, array iterators, symbol-keyed installation

status: in-progress
agent: heap
branch: issue/0030-p1-iteration-runtime
depends-on: 0028 (done), 0029 (for the .d.ts declarations and the iteration ops)
touches: crates/djs-core/src/interp/, crates/djs-core/src/vm/, crates/djs-core/src/heap/ (additive)

## Goal
The interpreter side of 0029 (the lexer agent's list, verbatim below). After this,
for-of/for-in/spread over iterables work end to end. Un-ignore 0029's
`end_to_end_iteration_rt` test.

## The list (from the lexer agent, 0029)
for-of compiles to plain ops + `call_intrinsic` of builtin-language intrinsics
(`%GetIterator`, `%IteratorClose`, `%IteratorCloseThrow` in builtins/iter.ts), not the
`get_iterator`/`iterator_close` ops. The step is `dup2; swap; call 0; iter_check_object;
dup; get_field done; jump_if_true; get_field value`. for-in uses
`for_in_start`/`for_in_next`.

1. Ops:
   - `for_in_start` (obj -> it): snapshot of the enumerable string keys in spec order
     (own: integer keys ascending, then strings in insertion order; then the proto chain,
     skipping seen keys (shadowing) and non-enumerable ones; no symbols). null/undefined
     give an empty iterator; other primitives use ToObject.
   - `for_in_next` (it -> it key done): skip keys deleted since the snapshot (no longer
     present on the object or its chain); at the end key = undefined, done = true.
   - `iter_check_object` (v -> v): TypeError "iterator must return an object" if v is not
     an object.
2. Leaf intrinsics (declared by 0029 in builtins/intrinsics.d.ts):
   - `CreateArrayIterator(o: Obj, kind: i32): Obj`: proto %ArrayIteratorPrototype%,
     internal state target = o, index = 0, kind 0/1/2 = keys/values/entries.
   - `ArrayIteratorTarget(it: Value): Value`: TypeError "Array Iterator object expected"
     unless `it` is an array iterator. Returns the target (undefined once exhausted).
   - `ArrayIteratorIndex(it: Obj): f64`, `ArrayIteratorSetIndex(it: Obj, i: f64): void`,
     `ArrayIteratorFinish(it: Obj): void` (target := undefined), `ArrayIteratorKind(it:
     Obj): i32`.
   - `CreateIterResultObject(value: Value, done: boolean): Obj`.
   - `CreateArrayFromPair(a: Value, b: Value): Obj`.
   - `WellKnownSymbol(name)`, used by bgen output for `[Symbol.iterator]` keys (check
     0029's .d.ts for the exact signature).
3. Installation: `%InstallMethod`'s key may be a symbol. Target paths may name realm
   intrinsic objects (`"%IteratorPrototype%"`, `"%ArrayIteratorPrototype%"`), which the
   realm must create (%ArrayIteratorPrototype%.[[Prototype]] = %IteratorPrototype%). One
   function may be installed twice (`Array.prototype.values` ===
   `Array.prototype[Symbol.iterator]`).

## Also (left over from 0028)
- `define_method` / `define_getter` / `define_setter` and their `_elem` forms, and
  `array_spread`: 0027's ignored test `compiler::tests::end_to_end_objects_0028` still
  fails on these (verified on main 468f874). Do these first, since they are on main
  already, then un-ignore that test.
- Widen `%InstallMethod`'s key type in `builtins/intrinsics.d.ts` to allow symbols (0029
  may already do this; check).
- The `return f()` stack position (qjs reports the `return` keyword): this is a compiler
  question. Note it for the lexer agent, don't change the compiler.

## Acceptance criteria
- [ ] Everything in the list, with qjs messages.
- [ ] 0029's `end_to_end_iteration_rt` un-ignored and passing; e2e cases in the three
      modes.

## Log
- 2026-11-01: 0028 leftovers first:
  - `define_method` / `define_getter` / `define_setter` and their `_elem` forms:
    DefinePropertyOrThrow, enumerable and configurable (methods also writable); a getter
    alone keeps an existing setter. Computed keys go through D1280's key conversion.
  - `array_spread`: arrays (holes read as undefined) and strings (by code points) here;
    other iterables wait for 0029's `%GetIterator`. A primitive gives qjs's
    `value is not iterable`.
  - `compiler::tests::end_to_end_objects_0028` un-ignored (14 cases pass). New e2e
    cases `object_literals`, `spreads`, in the three modes.
  - The home object (`define_method` "also sets the home object") is only needed for
    `super` (P1-7 classes); not stored yet (Follow-ups).
- Part 1 merged: 23a66f5..2c11128 (main 2c11128). Status stays in-progress for the P1-5
  list. The difftest ratchet reported 1 stale entry (a case that passes now); left for
  the coordinator to `--update-ratchet`.

- Part 2 (merged early at the coordinator's request, to shrink the conflict window with
  0031):
  - `for_in_start` / `for_in_next`: a key snapshot in an internal `ValueArray`
    `[object, index, keys...]` (D007: never visible to JS). Spec order, shadowing by
    keys already seen (even non-enumerable ones), symbols skipped, and keys deleted
    since the snapshot skipped. Checked against qjs (`1,2,b,a,inherited,gone,` and the
    deletion variant).
  - `iter_check_object` with qjs's message.
  - Realm intrinsics `%IteratorPrototype%` and `%ArrayIteratorPrototype%`, and
    `ObjectClass::ArrayIterator = 6` (slots target / index / kind).
  - `%InstallMethod`: the key may be a symbol (name `[desc]`), a function installed twice
    keeps its first name, and a first path segment `%Name%` names a realm object.
  - The array iterator leaf intrinsics are drafted in
    `work/scratch/0030/iter_intrinsics.rs`; they need 0029's ids, so they wait for it.

- Part 4 (after 0029 part 1):
  - The leaves `CreateArrayIterator`, `ArrayIteratorTarget`, `ArrayIteratorIndex`,
    `ArrayIteratorSetIndex`, `ArrayIteratorFinish`, `ArrayIteratorKind`,
    `CreateIterResultObject` and `CreateArrayFromPair`. Any kind is accepted (infra's
    String iterator will be kind 3).
  - Spread over general iterables (D1351): `builtins/spread.ts` `%ArraySpread` runs the
    iterator protocol. The Rust path keeps strings and arrays with original iteration
    and no holes. Additive edit to 0029's `iter.ts`: `@intrinsic` on
    ArrayPrototypeValues and ArrayIteratorPrototypeNext, for the identity check.
  - qjs messages for symbol keys in TypeErrors (`describe_key`).
  - New e2e case `iteration_runtime` (iterators, holes through prototype getters,
    custom and patched iterators, the next/done/value order, call and new spread, 2000
    values across safepoints, errors). Passes in all three modes.
  - Fixed `vm::tests::to_primitive_through_a_host_valueof`: it compared a raw `o`
    across image round trips, which the bigger prelude now moves. It re-reads the
    global now.
  - test262: 46 more pass (spread in `new`/calls/arrays, Array keys/values/entries).
    The ratchet was updated.
  - `end_to_end_iteration_rt` is not on main yet; it comes with 0029 part 2.

- Home object for methods (`super`), with classes (P1-7).
- `return f()` stack positions: qjs reports the `return` keyword; ours, the `(`. A compiler
  question for the lexer agent (from 0028).
- `%ArrayIteratorPrototype%[@@toStringTag]` = "Array Iterator" is missing (iter.ts,
  0029; test262 `ArrayIteratorPrototype/Symbol.toStringTag`).
- Once a String iterator exists, `array_spread`'s string path needs D1351's identity
  check (String.prototype[@@iterator] and %StringIteratorPrototype%.next).
- djs has no global `JSON` yet (noticed while testing; probably 0031's).

## Result
