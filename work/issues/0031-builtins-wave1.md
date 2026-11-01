# 0031: Core builtins wave 1: Object, Function, Error, Boolean, Symbol, Number, Math, globals

status: done
agent: infra
branch: issue/0031-builtins-wave1
depends-on: 0028
touches: builtins/ (new *.ts files; intrinsics.d.ts additions), builtins/gen/, crates/djs-core/src/builtins/ (new: leaf intrinsic implementations, one module per area) or wherever 0028 put intrinsics (follow its pattern), crates/djs-core/src/bytecode/intrinsics.rs (the table), crates/djs-bgen/ (installation forms only), crates/djs-core/src/interp/realm.rs (additive: intrinsic objects), tests/corpus/expected-failures.txt + tests/test262/expected-failures.txt (ratchet updates)

## Goal
The difftest clusters (0016) show missing globals as the biggest non-compiler blocker
(`Object` alone in 11 cases), and the test262 harness needs much of this too. This wave
builds the foundation and the pattern for all later builtins.

**The pattern** (ARCH "The leaf rule"): a builtin that may run user code (almost all of
them, through ToString/ToNumber/ToPrimitive of arguments or `this`, getters and the like)
is written in the builtin language. It does the coercions and checks and then calls
**leaf intrinsics** in Rust for the actual work. Pure-leaf builtins (for example
`Math.sqrt` after ToNumber) are still a thin builtin-language wrapper around a leaf
intrinsic. Document the pattern in builtins/README.md with one worked example.

## Acceptance criteria
- [ ] bgen installation forms (0017 follow-up; coordinate with 0029, which adds symbol keys
      and `%Name%` targets): constructors (`@constructor Object`, with separate
      call/construct behavior: `new.target` available to the builtin), static methods,
      getters/setters (`@getter X.prototype.y`), value properties with attributes
      (`Math.PI`, `Number.MAX_SAFE_INTEGER`), and globals (`globalThis.parseInt`).
      Keep it minimal, and log the format.
- [ ] Object: the constructor, `keys values entries assign freeze isFrozen seal isSealed
      preventExtensions isExtensible create defineProperty defineProperties
      getOwnPropertyDescriptor(s) getOwnPropertyNames getOwnPropertySymbols
      getPrototypeOf setPrototypeOf fromEntries hasOwn is groupBy`.
      Object.prototype: `toString` (with the class tag and Symbol.toStringTag),
      `valueOf hasOwnProperty isPrototypeOf propertyIsEnumerable toLocaleString`.
      `__proto__` as an accessor is Annex B: skip it, unless the harness needs it.
- [ ] Function.prototype: `call apply bind toString` (D009 source slices; native
      functions print `function name() {\n    [native code]\n}` like qjs) and
      `[Symbol.hasInstance]`. The `Function` constructor itself is out of scope (it
      would need eval-like compilation: a non-goal for now).
- [ ] Error, TypeError, RangeError, SyntaxError, ReferenceError, EvalError, URIError,
      AggregateError: constructors (`message`, `cause` option, `stack` per 0028), and
      `Error.prototype.toString`. The interpreter's internal errors use the same
      prototypes.
- [ ] Boolean and Symbol (the constructor without `new`, `description`, `toString`,
      `valueOf`, `Symbol.for`/`keyFor` (needs a registry; atoms are never collected, so
      a heap table is fine), and the well-known symbols as statics). Wrapper objects for
      ToObject of primitives (a 0028 follow-up) as far as these need them.
- [ ] Number: the constructor, statics (`isInteger isSafeInteger isFinite isNaN
      parseFloat parseInt` and the constants), and the prototype (`toString(radix)
      toFixed toExponential toPrecision valueOf toLocaleString`) via `crate::num`.
      Globals `parseInt parseFloat isNaN isFinite NaN Infinity undefined globalThis`.
- [ ] Math: every function and constant (libm, D1150). `Math.random` gets its seed from
      the host or heap state (ARCH "Nondeterminism"): an embedder-settable seed in the
      realm, an xorshift128+ or similar algorithm, deterministic across snapshots.
- [ ] Tests: difftest cases for each area (qjs is the oracle, so add cases to
      tests/corpus/builtins/), e2e cases in the three modes, and the ratchets updated.
      Report the difftest and test262 numbers before and after (per directory for
      built-ins/Object, Function, Error, Boolean, Symbol, Number, Math).
- [ ] Startup cost of `Vm::new` with the bigger prelude, recorded. If it goes well beyond
      ~300 µs, say so (the base snapshot then becomes urgent).

## Notes
- This is big. Land it in parts (for example: bgen forms + Object + Function first), each
  merged with tests, and log what remains.
- 0029 (iteration, lexer agent) and 0030 (iteration runtime, heap agent) are running in
  parallel and also add intrinsics. The intrinsic table is ordered by declaration, so
  rebases will conflict there. The drift test prints the right table: regenerate it,
  never hand-merge it.

## Notes from 0007 (test262 blockers)
- Tests stopping first at a missing global: `Object` 2,597, `Array` 1,036, `String` 743,
  `Number` 492, `TypeError` 475, `ReferenceError` 179. `propertyHelper.js` needs, at load
  time: `Array.isArray`, `Object.defineProperty`, `getOwnPropertyDescriptor`,
  `getOwnPropertyNames`, `Function.prototype.call/bind`, `Array.prototype.join/push`,
  `Object.prototype.hasOwnProperty/propertyIsEnumerable`, `Math.pow`, and `String()`.
  Prioritize those first (with the Error constructors, for `assert.throws`). Adding
  `Array.isArray`, `push` and the `String`/`Array` constructors basics here is in scope,
  even though Array/String in full are wave 2.
- `$262.evalScript` runs with `this` undefined; fix the wrapper with
  `Function.prototype.call` once it exists.

## Log

### Baseline (main 2312924, before any change)
- difftest (djs): 51 pass / 165 cases (103 expected failures, 11 skipped).
- test262 active: 2,961 / 20,124 (built-ins 140 / 9,049, language 2,802, harness 19).
  Per dir (active pass): Object 1/3197, Function 1/242, Error 0/50, NativeErrors 0/82,
  AggregateError 0/19, Boolean 0/45, Symbol 0 active (77 later), Number 6/324,
  Math 0/284, Array 129/2680, String 0/1040, global 2/21, NaN 1/4, Infinity 0/4,
  undefined 0/4, isNaN/isFinite 0/8, parseInt 0/54, parseFloat 0/53.

### Part 1: bgen forms, Error constructors, the propertyHelper set
- bgen (D1310): `@constructor` (+ `@extends`, `new.target`), `@getter`/`@setter`,
  `@value` (+ `@attributes`), repeatable `@builtin`, bare/globalThis targets, rest
  params in builtins. Globals are installed first. Checker tests + golden fixture.
- Rust: new module `crates/djs-core/src/builtins/` (object.rs: descriptors, keys,
  attributes; function.rs: bound functions, toString); dispatch and the install
  intrinsics in interp/run/intrinsics.rs; realm: EvalError/URIError/AggregateError
  prototypes, `Intrinsic::prototype_of`/`by_name` (for `%RealmIntrinsic`).
- Builtins: error.ts (Error + 6 NativeErrors, Error.prototype.toString), object.ts
  (Object ctor, defineProperty, getOwnPropertyDescriptor, getOwnPropertyNames,
  hasOwnProperty, propertyIsEnumerable, ToPropertyDescriptor), function.ts (call, apply,
  bind, toString, a throwing `Function` ctor so Function.prototype links work),
  array.ts (Array ctor, isArray, push), math.ts (Math namespace, pow), string.ts
  (String(); `new String` throws "not supported yet"), conversion.ts (ToPropertyKey,
  GetPrototypeFromConstructor). behavior.js compares all of them with qjs natives
  (stand-ins for the new intrinsics): no mismatches.
- **Outside touches (minimal, announced to the coordinator):** heap/object.rs
  (`ObjectClass::BoundFunction = 7`, `remove_own_property_unchecked`), heap/cells.rs
  (`OBJ_BUILTIN_CTOR`), js/src/layout.generated.ts (regenerated), interp/run.rs (bound
  function unwrap in call/construct/instance_of, `is_constructor` for builtins,
  `stack_trace_from` that can skip the prelude frames at the top), and
  crates/djs-test262/src/djs.rs (`$262.evalScript` uses `.call($262.global)`).
- **Pre-existing GC bug fixed** (on main too): `construct` wrote the callee it had read
  before `call`, whose safepoint may move it, into the frame's NEW_TARGET slot. Any
  `new F()` whose body reached a GC with gc-stress panicked in the arena (e.g.
  `function F(){ [1].map(x => x) }; new F()`). It now re-reads the callee.
- Tests: tests/corpus/builtins/{error-constructors,function-call-bind,
  object-descriptors}.js (two are ratcheted until Object.getPrototypeOf / Symbol land),
  tests/data/e2e/builtins_wave1.js (three modes).
- Intrinsic table: regenerated with work/scratch/0031/regen-table.py (mirrors the
  drift test, which stays the authority; needed because the crate does not compile
  while the table lacks a name used by `id()`).
- Results: difftest 55 / 168 pass (102 expected failures). test262 active 5,108 /
  20,124 (25.4%): built-ins 1,765, language 3,305, harness 38. Per dir: Object 825,
  Function 87, Error 19, NativeErrors 7, Array 692, String 89, Math 20, Number 8,
  Boolean 2, global 10.
- Startup: `Vm::new` with the prelude 210 µs native release (was 92 µs per D1281),
  without 3 µs; heap 74 KB (was 36 KB). Below the ~300 µs flag, but it doubled with
  this part; the base snapshot (0032) matters.
- Remaining stack-position difference: `new X(...)` stacks point at `new`, qjs at the
  `(` (compiler line table; follow-up for the compiler).

### Part 2: Object rest, wrappers, Boolean, Symbol, Number, Math, globals
- Main had 0032 (base snapshot): rebased; the Math.random seed is per VM via
  `VmOptions::random_seed`, applied in `Vm::new` and `after_copy` (D1314).
- Object: getPrototypeOf setPrototypeOf create defineProperties keys values entries
  assign freeze seal preventExtensions isFrozen isSealed isExtensible
  getOwnPropertySymbols getOwnPropertyDescriptors hasOwn is; Object.prototype toString
  (class tag + @@toStringTag) toLocaleString valueOf isPrototypeOf. Not yet:
  `fromEntries` and `groupBy` (need iteration, 0029/0030).
- Wrappers + primitive lookup (D1313), Boolean, Symbol (constructor, for/keyFor,
  description, toString, valueOf, @@toPrimitive, @@toStringTag, the 13 well-known
  symbols), Number (constructor, statics, constants, toString/toFixed/toExponential/
  toPrecision/valueOf/toLocaleString via crate::num), globals parseInt parseFloat
  isNaN isFinite (Number.parseInt === parseInt), Math (all functions and constants,
  libm; random D1314), Function.prototype[@@hasInstance], %ThrowTypeError% for
  Function.prototype.caller/arguments, `new String` wrappers and
  String.prototype.toString/valueOf.
- bgen: symbol keys `[Symbol.x]`, `@attributes` on `@builtin` (+ `none`), accessor
  halves on one function (D1315). New leaf module crates/djs-core/src/builtins/number.rs.
- behavior.js compares every part-2 builtin with qjs (0 mismatches), except one
  deliberate difference: qjs's Object.defineProperties reads all descriptor objects
  before converting any, the spec (and djs) converts each right after reading it.
- Outside touches: heap/object.rs (wrapper classes 8-11), interp/run.rs (`lookup` of
  primitives through their prototypes), vm/mod.rs (`VmOptions::random_seed`,
  `after_copy`, `set_random_seed`), vm/tests.rs (seed test; one runtime_errors
  expectation: `{} + 1` no longer throws now that Object.prototype.toString exists).
- Tests: difftest builtins/{object-statics,number-math,boolean-symbol}.js, e2e
  builtins_wave1_part2.js; all six builtins cases pass on djs (with stress).
- libm precision: `Math.exp(1) !== Math.E` with libm (qjs uses the system libm, which
  gives E exactly); left out of the difftest case.
- Results after part 2: difftest 74 / 171 pass (86 expected failures, from 103 at the
  start). test262 active 7,687 / 20,124 (38.2%): built-ins 3,617, language 4,027,
  harness 43. Per dir: Object 1,838/3,197 (from 1), Function 150/242 (from 1), Error
  28/50 (0), NativeErrors 38/82 (0), Boolean 35/45 (0), Number 270/324 (6), Math
  162/284 (0), Array 809/2,680 (129), String 156/1,040 (0), parseInt 51/54,
  parseFloat 50/53, isNaN 6/8, isFinite 6/8, global 11/21. Symbol's tests are all in
  the `later` group. Most remaining failures in these dirs are propertyHelper.js's
  for-in (1,058) and for-in/of in tests (276), then Date/RegExp/JSON globals.
- Startup: `Vm::new` compiling the prelude 600 µs native release; heap after collect
  174 KB (17 KB bare). With the base snapshot (0032) `djs_prelude::new_vm` takes
  2.3 µs. The base grew from 36 KB; 46 KB of it is the prelude's source text, kept
  for Function.prototype.toString (D009) although prelude functions print as native:
  a compile option to drop it would shrink the base by a quarter (follow-up).
- The e2e suite takes ~47 s in debug (was ~24 s at part 1): the suspension mode copies
  the whole heap at every fuel-1 stop, so its cost grows with the base size.

## Follow-ups
- Compiler: stack position of `new F()` (qjs: the call's `(`), see Log.
- Setting an array index past a non-writable `length` says "object is not extensible"
  (interp set_property), qjs says "'length' is read-only".
- Object.fromEntries, Object.groupBy and AggregateError need IterableToList / GetIterator
  (0029/0030); write them on top of those.
- `instanceof` in the interpreter ignores a user `Symbol.hasInstance`; builtins/
  function.ts has InstanceofOperator, which the op could call as a slow path.
- Privileged compiles could drop the source text (46 KB of the 174 KB base).
- e2e suspension mode cost grows with the base size (see Log); maybe suspend every N > 1
  or restore from a smaller delta.
- %ThrowTypeError% is named `get caller` (should be "", non-extensible, frozen
  name/length).
- Symbol.prototype[@@toPrimitive], Function.prototype[@@hasInstance] attributes are
  right; Math functions use libm (Math.exp(1) differs from qjs's system libm in the
  last bit).

## Result
