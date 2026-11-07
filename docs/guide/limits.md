# Limits and resource control

A script may loop forever, allocate without end or recurse without a base case, by
mistake or on purpose. The host decides how much a run may take, and djs enforces it
without trusting the script: work is measured in **fuel**, the heap has a **cap**, the
call stack has a **depth limit**, and the compiler bounds how deeply code may nest. This
page says what each limit is, what a script or a host sees when it is reached, and how a
host keeps control of time.

> **In short.** Every instruction costs fuel, and so does the work inside builtins; a
> run goes in slices of fuel, between which the host gets control back, and a budget
> ends it. Reaching the heap cap is a catchable `InternalError: out of memory`, with a
> reserve so the handler can run, or, if the host asks for it, the end of the run. Deep
> recursion is a `RangeError`, deep nesting in source a `SyntaxError`. None of these can
> hurt the host: they are errors in the run, and the sandbox stays usable.

## Fuel

Fuel is djs's unit of work. Every instruction costs fuel before it runs, by its class:

| class | fuel | instructions |
| --- | ---: | --- |
| `Free` | 0 | pseudo-instructions that never execute |
| `Basic` | 1 | stack shuffles, locals, constants, arithmetic |
| `Access` | 2 | property, global and iterator access |
| `Alloc` | 3 | instructions that allocate a cell |
| `Call` | 5 | calls and constructs |

Work that is not one instruction is charged in proportion to what it does: a builtin
that copies a string charges for the characters it copies, `JSON.parse` for the bytes it
reads, the regular expression matcher for every step it takes, and a builtin compiled to
native code charges 4 per loop iteration and 2 per call it makes. So fuel bounds the
*work* a run does, not its instruction count: one instruction that parses a large tool
result costs as much as the parsing. As a rough scale, a million units are a few
milliseconds of a tight loop in wasm.

An expensive instruction can take a slice below zero. That overshoot is carried as a
debt and paid first by the next slice, so a budget stops at the same instruction however
it is sliced.

> **Trade-off.** Fuel over wall-clock time. Time is what the host cares about, but a time
> limit makes the stopping point depend on the machine and its load, so the same script
> with the same inputs could stop on one run and not the next. Fuel makes the stopping
> point a function of the program, which [determinism](determinism.md) needs. The cost:
> fuel only approximates time, the weights are coarse, and every native operation must
> charge for its work, or it becomes a way to stall the host.

## Slices and budgets

A host never hands djs its thread for longer than it wants: the VM runs a **slice** of
fuel and returns. In [the JavaScript interface](js-api.md#limits), `fuelSlice` sets the
slice (a million units by default) and `maxFuel` the budget of one run, which fails with
a `fuel` error when it is used up: `out of fuel after 10000000 units`. The run is
abandoned, and the sandbox stays usable. Fuel is counted over the whole run, across host
calls and promise jobs; the budget is checked between slices, so it is exact to the
slice.

A run that waits on async host calls uses no fuel while it waits.

## Time

djs has no clock of its own, so a wall-clock limit is the host's: between two slices it
looks at the time and decides whether to go on. A host that drives the engine directly
does this in its run loop; at the level of the wasm module, the run is abandoned with
one call, and the VM is ready for the next. With the `Sandbox`, `maxFuel` is the
budget, and a task that should end early is ended by disposing of its sandbox. A host
that polls often keeps its latency low: a slice of 10,000 fuel is about <!--n fuel/slice-us .0-->13<!--/n--> microseconds
of work in a tight loop in wasm.

## Memory

A sandbox's heap has a **cap**, `memoryLimit` (4 GiB, the whole wasm32 address space,
by default). When a script reaches it, it gets an exception it can catch:

```js
const chunks = [];
try {
  for (;;) chunks.push("x".repeat(1 << 20));
} catch (e) {
  chunks.length = 0;          // drop what filled the heap
  console.log(String(e));     // InternalError: out of memory
}
```

A reserve, 1/32 of the cap and at most 1 MiB, is held back from the script, so that the
handler can run and a collection can make room. Before the error reaches the script, the
step that failed collects garbage and tries once more, so the error means that live data
fills the cap. Since the cap is checked against the heap's own size, the point at which
it is reached is a function of the program, like everything else in a run. An uncaught
out-of-memory error ends the run as an ordinary exception (kind `threw`).

A host can also make the cap final. With that option, used by every VM of the
[Rust interface](rust-api.md#limits), reaching the cap stops the run at once, as host
memory exhaustion does below: no `catch` or `finally` block runs, the call is abandoned,
and the VM is idle and usable. A script that must not decide anything after running out
of memory, such as a policy whose answer would otherwise come from its error handler,
cannot swallow it then.

The host's own memory is a different matter: if wasm memory cannot grow below the cap
(the machine is out of memory, or the cap is larger than the host can back), the run
stops with a `memory` error, `out of host memory`, which is not a JavaScript error and
not deterministic. The sandbox stays usable. A host that wants out-of-memory to be
deterministic chooses a cap it can always back.

## Stack depth

JavaScript calls do not use the native stack: their frames live in the heap, on a value
stack of at most 2^20 slots (8 MiB). A recursion that fills it throws a catchable
`RangeError: Maximum call stack size exceeded`. With a small heap cap, the heap may run
out first, and the error is out of memory instead. `JSON.stringify` limits nesting to
5,000 levels in the same way, and `JSON.parse` rejects deeper input as a `SyntaxError`:

```js live
let deep = [];
for (let i = 0; i < 6000; i++) deep = [deep];
try {
  JSON.stringify(deep);
} catch (e) {
  console.log(String(e));
}
```
<!-- gen:run -->
```text
RangeError: Maximum call stack size exceeded
```
<!-- /gen -->

## Nesting in source

The compiler does recurse on the native stack, which in wasm is a fixed 128 KiB. To keep
any input within it, the parser limits how deeply code nests (brackets, blocks,
expressions, patterns), weighting each construct by the stack it takes. Deeper code is a
`SyntaxError: stack overflow` with its position, before anything runs. Code that people
or models write is far from the limit; it exists for generated or hostile input.

## When the engine itself fails

None of the limits above can stop the host: they end a run with an error, and the
sandbox goes on. What remains is a bug in djs. Every access the engine makes stays
inside its own wasm instance, so a bug cannot corrupt the host's memory. Most stop the
engine with a wasm trap, which the `Sandbox` reports as a `trap` error with the engine's
message. The shipped modules do not bounds-check heap accesses themselves (wasm confines
them to the instance), so a bug can also leave that VM's state wrong instead of
trapping; native and debug builds check every access and panic. The sandbox is unusable
afterwards and should be discarded; every sandbox has its own instance, so no other is
affected. Should anything still exhaust the engine's 128 KiB wasm stack, the sandbox
reports `stack overflow: the engine's wasm stack is exhausted` rather than a bare memory
access fault.

## Where to look

- [`crates/djs-core/src/interp/run.rs`](../../crates/djs-core/src/interp/run.rs): the
  run loop, fuel and its debt, the stack limit.
- [`crates/djs-core/src/heap/mod.rs`](../../crates/djs-core/src/heap/mod.rs): the cap
  and its reserve.
- [`crates/djs-core/src/compiler/parser.rs`](../../crates/djs-core/src/compiler/parser.rs):
  the nesting limit.
- [`js/src/sandbox.ts`](../../js/src/sandbox.ts): `fuelSlice`, `maxFuel`,
  `memoryLimit`, and how traps are reported.
