# Builtins: the TypeScript subset and the leaf rule

`Array.prototype.map`, `JSON.stringify`, `Promise.all`, `Object.defineProperty`: a
JavaScript engine is mostly its standard library, and a codemode script spends most of
its time in it. This page explains how djs writes its builtins, why almost none of them
are hand-written Rust, how the hot ones still run as Rust generated from the same
source, and how the finished builtins reach every VM. [The language](language.md#the-builtins) lists what a script
can use.

> **In short.** Hand-written Rust never calls back into JavaScript: it only provides
> *leaf intrinsics*, operations that cannot run user code. Every builtin that might run
> user code is written in a strict subset of TypeScript, and djs-bgen compiles each one
> exactly one way: to bytecode, which runs once at build time to make the base snapshot;
> to a resumable Rust state machine (`@native`); or, for hot builtins that can only meet
> user code while converting their arguments, to straight-line Rust (`@rust`). A few
> bytecode builtins also have a hand-written Rust entry for everyday arguments (`@fast`).
> Because the builtins live at fixed offsets in every VM, a VM can treat them as frozen
> without changing a byte of them, which is what continuation dumps rely on.

## Why builtins cannot be ordinary Rust

Almost any builtin can run user code. `[1, 2, 3].map(f)` calls `f` three times. But even
a builtin that takes no callback can run user code: `Array.prototype.includes` reads
`length` and the elements through `Get`, which calls getters, and it converts
`fromIndex` with `ToIntegerOrInfinity`, which may call a `valueOf` method. Proxies make
every property read a possible call.

In a conventional engine, such a builtin is a C or Rust function that calls the
interpreter recursively whenever it reaches user code. djs cannot do that. A codemode
script must be stoppable after any instruction, including the instructions of a `map`
callback (the goal to [suspend anywhere](goals.md#suspend-anywhere)). When the
interpreter stops, everything it was doing must be in the arena, so that the VM can be
copied into an image and resumed elsewhere ([persistence](goals.md#persistence)). A Rust
function halfway through its loop has state on the Rust stack, and that state cannot be
saved.

So djs has one rule:

> **The leaf rule.** Hand-written Rust never calls back into JS. It is limited to leaf
> intrinsics (`%ArrayPush`, `%StringIndexOf`, `%DefineOwnPropertyFromRecord`, ...),
> which never run user code. Every builtin that may run user code is written in the
> builtin language.

## The builtin language

The builtins live in `builtins/*.ts`: about a thousand functions in a strict subset of
TypeScript, of which <!--n builtins/all-->458<!--/n--> are installed as builtins and the rest are the spec's
abstract operations and helpers. A builtin does the conversions and checks in spec order,
where user code can run, and hands the actual work to leaf intrinsics. Here is
`Array.prototype.includes` from [`builtins/array.ts`](../../builtins/array.ts):

```ts
/**
 * Array.prototype.includes ( searchElement [ , fromIndex ] ), 23.1.3.16.
 * @builtin Array.prototype.includes
 * @fast
 */
function ArrayPrototypeIncludes(this: Value, searchElement: Value, fromIndex?: Value): boolean {
  const o: Obj = ToObject(this);
  const len: f64 = LengthOfArrayLike(o);
  if (len === 0) {
    return false;
  }
  let n: f64 = ToIntegerOrInfinity(fromIndex);
  if (n < 0) {
    n = len + n;
    if (n < 0) {
      n = 0;
    }
  }
  const found: f64 = ArraySearchDense(o, searchElement, n, len, 2);
  if (found !== -2) {
    return found !== -1;
  }
  for (let k: f64 = n; k < len; k++) {
    if (SameValueZero(Get(o, k), searchElement)) {
      return true;
    }
  }
  return false;
}
```

It reads like the spec, step by step. `LengthOfArrayLike`, `ToIntegerOrInfinity` and
`Get` may run user code. `ArraySearchDense` is a leaf: it searches the array's dense
storage in Rust in one go when every element it looks at is a plain data element, and
returns `-2` when it cannot answer (a hole, an accessor, an object that is not an Array),
so the loop below it handles the general case.

The subset is small on purpose:

- Only top-level functions with explicit types: `Value`, `Obj`, `Str`, `Sym`, `Fn`,
  `boolean`, and the numeric types `i32`, `u32` and `f64`. Numbers never mix types
  implicitly, so every backend (below) computes the same results.
- No property access, object literals, closures, `new`, `switch`, `typeof` or
  generators. `o.x` is `Get(o, "x")`, and user functions are called through
  `Call(f, thisArg, ...)`. If something is awkward without them, the answer is a new
  intrinsic.
- Conditions are booleans: there is no truthiness.

Intrinsics are declared in [`builtins/intrinsics.d.ts`](../../builtins/intrinsics.d.ts),
each tagged `@leaf` (never runs user code) or `@reentrant` (may call exactly one user
function, whose result completes the operation, such as `Get` calling a getter).
<!--n intrinsics/leaf-->296<!--/n--> are leaves and <!--n intrinsics/reentrant-->30<!--/n--> are reentrant. Builtins tagged `@intrinsic` join them, so that the
interpreter's slow paths and other builtins can call them as `%Name(...)`. They are all
numbered in one table; the [intrinsics reference](ref-intrinsics.md) lists them.

A call to a reentrant intrinsic or to another reentrant builtin is a **resume point**:
the builtin may stop there and resume later. djs-bgen finds every resume point and lists
them in [`builtins/gen/resume-points.txt`](../../builtins/gen/resume-points.txt), which is
checked in, so a review shows when a builtin starts calling user code.

> **Trade-off.** A subset of TypeScript over hand-written Rust builtins. The subset
> keeps every builtin resumable: its state lives in frames in the arena, never on the
> Rust stack. Builtins read like the spec they implement, and editors type-check them
> with plain `tsc`. The cost: a compiler to maintain (djs-bgen), a style without
> property syntax or closures, and builtins that run as interpreted bytecode unless they
> are compiled to Rust as below.

> **Trade-off.** Its own parser for the subset over the TypeScript compiler or another
> full TypeScript front end. djs-bgen is a dependency-free Rust crate with a hand-written
> parser and type checker, so regenerating the builtins needs only cargo, and accepting
> only the subset is most of the checking. The cost: a parser and its error messages to
> maintain, and no full TypeScript tooling inside bgen.

## Three backends, one implementation each

djs-bgen compiles each builtin one way: to bytecode by default, to a Rust state machine
when it is tagged `@native`, and to straight-line Rust when it is tagged `@rust`. A
builtin compiled to Rust has no bytecode body: the prelude keeps an empty function for
it, so that the function object, its name and its length exist, and the interpreter runs
the Rust whenever it is called. The TypeScript stays the one source of every builtin.

### Bytecode: the prelude and the base snapshot

For the bytecode backend, djs-bgen erases the types and writes
[`builtins/gen/prelude.js`](../../builtins/gen/prelude.js): plain JavaScript with
`%Name(...)` intrinsic calls, an installation call per builtin (`%InstallMethod`,
`%InstallConstructor`, ...), and the list of `@native`, `@rust` and `@fast` builtins. djs compiles
it in *privileged mode*, the only mode in which `%Name(...)` compiles: to the
`call_intrinsic` opcode.

<!-- gen:ops call_intrinsic -->
| opcode | byte | immediates | size | pop → push | cost | description |
| --- | ---: | --- | ---: | --- | --- | --- |
| `call_intrinsic` | 157 | Intrinsic, Argc | 5 | imm1 → 1 | Call | `a1 .. an -> r`: privileged code only. |
<!-- /gen -->

The prelude is one wrapper function that declares every builtin as a function inside
it. A builtin that calls another one, such as the helper `LengthOfArrayLike`, would
normally capture that name the way any closure captures a variable of its enclosing
function: a Box cell per captured function, a capture descriptor per caller and a slot
per capture in every caller's function object. Privileged code does without them. The
compiler gives each prelude function that another one refers to a slot in one table of
the realm, references compile to `get_builtin` (read the slot) instead of `get_cap`, and
the wrapper fills the table once with `put_builtin`. User code never sees the table, and
its closures capture as usual.

Builtins are written as many small helpers (`ToNumber`, `ToIntegerOrInfinity`,
`ThisToString`, `ClampPosition`), as the spec is, and a call of one costs a frame and a
handful of instructions, often more than the helper's own work. So djs-bgen inlines a
small helper into its callers in the prelude: when the helper's body can be written as
one expression (declarations, `if` with `return`, `throw` as the `%Throw(v)` intrinsic)
and that expression stays very short, the call becomes the expression; a tiny helper
inlined is smaller than the call it replaces. The arguments go into temporaries of the caller first,
in order, so they are evaluated exactly once and in the same order as before. Only
internal helpers are inlined: they never show in stack traces, so nothing observable
changes. A helper whose frame the engine looks for on the stack is tagged `@frame` and
keeps it. Here is `String.prototype.substr` as the prelude has it, with the helper
`StrLen` inlined (it became `(%StrLength(s))`) and the longer `ThisToString` and
`RelativeIndex` called:

<!-- gen:excerpt builtins/gen/prelude.js "String.prototype.substr" "function StringPrototypeSubstr(" 8 -->
[`builtins/gen/prelude.js`](../../builtins/gen/prelude.js#L9699-L9706), lines 9699–9706:

```js
function StringPrototypeSubstr(start, length) {
  const s = ThisToString(this);
  const size = (%StrLength(s));
  const intStart = RelativeIndex(start, size, 0);
  let intLength = size;
  if (length !== undefined) {
    intLength = ToIntegerOrInfinity(length);
  }
```
<!-- /gen -->

> **Trade-off.** Inlining at the source over cheaper helper calls. A frame for a
> builtin's helper is already about as cheap as a call of a user function; what costs
> is the number of them. The cost: every inlined call site is a copy of the helper's
> body in the base image, so only the shortest bodies are inlined (the hot builtins whose
> speed depended on more inlining run as Rust), and the prelude reads less like the
> TypeScript it comes from.

The prelude runs once, at build time, when the base snapshot is made: the realm with
every builtin installed and compiled, collected, compacted and embedded in the engine
([the base snapshot](snapshots.md#the-base-snapshot)). Every VM starts as a copy of that
image, so no VM ever compiles the prelude, and creating one is a copy of bytes
([fast startup in wasm](goals.md#fast-startup-in-wasm)). Privileged templates keep no
line tables or local names, which keeps that image small.

### Rust: `@native` state machines

A builtin tagged `@native` is compiled to a Rust state machine in
[`crates/djs-core/src/builtins/gen/natives.rs`](../../crates/djs-core/src/builtins/gen/natives.rs).
There are eight: `Array.prototype.push`, `sort` and its comparison loop, and the five
functions of `JSON.stringify`'s serializer. (The merging between two comparisons is a
leaf: it keeps its position in an internal array on the heap, so the state machine only
asks for the next comparison's result and hands it back. A comparator that is a plain
closure runs its comparisons the way `%EachDense` runs callbacks, described below: one
frame, entered again per comparison while the results are Numbers.) bgen splits the function into basic blocks,
and each block becomes one arm of a `match` on the state. A resume point ends a block.
Here is the loop of `Array.prototype.push` for several items, excerpted from the
generated file. The source is the plain loop
`for (let i = 0; i < argCount; i++) { Set(o, len, Get(items, i)); len++; }`:

<!-- gen:excerpt crates/djs-core/src/builtins/gen/natives.rs "fn native_ArrayPrototypePush(" "7 => {" 18 -->
[`crates/djs-core/src/builtins/gen/natives.rs`](../../crates/djs-core/src/builtins/gen/natives.rs#L95-L112), lines 95–112:

```rust
7 => {
    if (l4f < l2f) { st = 8; continue; } else { st = 10; continue; }
}
8 => {
    // resume point: Set
    match { let a0 = ex.n_slot(0); let a1 = ex.n_slot(1); let a2 = { let a0 = ex.n_slot(4); ex.n_rest_get(0, a0) }; ex.n_set(&[a0, a1, a2], 11) }? { Flow::Inline => { st = 11; continue } Flow::Leave(s) => return Ok(s) }
}
9 => {
    let rv = (l4f + 1.0_f64); l4f = rv; ex.n_set_slot(4, num(rv));
    if ex.n_backedge(7) { return Ok(None); } st = 7; continue;
}
10 => {
    // resume point: Set
    match { let a0 = ex.n_slot(0); let a1 = ex.n_str("length")?; let a2 = ex.n_slot(1); ex.n_set(&[a0, a1, a2], 12) }? { Flow::Inline => { st = 12; continue } Flow::Leave(s) => return Ok(s) }
}
11 => {
    ex.n_result();
    let rv = (l1f + 1.0_f64); l1f = rv; ex.n_set_slot(1, num(rv));
```
<!-- /gen -->

A native builtin runs in a **native frame**: an ordinary frame record in the arena with
a header bit set, its state in place of the program counter and its locals in the
frame's slots. A typed local such as the counter `i` (`l4f` above) also lives in a Rust
variable, but every write goes to the slot too, so the frame always holds the whole
state. The common case stays in the same Rust loop. State 8's `Set` is a resume point:
on an Array it completes inline (`Flow::Inline`) and the loop goes on to state 11. When a
call needs a JavaScript frame (a setter, a proxy trap), the native writes the callee's
frame and returns to the interpreter (`Flow::Leave`), which runs it and later re-enters
the state machine at the resume state. A loop back-edge (`n_backedge`) charges fuel and
is a safepoint, so a native loop over a million elements can stop and be saved in the
middle, like bytecode.

Two rules keep this safe:

- **No collection inside a state arm.** An allocation never collects; it only sets a
  pending flag. The collector runs at state transitions (back-edges, and calls that
  return to the interpreter), where every value the builtin holds is in a frame slot.
  The one exception is collect-and-retry at the heap cap: a native operation that hits
  the cap collects and runs again, but only at points where bgen knows that the
  generated code holds no other heap value in a Rust local.
- **Fuel per transition.** A native frame charges 4 fuel per back-edge and 2 per call,
  where bytecode charges per instruction ([fuel](limits.md#fuel)).

The callback loops of `forEach`, `map`, `filter`, `some`, `every`, `find`, `findIndex`
and `reduce`, and `Array.from(arrayLike, f)`, are bytecode builtins, not native ones. Their dense part is one intrinsic, `%EachDense`: it enters the callback's frame
once and, on each return, completes the step (store, test, accumulate) and enters the
same frame again for the next element. `sort`'s comparisons use the same loop: each
Number result goes straight to the merge step, and the frame is entered again with the
next pair; a result of another type goes back to the builtin, whose `ToNumber` may run
user code. No builtin frame runs per element, which is
faster than a native state machine that calls the callback each time, and the builtin's
own spec loop takes over at the first hole, proxy or other kind of object. `reduce`
keeps its accumulator in the loop's own state and learns whether `initialValue` was
passed from its frame's argument count (`%ArgumentCount`) rather than from a rest
parameter, so a call on an empty array allocates nothing.

### Rust: `@rust` straight-line builtins

Most hot builtins never loop over user code: `s.trim()`, `Math.floor(x)`,
`s.slice(a, b)`, `s.startsWith(t)`. They meet user code only while converting an
argument (a `valueOf` or `toString` method, a `Symbol.toPrimitive`) or reading one
property of an argument (`startsWith` asks whether its argument is a regular
expression). A builtin tagged `@rust` is compiled to one plain Rust function in
[`crates/djs-core/src/builtins/gen/leaves.rs`](../../crates/djs-core/src/builtins/gen/leaves.rs):
the helpers it calls are inlined, the leaf intrinsics are called directly, and it runs
with no frame. djs-bgen refuses a `@rust` builtin with a loop, a `try`, a rest
parameter or any call that could run user code anywhere but at those places.

Those places are its *conversion sites*: `ToPrimitive` of an object and `Get`. The Rust
cannot run user code itself, so at a site it stops and asks: the interpreter runs the
operation as bytecode (the prelude's `ToPrimitive`, or a `Get` that may call a getter)
in a frame of its own, puts the result in the site's slot on the stack, and calls the
builtin again. On that call the site takes its value from the slot, and the builtin
goes on. Everything before a site only computes, without any effect, so running it
again changes nothing, and the arguments are never replaced: a check of the original
value, such as `trim`'s `this === null`, still sees the object. While the operation
runs, a stack trace shows the builtin as `(native)` between it and the caller, as it
would show a bytecode builtin's frame:

```js live
try {
  "abc".startsWith({ get [Symbol.match]() { throw new Error("from the getter"); } });
} catch (e) {
  console.log(e.message);
}
console.log(Math.floor({ valueOf() { console.log("valueOf ran"); return 4.5; } }));
console.log(String.prototype.trim.call({ toString() { return null; } }));
```
<!-- gen:run -->
```text
from the getter
valueOf ran
4
null
```
<!-- /gen -->

A builtin that reaches the heap's memory cap is called once more after a collection,
the way bytecode retries an instruction; the second failure throws. Builtins whose
generated bodies differ only in which leaves they call and in their number literals
share one function: every `Math` function of one argument is the same Rust, with the
leaf to call in a small table.

## `@fast`: leaf entries for everyday calls

Some hot builtins rarely meet user code but loop over their arguments or need more than
a conversion site: `"a,b".split(",")`, `xs.indexOf(3)`, `xs.push(v)`, `m.get(k)`. Even as native code, entering a frame and running
`ToObject` and `LengthOfArrayLike` costs more than the work itself. A builtin tagged
`@fast` has a hand-written Rust entry of the same name in
[`crates/djs-core/src/builtins/fast.rs`](../../crates/djs-core/src/builtins/fast.rs).
A call first runs the entry, which either returns the builtin's result or *declines*,
and then the builtin runs as usual. <!--n builtins/fast-->43<!--/n--> builtins have one.

A constructor's entry can also answer `new`: `new Map()`, `new Set()`, `new Array(n)`
and `new Date(v)` for a primitive or a Date. It runs only when `new.target` is the
constructor itself, whose `prototype` property can never change, so the new object's
prototype is known without a lookup. A subclass's `super()` and `Reflect.construct`
with another `new.target` run the builtin.

An entry may answer only when nothing observable can happen: primitive arguments, dense
arrays whose elements are plain data, no proxies. Every case that would throw declines,
so errors and their stack traces always come from the builtin itself. An entry that
reaches the heap cap declines too, so that the builtin's own allocations collect and
retry. Only `Array.prototype.push` is both `@fast` and `@native`: the entry covers the
common case, and the state machine handles the rest.

Here is the slow path at work. The array has a getter at index 1, so `includes` can use
neither its fast entry nor its dense leaf and runs the getter from its spec loop. The
`map` callback's stack trace shows the builtin's frame as `(native)`, as QuickJS prints
it:

```js live
const arr = [1, 2, 3];
Object.defineProperty(arr, 1, { get() { console.log("getter ran"); return 2; } });
console.log(arr.includes(2));
const doubled = [1, 2, 3].map((x) => {
  if (x === 2) console.log(new Error("in the callback").stack.trimEnd());
  return x * 2;
});
console.log(doubled);
```
<!-- gen:run -->
```text
getter ran
true
    at <anonymous> (example.js:5:37)
    at map (native)
    at <eval> (example.js:4:30)
[2,4,6]
```
<!-- /gen -->

> **Trade-off.** Hand-written fast entries over making the generic path faster. An entry
> does in a few Rust instructions what the builtin does in dozens of steps, for the
> calls codemode scripts make most. The cost: each entry duplicates its builtin's logic
> for the cases it accepts, and only tests keep the two in step. The corpus checks the
> entries against QuickJS, including every case where they must decline. Where a
> builtin fits `@rust`, generating it removes the duplicate.

## Which builtins run as Rust

Rust is several times the size of the same builtin as bytecode, in every wasm module,
and module size is part of startup
([fast startup in wasm](goals.md#fast-startup-in-wasm)). So a builtin runs as Rust only
where that pays. A `@rust` builtin costs about fifty bytes of gzipped module; the hot
ones that fit are generated (the `Math` functions of one argument, `trim`, the case
functions, `slice`, `substring`, `at`, `charAt`, `charCodeAt`, `indexOf`, `lastIndexOf`,
`includes`, `startsWith`, `endsWith`, `repeat`, `padStart`, `padEnd`, `toFixed`,
`toString` of numbers, `parseInt`, `parseFloat`, `Array.isArray`, and a few more), the
cold ones stay bytecode. An `@native` state machine or an `@fast` entry, a larger piece
of Rust, must pay for its bytes by measurement: it stays only where turning it off makes
typical codemode work at least 1% slower in wasm instructions, or makes some benchmark
workload slower by at least 10% for each kilobyte (gzipped) of its code. Ones that call
each other are judged together, such as `sort` and its comparison loop, or the
serializer functions of `JSON.stringify`.
`scripts/native-sizes.sh` prints the wasm size of each native function.

By that rule there are <!--n builtins/fast-->43<!--/n--> `@fast` entries (`split` on strings, `indexOf`, `includes`
and `slice` on arrays, `push`, `pop` and `join`, `String.fromCharCode`, `String` and
`Number` called as functions, `Math.max` and `Math.min`, `ArraySpeciesCreate`, `get`,
`has`, `set` and `add` of `Map` and `Set`, and the conversion helpers the builtins call
most) and the <!--n builtins/native-->8<!--/n--> `@native` functions above. A bytecode builtin is not slow by
default: its loops over dense arrays already run in leaf intrinsics.

> **Trade-off.** Rust for the hot builtins only, over making every builtin native.
> Generated Rust runs several times faster than the same builtin as bytecode (on the
> benchmark workloads, forEach callbacks run <!--n native/foreach-x .1-->2.1<!--/n--> and array methods <!--n native/arrays-x .1-->3.7<!--/n--> times as fast),
> but most builtins are called too rarely for that to show, and each one would add wasm
> code that every host loads.

> **Trade-off.** One implementation per builtin over a bytecode twin of every Rust one.
> With a twin, the two compilations could be run against each other; without it, the
> builtin is checked against QuickJS (the differential corpus) and test262 directly, and
> the TypeScript of every builtin, the generated ones included, still runs in QuickJS in
> djs-bgen's own test against QuickJS's natives. The bytecode a twin would need costs
> base-image bytes, and the rules that kept a twin's behavior in step with its Rust
> (declining wherever the builtin would throw) are gone with it.

## Names that outlive a build: the builtin registry

Offsets, intrinsic ids and the base image's layout change from build to build: a new
builtin shifts every cell after it. Something that must name a builtin across builds
(a snapshot exported by meaning, a heap dump read by a newer tool) names it by its
*registry* entry instead. Every builtin object, function and symbol of the base realm,
and every intrinsic, has a stable id there, keyed by a path in the spec's style:

```text
0     value      globalThis
2     value      Array
267   value      Array.prototype.map
431   value      get Map.prototype.size
538   value      %ArrayIteratorPrototype%
690   value      %intrinsic:PromiseResolveFunction%   (the prelude function behind it)
1409  value      %prelude:SameValueZero%              (a prelude helper builtins call)
1349  intrinsic  PromiseResolveFunction               (the intrinsic, by name)
```

A path walks from `globalThis`, a global, a realm intrinsic that no global reaches
(`%TypedArray%`), a prelude function registered for an intrinsic (`%intrinsic:Name%`) or
a helper in the realm's prelude function table, by its source name (`%prelude:Name%`),
through properties (`.key`, `[Symbol.iterator]`) and `.[[Prototype]]`. A new builtin gets
the first path a breadth-first walk finds, so the paths read like the spec wherever the
spec has a name.

The table is a committed file,
[`tests/builtins/registry.tsv`](../../tests/builtins/registry.tsv), and it only grows:

- an id is never renumbered or reused;
- a builtin that goes away keeps its line, marked removed: a tombstone;
- a builtin reached by a second path keeps its first id (the paths are aliases).

`scripts/registry-update.sh` appends new builtins and tombstones removed ones. Tests fail
when a builtin has no entry or an entry does not resolve, and the gate fails when a line
of main's table is renumbered or dropped. At build time the base image gets a table from
ids to cell offsets
([`crates/djs-prelude/src/registry.rs`](../../crates/djs-prelude/src/registry.rs)); for
any other heap the paths are resolved on that heap
([`crates/djs-core/src/vm/registry.rs`](../../crates/djs-core/src/vm/registry.rs)). The
heap analyzer prints builtins by their registry paths (`%Array.prototype.map%`).

> **Trade-off.** Paths over generated names. A path is what a reader recognizes and
> what the spec prints, and it is checked by resolving it. Helpers that no property
> reaches are named by their source name in the prelude (`%prelude:Name%`), which stays
> the same as long as the helper keeps its name, whatever the builtins that call it do.
> The cost: a rename of a helper is a new entry and a tombstone.

## Frozen builtins

Codemode hosts usually want builtins that a script cannot change: a script that patches
`Array.prototype.map` changes what every later call means, and in a VM that lives
across snapshots the patch lives on too. The usual answer is a lockdown at startup, in
the style of SES: walk every builtin, `Object.freeze` it, and replace the commonly
overridden properties with accessor pairs. In djs that walk turns hundreds of compact
objects into property dictionaries, adds hundreds of accessor closures, and takes
milliseconds per VM, where a start without it takes tens of microseconds.

djs offers the same result as an option at VM creation: `frozen_builtins` in
[`VmOptions`](../../crates/djs-core/src/vm/mod.rs), `frozenBuiltins` in the
[Sandbox](js-api.md#creating-a-sandbox), flag 1 of `vm_new_with` in wasm. Nothing is
frozen. Every VM's arena starts with a copy of the base image, so the builtins sit below
a known offset, the end of the builtin prefix. With the option on, the property
operations treat every object below it as frozen: it is not extensible, its properties
read as non-writable and non-configurable, and every write, addition, deletion or
redefinition fails, with a `TypeError` in strict code and `false` from `Reflect`.
`Object.isFrozen(Array.prototype)` is true. The global object lives above the prefix and
stays ordinary, so scripts can still define globals. [The
language](language.md#frozen-builtins) shows what a script sees.

A frozen prototype chain has the *override mistake*: the spec lets an inherited
read-only property stop an assignment to a different object, so under `Object.freeze`
`this.name = "NotFound"` in an `Error` subclass, or `o.toString = f` on a plain object,
would throw. The option fixes that the way SES and Node's `--frozen-intrinsics` do: when
the read-only property belongs to a frozen builtin that is not the receiver, the
assignment defines an own property on the receiver instead. The builtin's properties
stay data properties; no accessors appear in their descriptors.

The check compares the object's offset with the prefix end, which is 0 with the option
off, so the default costs one compare per property write and stays spec-conformant.
Engine-internal state in those objects (internal slots, header bits) is written as
usual; only the JavaScript-visible property operations look at the check. The mode is
part of the image: a restored VM keeps it. Fast paths lean on it too: `map`, `filter` and
the other methods that create their result through the array's constructor normally
check that `Array.prototype.constructor` and `Array[Symbol.species]` are still the
originals; when both objects are frozen builtins, that check is the offset compare.

**Why dumps need it.** A [continuation dump](dumps.md) is a JavaScript program that
rebuilds a run, and it does not copy the builtins: it writes a builtin the run looked up
as the expression that finds it again in the loading VM, and a method a half-done call
had already read as a second read of it. That is only right if no builtin can change
between the moment the run read it and the load. Frozen builtins guarantee it, so only a
VM with frozen builtins can write a dump ([what a dump needs](dumps.md#what-a-dump-needs)).

> **Trade-off.** A region check over freezing the objects. Freezing at startup costs
> time and memory in every VM and rewrites the base; marking frozen objects in their
> headers would need a different base image per mode. The region check keeps one base for
> both modes and freezes nothing. The cost: the override fix is a deliberate deviation
> from the spec (test262 expects the `TypeError`), so the mode is opt-in, and a builtin is
> frozen by where it lives, which works only because the prefix never moves: collections
> leave every cell below its end in place ([the garbage collector](gc.md)), and the
> per-VM objects that must stay writable, such as the global object, live above it.

## Two builds: full and slim

djs builds in two variants from one source. The default build has every builtin family.
The *slim* build leaves out the families that codemode scripts never use and that cost
real size: typed arrays with ArrayBuffer and DataView, BigInt, the Iterator helpers and
the `Iterator` global, WeakRef and FinalizationRegistry, DisposableStack with
SuppressedError and the `using` declarations, and decorators. None of thousands of real
codemode scripts uses any of them. Leaving them out makes the wasm module tens of
kilobytes smaller in gzip and the base image, which every VM copies, about a quarter
smaller. [The language](language.md#the-slim-build) shows what a script sees in it.

The variants are cargo features, and features may only add things. So the default build
turns on a feature called `full`, which carries those families, and a second one,
`unicode`, which carries the Unicode property tables of `\p{…}` in regular expressions
and full normalization. Building without the default features gives the slim build.

Three mechanisms take a family out:

- **Builtins.** A builtin file can be marked as left out of the slim build, and so can a
  single builtin (SuppressedError lives with the other errors). The builtin compiler then
  writes a second prelude without them,
  [`builtins/gen/prelude-slim.js`](../../builtins/gen/prelude-slim.js), and the slim
  build makes its base image from that one. Installs that a kept builtin makes onto an
  object of a left-out family, such as `%TypedArray%.prototype.toString`, which is
  Array's `toString`, go too.
- **Rust.** Code is cut with a constant (`FULL` in
  [`crates/djs-core/src/lib.rs`](../../crates/djs-core/src/lib.rs)) rather than with
  conditional compilation, so both builds type-check the same code and the optimizer
  drops what the slim one can't reach. The leaf intrinsics of the left-out families have
  a `FULL` guard on their dispatch arms, and `Value` never reports a BigInt in the slim
  build, so the BigInt arithmetic behind every numeric operator falls away.
- **Syntax.** BigInt literals, decorators and `using` are a SyntaxError that names the
  build.

The builtin registry stays one table. Next to it a committed list,
[`tests/builtins/registry-slim.txt`](../../tests/builtins/registry-slim.txt), names the
entries the slim build doesn't have, and the slim build's tests check that exactly those
don't resolve. The prototype of an absent constructor that the engine still has
(`Iterator.prototype`, which every iterator inherits from) keeps its id. An export from a
slim VM therefore names its builtins with the same ids as a full one.

> **Trade-off.** One feature for the whole cut over one per family. Each extra
> combination would need its own base image, test runs and size budget, and the families
> were chosen together from the same evidence. The cost: the slim build can't run all of
> test262 or the differential corpus, so it is tested on its own terms: its codemode
> corpora with the stress modes, test262 without the left-out families against its own
> list of expected failures, an [agent host's](pi.md) own codemode tests on frozen
> builtins, and its own size ceiling.

## Where to look

- [`builtins/`](../../builtins/): the builtins in the TypeScript subset;
  [`builtins/intrinsics.d.ts`](../../builtins/intrinsics.d.ts): the intrinsic
  declarations ([reference](ref-intrinsics.md)).
- [`crates/djs-bgen/`](../../crates/djs-bgen/): the parser, checker, resume-point
  analysis ([`crates/djs-bgen/src/analysis.rs`](../../crates/djs-bgen/src/analysis.rs))
  and the three emitters
  ([`crates/djs-bgen/src/emit_js.rs`](../../crates/djs-bgen/src/emit_js.rs),
  [`crates/djs-bgen/src/emit_rust.rs`](../../crates/djs-bgen/src/emit_rust.rs),
  [`crates/djs-bgen/src/emit_leaf.rs`](../../crates/djs-bgen/src/emit_leaf.rs)).
- [`builtins/gen/`](../../builtins/gen/): the generated preludes (full and slim) and the
  resume-point report;
  [`crates/djs-core/src/builtins/gen/natives.rs`](../../crates/djs-core/src/builtins/gen/natives.rs):
  the generated state machines;
  [`crates/djs-core/src/builtins/gen/leaves.rs`](../../crates/djs-core/src/builtins/gen/leaves.rs):
  the generated straight-line builtins, called by `own_call` in
  [`crates/djs-core/src/interp/run/fastpath.rs`](../../crates/djs-core/src/interp/run/fastpath.rs).
- [`crates/djs-prelude/build.rs`](../../crates/djs-prelude/build.rs): runs the prelude
  and writes the base snapshot.
- [`crates/djs-core/src/interp/run/native.rs`](../../crates/djs-core/src/interp/run/native.rs):
  native frames, their fuel and their safepoints.
- [`crates/djs-core/src/builtins/fast.rs`](../../crates/djs-core/src/builtins/fast.rs):
  the `@fast` entries;
  [`crates/djs-core/src/builtins/dense.rs`](../../crates/djs-core/src/builtins/dense.rs):
  the dense leaves;
  [`crates/djs-core/src/interp/run/each.rs`](../../crates/djs-core/src/interp/run/each.rs):
  `%EachDense`.
- [`crates/djs-core/src/heap/frozen.rs`](../../crates/djs-core/src/heap/frozen.rs): the
  frozen-builtins check; its gates are in
  [`crates/djs-core/src/heap/object.rs`](../../crates/djs-core/src/heap/object.rs), the
  override fix in `ordinary_set`
  ([`crates/djs-core/src/interp/run/intrinsics.rs`](../../crates/djs-core/src/interp/run/intrinsics.rs)).
- [`crates/djs-core/src/vm/registry.rs`](../../crates/djs-core/src/vm/registry.rs) and
  [`tests/builtins/registry.tsv`](../../tests/builtins/registry.tsv): the builtin
  registry.
