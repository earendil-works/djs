# Builtins: the TypeScript subset and the leaf rule

`Array.prototype.map`, `JSON.stringify`, `Promise.all`, `Object.defineProperty`: a
JavaScript engine is mostly its standard library, and most of a codemode script's time
is spent in it. This page explains how djs writes its builtins, why almost none of them
are hand-written Rust, and how the hot ones still run as compiled Rust.

> **In short.** Hand-written Rust never calls back into JavaScript. It only provides
> *leaf intrinsics*, operations that cannot run user code. Every builtin that might run
> user code is written in a strict subset of TypeScript and compiled by djs-bgen to
> bytecode that ships in the base snapshot, and, for the hot ones, also to resumable
> Rust state machines (`@native`). Everyday calls with plain arguments skip both through
> `@fast` leaf entries.

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
copied into an image and resumed elsewhere. A Rust function halfway through its loop has
state on the Rust stack, and that state cannot be saved.

So djs has one rule:

> **The leaf rule.** Hand-written Rust never calls back into JS. Hand-written Rust is
> limited to leaf intrinsics (`%ArrayPush`, `%StringIndexOf`,
> `%DefineOwnPropertyFromRecord`, ...), which never run user code. Every builtin that
> may run user code is written in the builtin language.

## The builtin language

Builtins live in `builtins/*.ts`, about a thousand functions in a strict subset of
TypeScript. A builtin does the conversions and checks in spec order, where user code can
run, and hands the actual work to leaf intrinsics. Here is `Array.prototype.includes`
from `builtins/array.ts`:

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
`Get` may run user code. `ArraySearchDense` is a leaf (D1801): it searches the array's
dense storage in Rust in one go when every element it looks at is a plain data element,
and returns `-2` when it cannot answer (a hole, an accessor, an object that is not an
Array), so the loop below it handles the general case.

The subset is small on purpose (D1171):

- Only top-level functions with explicit types: `Value`, `Obj`, `Str`, `Sym`, `Fn`,
  `boolean`, and the numeric types `i32`, `u32` and `f64`. Numbers never mix types
  implicitly, so both backends (below) agree on every result.
- No property access, object literals, closures, `new`, `switch`, `typeof` or
  generators. `o.x` is `Get(o, "x")`, and user functions are called through
  `Call(f, thisArg, ...)`. If something is awkward without them, the answer is a new
  intrinsic.
- Conditions are booleans: there is no truthiness.

Intrinsics are declared in `builtins/intrinsics.d.ts`, each tagged `@leaf` (never runs
user code) or `@reentrant` (may call exactly one user function, whose result completes
the operation, such as `Get` calling a getter). Today about 280 are leaves and about 30
are reentrant. Builtins tagged `@intrinsic` join them, so that the interpreter's slow
paths and other builtins can call them as `%Name(...)`. They are all numbered in one
table (D1270); the [intrinsics reference](ref-intrinsics.md) lists them.

A call to a reentrant intrinsic or another reentrant builtin is a **resume point**: the
builtin may stop there and resume later. djs-bgen finds every resume point and lists
them in `builtins/gen/resume-points.txt`, which is checked in, so a review shows when a
builtin starts calling user code.

> **Trade-off.** A subset of TypeScript over hand-written Rust builtins. The subset
> keeps every builtin resumable: its state lives in frames in the arena, never on the
> Rust stack. Builtins read like the spec they implement, and editors type-check them
> with plain `tsc`. The cost: a compiler to maintain (djs-bgen), a style without
> property syntax or closures, and builtins that run as interpreted bytecode unless they
> get one of the faster paths below.

> **Trade-off.** Our own parser for the subset over the TypeScript compiler or oxc
> (D1170). djs-bgen is a dependency-free Rust crate with a hand-written parser and type
> checker, so regenerating needs only cargo, and accepting only the subset is most of
> the checking. The cost: its own parser and messages to maintain, and no full-TS
> tooling inside bgen.

## Two backends from one source

djs-bgen compiles every builtin twice.

**The bytecode backend** covers every builtin. bgen erases the types and writes
`builtins/gen/prelude.js`: plain JavaScript with `%Name(...)` intrinsic calls, an
installation call per builtin (`%InstallMethod`, `%InstallConstructor`, ..., D1172,
D1310), and the list of `@native` and `@fast` builtins. djs compiles it in *privileged
mode*, the only mode in which `%Name(...)` compiles: to the `call_intrinsic` opcode.

<!-- gen:ops call_intrinsic -->
| opcode | byte | immediates | size | pop → push | cost | description |
| --- | ---: | --- | ---: | --- | --- | --- |
| `call_intrinsic` | 157 | Intrinsic, Argc | 5 | imm1 → 1 | Call | `a1 .. an -> r`: privileged code only. |
<!-- /gen -->

The prelude runs once, when the base snapshot is built (D1321). Every VM starts as a
copy of that image, with all builtins installed and compiled, so no VM ever compiles the
prelude. Privileged templates keep no line tables or local names, which saves space in
that image (D1430, D1980).

**The Rust backend** is opt-in. A builtin tagged `@native` (ten of them: `push`, `sort`
and its merge, `JSON.stringify`'s slow path, promise reaction jobs, ...) is also
compiled to a Rust state machine in
`crates/djs-core/src/builtins/gen/natives.rs` (D1632). bgen splits the function into
basic blocks, and each block becomes one arm of a `match` on the state. A resume point
ends a block. Here is the loop of `Array.prototype.push` for several items, excerpted
from the generated file. The source is the plain loop
`for (let i = 0; i < argCount; i++) { Set(o, len, Get(items, i)); len++; }`:

<!-- gen:excerpt crates/djs-core/src/builtins/gen/natives.rs "fn native_ArrayPrototypePush(" "7 => {" 18 -->
[`crates/djs-core/src/builtins/gen/natives.rs`](../../crates/djs-core/src/builtins/gen/natives.rs#L108-L125), lines 108–125:

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
frame's slots (D1631). A typed local such as the counter `i` (`l4f` above) also lives in
a Rust variable, but every write goes to the slot too (D2000), so the frame always holds
the whole state. The common case stays in the same Rust loop. State 8's `Set` is a
resume point: on an Array it completes inline (`Flow::Inline`) and the loop goes on to
state 11. When a call needs a JavaScript frame (a setter, a proxy trap), the native
writes the callee's frame and returns to the interpreter (`Flow::Leave`), which runs it
and later re-enters the state machine at the resume state. A loop back-edge
(`n_backedge`) charges fuel and is a safepoint, so a native loop over a million elements
can stop and be saved in the middle, like bytecode.

The callback loops of `forEach`, `map`, `filter`, `some`, `every`, `find`, `findIndex`
and `reduce` are bytecode builtins. Their dense part is one intrinsic, `%EachDense`
(D2303). It enters the callback's frame once and, on each return, completes the step
(store, test, accumulate) and enters the same frame again for the next element. No
native state machine runs per element, and the builtin's own spec loop takes over at the
first hole, proxy or other array. That made these builtins about twice as fast per
element as their native versions, and dropping them from the Rust backend left the
module smaller.

Two rules keep this safe:

- **No collection inside a state arm.** An allocation never collects; it only sets a
  pending flag. The collector runs at state transitions (back-edges, and calls that
  return to the interpreter), where every value the builtin holds is in a frame slot.
  The one exception is collect-and-retry at the heap cap (D1900): a native operation
  that hits the cap collects and runs again, but only at points where bgen knows that
  the generated code holds no other heap value in a Rust local.
- **Fuel per transition.** A native frame charges 4 fuel per back-edge and 2 per call
  (D1633). Bytecode charges per instruction, so the fuel a run uses depends on the
  backend. Results do not.

Both backends must behave the same, and they are tested against each other: `djs run`,
the difftest runner and the test262 runner take `--backend bytecode|native`, and the
gate runs the corpus with both (D1635, [How djs is tested](testing.md)).

> **Trade-off.** An opt-in Rust backend over making every builtin native. Generated Rust
> runs much faster than the same builtin as bytecode (when it landed, 3.4 times for
> `map` and 5.3 times on the higher-order benchmark), but every native function adds
> wasm code, and wasm size is part of startup ([fast startup in
> wasm](goals.md#fast-startup-in-wasm)). The native API is kept out of line in wasm to
> limit that cost (D1634), and `scripts/native-sizes.sh` prints the size per function. A
> builtin becomes `@native` only when a benchmark shows that it is hot. The cost of
> having the option: two implementations of every native builtin's semantics to keep
> equal, which is why both are tested in the gate.

## `@fast`: leaf entries for everyday calls

Many hot builtins rarely meet user code: `"a,b".split(",")`, `xs.includes(3)`,
`Math.max(a, b)`, `xs.push(v)`. Even as native code, entering a frame and running
`ToObject` and `LengthOfArrayLike` costs more than the work itself. A builtin tagged
`@fast` has a hand-written Rust entry of the same name in
`crates/djs-core/src/builtins/fast.rs` (D1850). With the native backend, a call first
runs the entry, which either returns the builtin's result or *declines*, and then the
builtin runs as usual. About 80 builtins have one.

An entry is allowed to answer only when nothing observable can happen: primitive
arguments, dense arrays whose elements are plain data, no proxies. Every case that would
throw declines, so errors and their stack traces always come from the builtin itself. An
entry that reaches the heap cap declines too (D1910), so that the builtin's own
allocations collect and retry. A builtin is `@fast` or `@native`, not both, except
`Array.prototype.push` (D2020): the leaf covers the common case, and the bytecode
version handles the rest.

Here is the slow path at work. The array has a getter at index 1, so `includes` cannot
use its fast entry or its dense leaf and runs the getter from its loop. The `map`
callback runs inside the native `map`, and its stack trace shows the native frame:

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
> for the cases it accepts, and nothing but tests keeps them in sync. The corpus checks
> the entries against QuickJS, including every case where they must decline, and the
> gate's bytecode-backend run, which never takes them, keeps testing the TypeScript
> path.

## Where to look

- `builtins/*.ts` and [builtins/intrinsics.d.ts](../../builtins/intrinsics.d.ts): the
  sources and the intrinsic declarations ([reference](ref-intrinsics.md)).
- [`crates/djs-bgen/`](../../crates/djs-bgen/): the parser, checker, resume-point
  analysis (`analysis.rs`) and the two emitters (`emit_js.rs`, `emit_rust.rs`).
- [`builtins/gen/`](../../builtins/gen/): the generated prelude and resume-point report;
  [`crates/djs-core/src/builtins/gen/natives.rs`](../../crates/djs-core/src/builtins/gen/natives.rs):
  the generated state machines.
- [`crates/djs-core/src/interp/run/native.rs`](../../crates/djs-core/src/interp/run/native.rs):
  native frames;
  [`crates/djs-core/src/builtins/fast.rs`](../../crates/djs-core/src/builtins/fast.rs)
  and `builtins/dense.rs`: the `@fast` entries and dense leaves.

Decisions on this page:

<!-- gen:decisions D1170 D1171 D1172 D1270 D1310 D1321 D1430 D1631 D1632 D1633 D1634 D1635 D1801 D1850 D1900 D1910 D2000 D2020 D2060 D2090 -->
- D1170: djs-bgen is Rust with its own parser for the subset
- D1171: Builtin subset rules
- D1172: Generated prelude format and installation
- D1270: Intrinsic ids
- D1310: Builtin installation forms
- D1321: The base snapshot (djs-prelude)
- D1430: Privileged code keeps no line tables or local names
- D1631: Native frames are ordinary frame records with a header bit
- D1632: The bgen Rust backend: states are basic blocks, calls go through call_intrinsic
- D1633: Fuel per transition; the backend is part of the engine's identity
- D1634: Code size: the native API is out of line in wasm
- D1635: Both backends are tested
- D1801: Dense-array leaves for the element loops of Array builtins
- D1850: Leaf fast paths for everyday builtins (`@fast`)
- D1900: Native builtins collect and retry at the heap cap
- D1910: Dense-array loops, fromCharCode and the URI functions as `@fast`; entries decline at the heap cap
- D2000: Typed locals of native builtins live in Rust variables, written through
- D2020: `@fast` builtins are not also `@native`, except Array.prototype.push
- D2060: The base is decoded or injected straight into the first VM's arena
- D2090: Fused dense HasProperty + Get in natives; one-pass string search and in-arena joins
<!-- /gen -->
