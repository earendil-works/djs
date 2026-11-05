# TypeScript

djs runs TypeScript directly. A host turns it on per sandbox (`typescript: true` in [the
JavaScript interface](js-api.md#creating-a-sandbox), `--ts` on [the command
line](cli.md)), and the compiler then reads TypeScript syntax in the same single pass in
which it reads JavaScript. There is no type checker and no separate transpile step: the
types are erased while parsing, and the few TypeScript constructs that do something at
run time compile straight to bytecode.

> **In short.** Types are skipped by the parser, so TypeScript costs nothing at run time
> and almost nothing at compile time. Enums, namespaces and parameter properties run as
> `tsc` emits them; decorators follow the standard, with TypeScript's legacy
> `experimentalDecorators` and `emitDecoratorMetadata` behind options. Modules are not
> supported, in TypeScript as in JavaScript. `djs strip-types` shows the erasure.

## Types are erased while parsing

Everything that only describes types is skipped: annotations, type parameters and
arguments, `as`, `satisfies` and the `!` assertion, `<T>expr` casts, `interface` and
`type` declarations, `declare` statements, overload signatures, `abstract` members,
`this` parameters and optional markers. Types are not checked: a script whose types are
wrong runs as its JavaScript would.

```ts live
interface Hit { name: string; stars: number }
type Ranked = Hit & { rank: number };
function rank<T extends Hit>(hits: T[]): (T & { rank: number })[] {
  return hits
    .toSorted((a, b) => b.stars - a.stars)
    .map((h, i) => ({ ...h, rank: i + 1 }));
}
const hits: Hit[] = [{ name: "djs", stars: 120 }, { name: "quickjs", stars: 9000 }];
const top = rank(hits)[0] satisfies Ranked;
const wrong: number = "not checked" as any;
console.log(top.name, top.rank!, wrong);
```
<!-- gen:run -->
```text
quickjs 1 not checked
```
<!-- /gen -->

The rules for telling a type from an expression (a generic call `f<T>(x)` from two
comparisons, an arrow function with a return type from a parenthesized expression, where
a line break ends a contextual keyword) are TypeScript's, and so are the syntax errors,
with QuickJS's wording. Semantics follow `tsc` with `isolatedModules` and
`useDefineForClassFields`: class fields are defined, not assigned.

> **Trade-off.** Erasing in the parser over transpiling first. Because djs compiles
> [without a syntax tree](goals.md#no-syntax-tree), it can skip types as it meets them,
> and a TypeScript script compiles about as fast as the same script without its types.
> The cost: no type information reaches the compiler, so there is no type checking and
> nothing that needs types (`const enum` across files, `emitDecoratorMetadata` for
> types it cannot see) works beyond what one file says.

## What runs

A few TypeScript constructs have run-time meaning, and djs compiles them as `tsc` emits
them:

- **Enums** become objects, numeric members with a reverse mapping. A `const enum`'s
  members are folded into the code that uses them; its object exists too.
- **Namespaces** become objects filled by a function, merged across declarations; a
  namespace with only types is erased.
- **Parameter properties** (`constructor(public x: number)`) define and assign the
  field, in `tsc`'s order relative to the other fields.

```ts live
enum Level { Debug, Info, Warn = "warn" }
const enum Limit { Max = 3 }
namespace Log {
  export const lines: string[] = [];
  export function add(level: Level, text: string) {
    if (lines.length < Limit.Max) lines.push(`${Level[level] ?? level}: ${text}`);
  }
}
class Job {
  constructor(public readonly id: number, private attempts = 0) {}
  retry() { return ++this.attempts; }
}
Log.add(Level.Info, "started");
Log.add(Level.Warn, "slow");
const job = new Job(7);
job.retry();
console.log(Log.lines.join("; "), JSON.stringify(job));
```
<!-- gen:run -->
```text
Info: started; warn: slow {"id":7,"attempts":1}
```
<!-- /gen -->

## Decorators

Decorators follow the standard (the TC39 proposal TypeScript 5 implements): class,
method, accessor, field and `accessor` decorators, with their context objects,
`addInitializer` and `Symbol.metadata`. They work in JavaScript too.

```ts live
function logged(fn: Function, ctx: ClassMethodDecoratorContext) {
  return function (this: unknown, ...args: unknown[]) {
    console.log(`${String(ctx.name)}(${args.join(", ")})`);
    return fn.apply(this, args);
  };
}
class Calc {
  @logged add(a: number, b: number) { return a + b; }
}
console.log(new Calc().add(40, 2));
```
<!-- gen:run -->
```text
add(40, 2)
42
```
<!-- /gen -->

**Legacy decorators.** Code written for TypeScript's `experimentalDecorators` expects the
older semantics: decorators get the prototype, the key and a property descriptor, and
parameters can be decorated. The `experimentalDecorators` option (`--experimental-decorators`)
selects them, and `emitDecoratorMetadata` (`--emit-decorator-metadata`, which implies
the first) also calls `Reflect.metadata` with `design:type`, `design:paramtypes` and
`design:returntype`, as far as the annotations in the file say. As in TypeScript, the
script (or a library it includes) provides `Reflect.metadata`.

```ts
function log(target: object, key: string, desc: PropertyDescriptor) {
  console.log("decorating", key);
}
function param(target: object, key: string, index: number) {
  console.log("parameter", index, "of", key);
}
class Greeter {
  @log greet(@param name: string) { return "hi " + name; }
}
// parameter 0 of greet
// decorating greet
```

## What is not supported

- **Modules.** `import` and `export` are syntax errors, also `import type` and
  `export type`: a script is one piece of code, and its host gives it its functions as
  globals or parameters ([bindings](js-api.md#host-functions)).
- **JSX and `.tsx`.** `<T>expr` is always a cast.
- **Type checking**, and everything that needs types from another file.
- In the [slim build](language.md#the-slim-build), decorators.

## `djs strip-types`

`djs strip-types file.ts` prints the JavaScript the compiler sees: the file with every
erased range replaced by spaces, line breaks kept, so positions in error messages and
stack traces are the same in both. The output is byte for byte what the ts-blank-space
tool produces for the same input, which the tests check against its output for
TypeScript's own sources. It accepts module syntax, which the compiler itself does not,
so it can be compared on any TypeScript file. Enums, namespaces and parameter
properties are kept as written: they are not erasable.

```text
$ cat a.ts
class A { x = 1 } const a: A = new A(); console.log(a.x)
$ djs strip-types a.ts
class A { x = 1 } const a    = new A(); console.log(a.x)
```

## Where to look

- [`crates/djs-core/src/compiler/`](../../crates/djs-core/src/compiler/): the parser
  and its type skipper.
- [`builtins/decorators.ts`](../../builtins/decorators.ts): the decorator runtime.
- [`tests/strip-types/`](../../tests/strip-types/): the strip-types cases with
  ts-blank-space's output.
