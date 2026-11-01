# 0031: Core builtins wave 1: Object, Function, Error, Boolean, Symbol, Number, Math, globals

status: in-progress
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

## Follow-ups

## Result
