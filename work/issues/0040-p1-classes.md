# 0040: P1-7 Classes

status: in-progress
agent: lexer
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

### Parked (coordinator: 0041 async/generators first). No code yet; design notes so far.

State: only reading and qjs probing done. Interp has none of get/put/define_private,
private_in, *_super_*, super_call, check_ctor (they hit the `not supported yet` arm);
`is_constructor` lacks KIND_DERIVED_CTOR. The 0036 "not a constructor" follow-up is
mostly fixed on main already; the only gap is the empty name: qjs says
` is not a constructor` (e.g. `new (() => 1)`, `new` of `class extends null`), djs
filters the empty name out. Scratch: `work/scratch/0040/errs.py` (qjs vs djs
SyntaxError compare, one source per line), `rt.js` (runtime message probes).

qjs runtime messages (probed): `'this' can be initialized only once` (RefErr),
`this is not initialized`, `derived class constructor must return an object or
undefined`, `class constructors must be invoked with 'new'`, `parent class must be
constructor`, `parent prototype must be an object or null`, `private class field '#x'
does not exist`, `invalid brand on object` (private method on wrong object), `'#m' is
read-only` (write to method / getter-only, read of setter-only), `invalid 'in' operand`,
`private class field '#x' already exists`, `private method is already present`,
`unsupported reference to 'super'` (delete super.x, RefErr), `object is not extensible`
(public field on non-extensible this; private fields ignore extensibility and freeze),
inner class binding write: `'A' is read-only`.

Design draft (to confirm, then send the coordinator ONE interp list):
- Private names = private symbols (Symbol cell with a private flag, maybe a method
  flag for the brand message), stored as ordinary own properties but skipped by
  `own_keys` (and freeze/JSON). Each private name is a const var `#x` in the class
  scope; resolve's EnterScope for the class scope creates the symbol
  (`private_symbol atom`), so names exist before heritage and computed keys.
  `o.#x` = `o; name_get #x; get_private_field` (shaped like get_elem: obj sym), so
  lvalues reuse the Elem paths. Private methods/accessors are installed per instance
  (read-only data / accessor private props) by the fields init function before the
  fields: brand check = presence; the symbol's method flag gives qjs's brand message.
  Undeclared `#x`: collect refs per class, check at class end, pass outward.
  Heritage refs use the outer class's names (ref scope = parent of class scope).
- Class scheme: enter class scope (inner const binding, private vars, hidden `.fields`,
  `.kN` computed field keys); heritage `; dup; call_intrinsic %ClassProtoParent`;
  `fclosure ctor` (child slot reserved up front, ctor parsed wherever it appears, src
  span = whole class); `define_class name flags` (parent protoParent ctor -> ctor
  proto); members with new `define_class_method[_elem] kind` (non-enumerable, sets
  home); fields/static blocks parsed into two parked FuncStates (instance init, static
  init) resumed per initializer (`jump` trick to put private-method installs first);
  at class end: `.fields` = init closure or undefined, inner binding init, static
  init called with this = F.
- Ctors: `check_ctor` at entry. Base ctor calls `.fields` (runtime undefined check)
  before param defaults. Derived ctor: `this` is a plain var starting undefined; reads
  go through `check_this`; `super(...)` = `get_super_ctor(active fn) args nt
  super_call argc` (spread: `%ConstructWithArray`), then bind once
  (`'this' can be initialized only once`), then call `.fields`. Return: `check_ctor_return`
  then undefined -> checked this. construct_with must not allocate `this` for derived.
- Home objects: extra trailing closure slot for templates with a HAS_HOME flag, set by
  define_method/define_class_method/`set_home_object`; `special_object HOME_OBJECT`
  into a pseudo var so arrows capture it. `get/put_super_field/elem` with receiver = this.

## Follow-ups

## Result
