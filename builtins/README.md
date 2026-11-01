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
  parameter and return types. Optional parameters (`x?: Value`) are allowed at the end.
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

## Tags

On builtin functions (JSDoc):

- `@builtin Array.prototype.map`: install the function as a method of the object at
  that path (writable, configurable, not enumerable; `name` is the last component).
- `@length N`: the `length` property (default: the parameters before the first
  optional one).
- `@intrinsic`: register it so that privileged code and the interpreter can call it
  as `%Name(...)` (for example `%ToPrimitive`).
- `@native`: also compile it with the Rust backend (once that exists).

On intrinsics in `intrinsics.d.ts`, exactly one of:

- `@leaf`: never runs user code.
- `@reentrant`: may call one user function (a getter, a setter, the callee), whose
  result completes the operation. Multi-step logic around user code belongs in a
  builtin, not in an intrinsic.

Calls to reentrant intrinsics and to reentrant builtins are resume points; bgen finds
them and lists them in `gen/resume-points.txt`.
