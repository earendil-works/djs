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

## Follow-ups

## Result
