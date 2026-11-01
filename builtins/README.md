# Builtins

Builtins that may run user code are written here in a strict subset of TypeScript
(ARCH.md "The builtin language"). `djs-bgen` checks them and generates
`gen/prelude.js` (JS with `%Name(...)` intrinsic calls, compiled by djs in privileged
mode) and `gen/resume-points.txt` (which calls may run user code). Both are checked in:

```sh
scripts/bgen.sh            # regenerate after editing *.ts or intrinsics.d.ts
scripts/bgen.sh --check    # is gen/ up to date? (the gate runs this)
scripts/bgen.sh --report   # print the resume-point report
```

`intrinsics.d.ts` declares the types and the intrinsics, the Rust operations a builtin
may call. `tsconfig.json` lets editors (or `tsc -p builtins`) type-check the sources;
bgen does not need node and is stricter than tsc.

## The subset

- Files are scripts with top-level `function` declarations only, each with explicit
  parameter and return types. Optional parameters (`x?: Value`) are allowed at the end,
  and a rest parameter `...args: Value[]` last (an array; its local type is `Obj`).
  A builtin that uses `this` declares it as the first parameter (`this: Value`).
- Types: `Value`, `Obj`, `Fn`, `Str`, `Sym`, `boolean`, `i32`, `u32`, `f64`, `void`,
  `never`. `Fn` widens to `Obj`, and everything widens to `Value`. Narrowing is
  explicit, with a test and a cast intrinsic: `if (IsObject(v)) { const o: Obj = AsObj(v); }`.
- Statements: `const`/`let` with a type and an initializer, assignment (`=`, `+=`, ...)
  and `++`/`--` (statements only, on locals), `if`/`else`, `while`, `do`/`while`, `for`
  with an index, `break`, `continue`, `return`, `throw`, `try`/`catch`/`finally`, blocks,
  and calls. No shadowing of other locals.
- Expressions: locals, `this`, literals (numbers, strings, booleans, `undefined`,
  `null`), unary `! - ~`, binary operators except `==`, `!=`, `**` and `??`, `?:`, and
  calls of intrinsics or other builtins by name. User functions are called through
  `Call(f, thisArg, ...args)` and constructors through `Construct(f, ...args)`.
- Conditions are booleans: there is no implicit truthiness.
- Numbers: `+ - *` on one integer type; `/` and `%` on f64 only; bitwise operators on
  i32 only (`>>>` gives u32); no mixing of numeric types, except that i32 and u32 widen
  to f64 where a value is assigned or passed; integer literals adapt if they fit.
  Integer overflow is a bug in the builtin (D1171).
- Not in the subset: property access (`o.x`, `o[k]`: use `Get`, `Set`, ...), object and
  array literals, `new`, closures and nested functions, `arguments`, `eval`,
  generators, `async`, classes, `switch`, labels, `typeof`/`in`/`instanceof`,
  templates, spread. If something is awkward without them, add an intrinsic.

## The pattern: coercions in the builtin language, work in leaf intrinsics

Almost every builtin may run user code: ToPrimitive of an argument calls `valueOf`,
`Get` calls getters. Hand-written Rust never calls back into JS (ARCH.md "The leaf
rule"), so a builtin is written here: it does the conversions and checks in spec order
and then calls **leaf intrinsics** (Rust in `crates/djs-core/src/builtins/` and
`interp/run/intrinsics.rs`) that do the actual work on values that can no longer run
user code. Worked example, `Object.defineProperty` (builtins/object.ts):

```ts
/**
 * Object.defineProperty ( O, P, Attributes ), 20.1.2.4.
 * @builtin Object.defineProperty
 */
function ObjectDefineProperty(o: Value, p: Value, attributes: Value): Value {
  if (!IsObject(o)) {
    throw NewTypeError("not an object");
  }
  const key: Value = ToPropertyKey(p); // may call toString/valueOf/@@toPrimitive
  const desc: Obj = ToPropertyDescriptor(attributes); // HasProperty/Get: proxies, getters
  DefinePropertyOrThrow(AsObj(o), key, desc); // ends in the leaf %DefineOwnPropertyFromRecord
  return o;
}
```

`ToPropertyDescriptor` reads the six fields with `HasProperty`/`Get` (each a resume
point) into a descriptor record (a prototype-less object holding only the present
fields), and the leaf `DefineOwnPropertyFromRecord(o, key, desc)` turns that into a
`PropertyDescriptor` and calls the heap's `[[DefineOwnProperty]]`. Even a "pure" builtin
like `Math.pow` is a thin wrapper: `NumberPow(ToNumber(base), ToNumber(exponent))`.
When a new builtin needs an operation no intrinsic offers, add a leaf to
`intrinsics.d.ts` (and its Rust), keep it free of user code, and let the builtin do the
rest. Messages follow QuickJS; `crates/djs-bgen/tests/behavior.js` checks the builtins
against qjs's natives, including proxy trap order.

## Tags

On builtin functions (JSDoc):

- `@builtin Array.prototype.map`: install the function as a method of the object at
  that path (writable, configurable, not enumerable; `name` is the last component).
  Repeatable (one function in several places). A bare name or `globalThis.x` installs
  on the global object; a well-known symbol key is written
  `Symbol.prototype[Symbol.toPrimitive]`. `@attributes` (see `@value`) overrides the
  method attributes.
- `@constructor Error` (+ `@extends Error` for subclasses): install a global
  constructor with its prototype object (the realm's intrinsic one if there is one).
  `new.target` is available in its body; it is `undefined` for a plain call. Other
  builtins are not constructors and have no `prototype`.
- `@getter X.prototype.y` / `@setter X.prototype.y`: one half of an accessor
  (configurable, not enumerable).
- `@value Math.PI` (+ `@attributes writable enumerable configurable` or `none`, default none): a
  data property whose value the function (no parameters) returns once at install time.
  Namespace objects are values: `@value globalThis.Math`.
- `@length N`: the `length` property (default: the parameters before the first
  optional or rest one).
- `@intrinsic`: register it so that privileged code and the interpreter can call it
  as `%Name(...)` (for example `%ToPrimitive`).
- `@native`: also compile it with the Rust backend (0063): a resumable Rust state
  machine in `crates/djs-core/src/builtins/gen/natives.rs`, which `scripts/bgen.sh`
  writes too. A reentrant builtin it calls must be `@intrinsic`, unless it is a small
  internal helper that bgen inlines (D1632); leaf builtins it calls become Rust
  functions. Not yet: `try`/`finally`, `@constructor`. Each one adds wasm size
  (`scripts/native-sizes.sh`): tag a builtin only when a benchmark shows it is hot.
  `--backend bytecode` (djs run, djs-difftest, djs-test262) runs the bytecode instead;
  both must behave the same.

On intrinsics in `intrinsics.d.ts`, exactly one of:

- `@leaf`: never runs user code.
- `@reentrant`: may call one user function (a getter, a setter, the callee), whose
  result completes the operation. Multi-step logic around user code belongs in a
  builtin, not in an intrinsic.

Calls to reentrant intrinsics and to reentrant builtins are resume points; bgen finds
them and lists them in `gen/resume-points.txt`.
