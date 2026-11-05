# The language

djs runs modern JavaScript: the language of the current ECMAScript specification, with
its builtins, in strict mode. This page says what a script can rely on: how its code is
compiled, what is left out and why, which builtins exist, and the two variations a host
can choose, the slim build and frozen builtins.

> **In short.** Code is always strict. A host runs a script as the body of an async
> function, so `await` and `return` work at the top level; Scripts in the specification's
> sense exist for the REPL and for tests. What is left out (`with`, direct `eval`,
> modules, `Intl`, timers, I/O) is what codemode scripts don't need and what would cost
> size or safety; the rest of the standard library is there, matching QuickJS's behavior
> and messages.

## Always strict

Every piece of code djs compiles is strict mode code, with or without a `"use strict"`
directive. There is no sloppy mode to fall back to: assigning to an undeclared name is a
`ReferenceError`, `this` in a plain function call is `undefined`, octal literals and
`with` are syntax errors, and writing to a read-only property throws.

```js live
function whoAmI() { return this; }
console.log(typeof whoAmI());
try {
  undeclared = 1;
} catch (e) {
  console.log(String(e));
}
try {
  Object.freeze([1, 2]).push(3);
} catch (e) {
  console.log(String(e));
}
```
<!-- gen:run -->
```text
undefined
ReferenceError: 'undeclared' is not defined
TypeError: object is not extensible
```
<!-- /gen -->

One exception keeps old code working: a function made by the `Function` constructor
without a `"use strict"` directive gets the sloppy binding of `this` (the global object
when called plainly), and nothing else of sloppy mode.

> **Trade-off.** Strict only, over both modes. Sloppy mode doubles the semantics of
> scopes, `arguments`, `this` and assignments, and every one of those paths would need
> the same care for suspension, dumps and determinism as the strict one. Scripts written
> today, and every script a model writes, are strict-compatible. The cost: an old
> snippet that relies on sloppy semantics (an implicit global, `arguments.callee`) fails
> with an error instead of running.

## Codemode bodies and Scripts

A host runs a script as a **codemode body**: the body of an async function. That gives
it top-level `await` and `return`, and the parameters the host passes (`params` in the
[JavaScript interface](js-api.md#running-code)) are its parameters. The value the body
returns, or its promise's value, is the result of the run; an exception it does not
catch is the run's error.

```js live
const sizes = await Promise.all(["a", "bb", "ccc"].map(async (s) => s.length));
if (sizes.length === 0) return "nothing";
const total = sizes.reduce((a, b) => a + b);
console.log(sizes, total);
return total;
```
<!-- gen:run -->
```text
[1,2,3] 6
```
<!-- /gen -->

A body's declarations are local to it: two runs in the same sandbox share only what the
first one put on `globalThis`.

The second kind of code is a **Script** in the specification's sense: top-level `var`
and function declarations become properties of the global object, `let`, `const` and
`class` go into a global lexical scope that later Scripts see, and the Script has a
completion value. The REPL runs every input as a Script ([the REPL](cli.md#the-repl)),
the test suites use them, and a continuation dump is one ([dumps](dumps.md)). A Script
has no top-level `await`.

## What is left out, and why

| Left out | Why |
| --- | --- |
| Sloppy mode, `with` | See above: one set of semantics. `with` is a syntax error. |
| Direct `eval` | A direct `eval` can see and change the caller's local variables, so every function that contains one would need its scopes kept by name. `eval(x)` is always an indirect eval: it compiles `x` as a global Script. The `Function` constructors work. |
| ES modules | A codemode script is one piece of code. `import` and `export` declarations are syntax errors, and `import()` compiles and returns a rejected promise. Hosts give scripts their functions as globals or parameters instead ([bindings](js-api.md#host-functions)). |
| `Intl` | Locale data is megabytes. `toLocaleString` and friends use a fixed, locale-independent form. |
| `Temporal` | Not needed by the scripts djs runs; `Date` is there. |
| `SharedArrayBuffer`, `Atomics`, workers | A sandbox is one thread. |
| Timers, `fetch`, file access, `TextEncoder`, `structuredClone` | The engine does no I/O and has no event loop of its own: everything outside the script is a host function the host chose to give it. |
| A JIT | wasm and a small module size come first; the interpreter is fast enough for scripts that mostly move data. |

A few host-like globals are there because scripts use them and they need no outside
world: `atob`, `btoa`, `queueMicrotask`, `DOMException`, and `console`, which is a host
object whose methods the host provides.

```js live
console.log(typeof Intl, typeof setTimeout, typeof atob, typeof queueMicrotask);
try {
  await import("./tools.js");
} catch (e) {
  console.log(String(e));
}
console.log(eval("typeof globalThis"), (1234.5).toLocaleString());
```
<!-- gen:run -->
```text
undefined undefined function function
ReferenceError: could not load module './tools.js'
object 1234.5
```
<!-- /gen -->

## The builtins

Everything else of the standard library is there:

- the fundamentals: `Object`, `Function`, `Symbol`, `Error` and its subclasses, `Proxy`,
  `Reflect`, `globalThis`;
- numbers and text: `Number`, `Math`, `BigInt`, `String` with all its methods,
  `RegExp` with named groups, lookbehind, the `d`, `s`, `u` and `v` flags and Unicode
  property escapes;
- collections: `Array` (with the change-by-copy methods such as `toSorted` and `with`),
  typed arrays, `ArrayBuffer`, `DataView`, `Map`, `Set` with its set operations,
  `WeakMap`, `WeakSet`, `WeakRef`, `FinalizationRegistry`, `Object.groupBy` and
  `Map.groupBy`;
- control: `Promise` with its combinators and `withResolvers`, generators, async
  functions and async generators, iterators with the `Iterator` helpers
  (`map`, `filter`, `take`, ...), `Array.fromAsync`;
- data: `JSON`, `Date`, `Uint8Array.fromBase64` and `toBase64`, the URI functions;
- resource management: `using` and `await using` declarations, `DisposableStack` and
  `AsyncDisposableStack`;
- classes with private fields and methods, static blocks, and decorators.

```js live
const words = ["fuel", "frames", "arena", "image", "fuel"];
const byLength = Map.groupBy(new Set(words), (w) => w.length);
console.log([...byLength.keys()].toSorted().join(" "));
console.log(/(?<word>a\w+)/v.exec("the arena").groups.word, "image".at(-1));
const it = Iterator.from(words).filter((w) => w.startsWith("f")).map((w) => w.toUpperCase());
console.log(it.toArray().join(" "));
{
  using _ = { [Symbol.dispose]() { console.log("disposed at the end of the block"); } };
  console.log("in the block");
}
```
<!-- gen:run -->
```text
4 5 6
arena e
FUEL FRAMES FUEL
in the block
disposed at the end of the block
```
<!-- /gen -->

What a builtin does, and the text of the errors it throws, follows QuickJS where the
specification leaves room, because models read those messages and the tests compare
them. [Status in numbers](status.md) gives the conformance figures.

## The slim build

djs also comes in a *slim* build, for hosts that count every kilobyte. It leaves out the
families that codemode scripts don't use and that cost real size: typed arrays with
`ArrayBuffer` and `DataView`, `BigInt`, the `Iterator` helpers, `WeakRef` and
`FinalizationRegistry`, `DisposableStack` and the `using` declarations, and decorators.
A script that reaches for one gets an error that names the build, and feature tests keep
working:

```js
new Uint8Array(4);   // ReferenceError: 'Uint8Array' is not available in this build
typeof BigInt;       // "undefined"
[].values().map;     // undefined: the Iterator helpers are gone
10n;                 // SyntaxError: BigInt literals are not available in this build
```

[Builtins](builtins.md#two-builds-full-and-slim) explains how the build is made.

## Frozen builtins

A host can make a sandbox with **frozen builtins** (`frozenBuiltins` in the [JavaScript
interface](js-api.md#creating-a-sandbox)). The builtin objects (prototypes,
constructors, `Math`, `JSON`, ...) then behave as frozen: writing to them fails with a
`TypeError`, their properties read as non-writable and non-configurable, and
`Object.isFrozen(Array.prototype)` is true. Assigning over an inherited builtin
property, as in `obj.toString = f` or `this.name = "MyError"` in an `Error` subclass,
defines an own property instead of failing, as after a "lockdown" in hardened
JavaScript. `globalThis` itself stays ordinary, so a script can still add globals.

```js
Array.prototype.sum = function () {};   // TypeError: the builtins are frozen
class MyError extends Error {
  constructor(m) { super(m); this.name = "MyError"; }   // fine: an own property
}
```

Frozen builtins protect the host's helpers from a script that patches a prototype, and
they are what lets djs write [continuation dumps](dumps.md): a builtin that cannot change
can be written in a dump by naming it. The mode is chosen when the sandbox is created and
stays with it.

## Where to look

- [`builtins/`](../../builtins/): the builtins, written in a strict subset of
  TypeScript ([builtins](builtins.md)).
- [`crates/djs-core/src/compiler/`](../../crates/djs-core/src/compiler/): the compiler,
  including what it rejects.
- [`tests/test262/expected-failures.txt`](../../tests/test262/expected-failures.txt):
  the test262 tests djs fails, with reasons.
