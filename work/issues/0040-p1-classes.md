# 0040: P1-7 Classes

status: open
agent: -
branch: issue/0040-p1-classes
depends-on: 0036
touches: crates/djs-core/src/compiler/, crates/djs-core/src/bytecode/ (additive), crates/djs-core/src/interp/ (class-specific ops: small, additive, logged; heap is in interp for 0037 part 3), builtins/ (helpers if needed), docs/compiler.md, ratchets

## Goal
docs/compiler.md section 10, row P1-7: class declarations and expressions, methods,
getters/setters, static members, computed keys, public fields (instance and static, with
`useDefineForClassFields` semantics: [[DefineOwnProperty]]), private names (fields,
methods, accessors, static privates, `#x in obj` brand checks), static blocks, `super`
(property access and calls, in methods, in object literal methods too: home objects,
the 0027 follow-up), derived constructors (`this` TDZ until `super()`, the return
override rules), `extends` with null and with builtins (Error, Array, Map, Promise:
subclassing must produce the right exotic objects via new.target), class name
binding, and the class TDZ. test262 has ~3,300 tests behind classes (0007 clusters,
plus 786 class early-error negatives), and classes also block ~74 Set/other tests.

## Acceptance criteria
- [ ] All of the above, per spec evaluation order (computed keys and field
      initializers), and all class early errors (duplicate constructors, `super()`
      outside derived constructors, private names undeclared or duplicate, `#constructor`,
      `delete this.#x`, …) with qjs messages.
- [ ] Private names: a storage design (per-class private name keys that never leak:
      symbols-like hidden atoms, or a separate private slot table in objects) recorded as
      a decision. Brand checks for private methods.
- [ ] Interp needs are sent to the coordinator early as one list (as in 0036). Small
      additive ops are fine to do yourself if heap isn't touching the same code: ask.
- [ ] Also from the 0036 follow-ups: `new` on a non-constructor says "not a
      constructor" (qjs wording); `(fn) = function () {}` must not name the function.
- [ ] e2e tests checked against qjs, a disassembly snapshot, the test262 compile sweep
      before/after, and the ratchets updated. Report the test262 active numbers.

## Notes
- The next compiler issue after this is async functions and generators (compiler side
  on top of heap's 0037 part 3 suspendable frames). Keep the function-kind plumbing
  ready for `async`/generator method forms in classes.

## Log

## Follow-ups

## Result
