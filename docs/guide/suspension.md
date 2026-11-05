# Suspending anywhere: fuel and host calls

A codemode script may loop forever by mistake, wait minutes for a tool, or be cancelled.
The host must be able to take control back at any moment. The goal to [suspend
anywhere](goals.md#suspend-anywhere) means that execution can stop **between any two
instructions**: in the middle of a `map` callback, inside a getter, in a promise job,
three generators deep, and continue later in the same process. This chapter shows what
stops a run, what a stop looks like inside the engine, how host calls, the job queue,
generators, async functions and builtins fit in, and how a run that waits is kept beyond
the process.

> **In short.** `run` executes until its fuel is used up, a sync host function is
> called, or nothing is left to do, and then returns a status. Every piece of state is
> already in the arena, so a stop is just a return: nothing to unwind, nothing to save.
> Fuel is charged before each instruction and in proportion to the work inside builtins,
> an overshoot carries to the next run, and generators and async functions suspend by
> copying their frame into a heap object.

## What a stop looks like

The host calls `run` with an amount of fuel. It returns one of:

| status | meaning | the host then |
| --- | --- | --- |
| `Done(v)` | the call returned and nothing is left: no jobs, no pending async host calls | reads `v` |
| `Threw(e)` | an uncaught exception, of the call or of a job | reads `e` |
| `OutOfFuel` | the fuel ran out at an instruction boundary | runs again later, or gives up |
| `HostCall` | a sync host function was called; the VM is parked on it | handles it, resolves it, runs again |
| `Idle` | nothing to run, but async host calls are pending | delivers a result, runs again |
| `HostOutOfMemory` | the host allocator refused memory | discards the call; the VM is still usable |

Whatever the status, the interpreter has written its registers to the `Interp` cell, and
the arena is a complete description of the VM ([one arena](arena.md)). Copying it gives
an image, and a fresh VM restored from that image in the same process continues exactly
where the old one stopped. The examples in this guide rely on it: every output on these
pages comes from a VM that was stopped every 50 units of fuel, copied into an image, and
restored into a new VM before it went on. On the docs site, the *Suspend* tab of a live
example shows the slices of a run in your browser.

```js live
const words = ["fuel", "frames", "arena", "image"];
const lengths = words.map((w) => {
  let n = 0;
  for (const ch of w) n++;          // a loop inside a callback inside a builtin
  return n;
});
console.log(lengths.join(" "), lengths.reduce((a, b) => a + b, 0));
```
<!-- gen:run -->
```text
4 6 5 5 20
```
<!-- /gen -->

## Fuel

Fuel is a counter in the run loop. What it costs a script, and how a host sets slices and
budgets, is on [limits](limits.md#fuel); this section is about where the engine charges
it.

- **Every instruction** subtracts the fuel of its cost class (0 to 5, from the
  instruction table) before it runs, and the loop checks the counter at every
  instruction boundary. The tight inner loop for the hottest instructions charges and
  checks exactly the same way, so it is invisible in fuel.
- **Leaf intrinsics** charge for the work they do: a string copy for its units,
  `JSON.parse` for the bytes it reads, the regex matcher for each step.
- **Native builtins**, the generated Rust state machines, charge 4 per loop back-edge
  and 2 per call they make.
- **Resuming** a generator or async function charges for the slots it copies back.

So fuel bounds the *work* a slice does, not its instruction count.

An instruction is charged before it runs, so a slice can end below zero when its last
instruction was expensive. `run` stores that overshoot in the heap as a debt
(`Interp::FUEL_DEBT`), and the next run pays it first; a debt larger than the next
slice stops that run at once and carries the rest. Once the VM is done or idle nothing
is owed, since those states do not depend on how the run was sliced. The result: a
budget of a million fuel stops at the same instruction whether it is given as one run or
as a thousand slices of a thousand, with or without images in between.

> **Trade-off.** A fuel counter over preemption. Native engines stop a script from
> outside: a watchdog thread, a signal, an interrupt flag. In wasm there are no signals,
> and the only way to stop a module from another thread is to terminate it, which loses
> the run. A counter needs nothing from the platform, costs a subtraction and a compare
> per instruction, and makes the stopping point a function of the program, which
> [determinism](determinism.md) needs. The cost: fuel only approximates time, the
> weights are coarse, and every native operation must charge for its work or it becomes
> a way to stall the host. A host that needs a time limit reads its clock between slices
> ([limits](limits.md#time)).

## Safepoints

Fuel can stop a run at any instruction boundary. Garbage is collected only at
**safepoints**, the boundaries where the interpreter checks whether a collection is
pending: backward jumps, frame entry, returns, the loop back-edges of native builtins,
and the handler of an out-of-memory error. At a safepoint every live value is on the
value stack or in a heap cell; the interpreter flushes its registers, clears the stack
above its top so that popped values keep nothing alive, collects, and re-derives its
cached offsets into the code and the stack. Between safepoints allocation only bumps
([the arena](arena.md#allocation-is-a-bump)), which is what lets Rust code keep heap
values in local variables.

The start of a run is not a safepoint. A collection that became pending waits for the
next safepoint in the program, so collection points depend only on the program, never on
where a slice ended. That is why two runs with different slicing leave byte-identical
images ([the garbage collector](gc.md#when-it-runs)).

## Host calls

A **sync** host function parks the VM. The call instruction copies the arguments into a
pending host call record in the heap, pops the callee and the arguments, records which
call the VM is parked on and where to continue, and `run` returns `HostCall`. The host
reads the arguments, does whatever it needs (asynchronously on its side, if it likes),
and resolves the call. The next `run` pushes the result where a JavaScript callee's
return value would go and continues after the call instruction; an error result is
thrown at the call, so the script can catch it.

An **async** host function does not park. The call creates a promise, records the call
id, the function, the arguments and the promise in the pending host call table, and the
script continues with the promise. The host collects new calls after each `run` and
resolves them whenever their results arrive. A result does not settle its promise at
once: it goes into an **inbox** in the heap, and the run loop takes it from there only
when the job queue is empty, as the next section describes.

Both kinds are heap state, so a VM that waits for ten tools is as complete a description
of the run as any other stop. [The wasm boundary](host.md#host-calls) has the protocol
from the host's side, and [the JavaScript interface](js-api.md#host-functions) how a
`Sandbox` defines both kinds.

## The job queue

Promise reactions run as **jobs**, and so does every resumption of an async function
after an `await`. The queue is a value array hanging off the `Roots` cell; each
entry is a function and four arguments, and a job runs in a frame of its own like any
call, so it can run out of fuel halfway as well. The run loop works like an event loop:

1. When the entry call returns, `run` takes the oldest job and runs it, and so on until
   the queue is empty.
2. With the queue empty, it notes unhandled rejections, then takes the oldest result
   from the inbox and settles its promise, which queues the reactions, and goes back to
   step 1. One result at a time, each followed by everything it triggers, like an I/O
   callback.
3. With the queue and the inbox empty, it returns `Done` or `Threw` if no async call is
   pending, else `Idle`.

So what a script does depends on the order of the results, never on when they arrived
relative to the fuel slices. A host that delivers the same results in the same order
gets the same run, which [deterministic runs and replays](determinism.md) build on.

## Generators and async functions

A generator's frame must outlive the call that created it, and an async function's frame
must survive every `await`. djs does not give them stacks of their own. When a generator
yields or an async function awaits, the interpreter copies the frame's slice of the value
stack (callee, `this`, arguments, header, locals, operand stack) into a `Generator`
object in the heap, and pops the frame. Resuming copies the slice back onto the top of
the stack and rewrites the four header words that link it to its new caller. A frame
holds no absolute stack positions besides those links ([the
interpreter](interpreter.md#frames-on-the-value-stack)), so it can be resumed at any
depth.

```js
async function twice(p) {
  const x = await p;
  return x * 2;
}
return twice(Promise.resolve(21));
```

<!-- gen:disasm -->
```text
function 0 <anonymous>: params=0 locals=2 stack=5 captures=0
  handler [3, 32) -> 32 depth 0
    0  async_start loc0 (<state>)
    3  fclosure child0
    8  put_loc1 (twice)
    9  get_loc1 (twice)  ; 5:8
   10  undefined
   11  get_global "Promise" ic0  ; 5:14
   18  get_method "resolve" ic1  ; 5:21
   25  push_i8 21
   27  call1  ; 5:29
   28  call1  ; 5:13
   29  async_return loc0 (<state>)
   32  async_reject loc0 (<state>)
function 0.0 twice: params=1 locals=2 stack=2 captures=0
  handler [3, 13) -> 13 depth 0
    0  async_start loc0 (<state>)
    3  get_arg0  ; 2:19
    4  await loc0 (<state>)  ; 2:13
    7  set_loc1 (x)  ; 3:10
    8  push2
    9  mul  ; 3:12
   10  async_return loc0 (<state>)
   13  async_reject loc0 (<state>)
```
<!-- /gen -->

In `twice`, `async_start` creates the async state object in a hidden local (`<state>`).
`await` saves the frame into it and replaces itself with a call of the builtin
`%AsyncAwait`, a tail call that subscribes the state to the promise and returns the
function's result promise to whoever ran the frame: the caller the first time, a job
later. When the promise settles, a promise job resumes the state, and `await` pushes
the value. Closures capture boxes and never point into frames, so nothing else has to
change when a frame moves.

```js live
function* counter() {
  let i = 0;
  while (true) yield i++;
}
const g = counter();
console.log(g.next().value, g.next().value, g.next().value);
const squares = await Promise.all([1, 2, 3].map(async (x) => {
  await null;
  return x * x;
}));
console.log(squares.join(" "));
```
<!-- gen:run -->
```text
0 1 2
1 4 9
```
<!-- /gen -->

> **Trade-off.** Copying frames over a stack per generator. Separate stacks would make
> yield and resume free, but a stack is a large growable region per generator, and the
> collector and images would need to know about many of them. Copying keeps one value
> stack and makes a suspended generator an ordinary heap object. The cost is a copy of
> the frame on every `yield` and `await` and back on every resume, proportional to the
> frame's size, which is small for the generators and async functions scripts write.

## Builtins in the middle

The hardest stops are the ones inside builtins: a `map` over a million elements, a
`sort` with a comparator, a regular expression that backtracks. The [leaf
rule](builtins.md) handles the first two: hand-written Rust never runs user code, and a
builtin that does is either bytecode or a generated state machine whose every resume
point and loop back-edge is a place it can stop. The regex matcher keeps its
backtracking state in the arena and can suspend in the middle of a match, so
catastrophic backtracking runs out of fuel instead of hanging the host. The remaining
leaf operations (a big `join`, `JSON.parse` of a large tool result) do not stop in the
middle, but they are bounded by their input and charge fuel for it.

> **Trade-off.** Long leaf operations are not resumable. A 10 MB `JSON.parse` runs to
> completion inside one instruction and then charges its fuel, overshooting the slice.
> Making every leaf resumable would need a state machine per operation, as the regex
> matcher has. Since the overshoot is carried as debt, the budget is still exact; only
> the latency of one slice grows.

## Persisting a waiting run

Suspension costs nothing inside a process, but an image is a poor way to keep a run
beyond it: it holds the engine's own numbering (cell layouts, opcodes, internal ids),
so it restores only into a djs that numbers everything the same way, and that numbering
changes between builds ([images](snapshots.md#what-an-image-depends-on)). A run that
waits minutes for a tool or a person must survive restarts and upgrades. That is
[persistence](goals.md#persistence), and it uses a different form: a **continuation
dump**, the run written out as an ordinary JavaScript program that recreates it, in
which every suspended function is a continuation, the rest of its code from where it
waits. Any later djs, or node, runs it.

A dump is taken only at an **idle point**: `run` returned `Idle`, no job is queued and
no frame is on the stack. There, every suspended piece of the run is an async function or
generator stopped at an `await` or `yield`, which the compiler can describe by name.
A run in the middle of a computation has no such description; [idle
points](dumps.md#idle-points) says what a host does about that, and [how continuation
dumps are written](continuations.md) how the description is made.

On the docs site, a `debugger;` statement in a live example makes such an idle point:
the page binds an async host function `debugger` that the statement awaits, so the run
stops there and waits for it. The page then writes the run out, shows a pause line that
gives the line and column where it stopped, with the dump under it, and on Continue
resumes the run from the dump in a fresh sandbox. After a Continue, later pause lines
give their position in the dump that part continued from ([continuation
dumps](dumps.md) has more):

```js live
let total = 0;
for (const word of ["fuel", "frames", "arena"]) {
  total += word.length;
  console.log(word, total);
  debugger;
}
return total;
```
<!-- gen:run -->
```text
fuel 4
frames 10
arena 15
```
<!-- /gen -->

The dump at the first pause is a Script that resumes the example's code from the loop:
the `debugger` call it was waiting on comes first, as an `await` the loading host
answers, then the rest of the loop, with the values of the bindings and the iterator's
remaining state passed in as arguments:

<!-- gen:dump -->
```js
"djs-dump";
$djs.k(async function (total, $djs_i1) {
await $djs.resume(2, "debugger", []);
for (const word of $djs_i1) {
  total += word.length;
  console.log(word, total);
  debugger;
}
return total;
}, $djs.source("example.js", "7ab0490b"), 1, 22)(4, ["frames", "arena"].values());
```
<!-- /gen -->

## Where to look

- [`crates/djs-core/src/interp/run.rs`](../../crates/djs-core/src/interp/run.rs): the
  run loop, the statuses, fuel and its debt, safepoints, the job loop and the parking of
  sync host calls.
- [`crates/djs-core/src/interp/run/hot.rs`](../../crates/djs-core/src/interp/run/hot.rs):
  the inner loop for the hottest instructions, with the same fuel checks.
- [`crates/djs-core/src/interp/run/native.rs`](../../crates/djs-core/src/interp/run/native.rs):
  native builtin frames, their back-edges and fuel.
- [`crates/djs-core/src/interp/run/suspend.rs`](../../crates/djs-core/src/interp/run/suspend.rs):
  generators and async functions.
- [`crates/djs-core/src/interp/run/promise.rs`](../../crates/djs-core/src/interp/run/promise.rs):
  promises, the job queue and the host inbox.
- [`crates/djs-core/src/interp/run/json_regexp.rs`](../../crates/djs-core/src/interp/run/json_regexp.rs):
  the regex matcher's suspension in the middle of a match.
- [`crates/djs-core/src/vm/mod.rs`](../../crates/djs-core/src/vm/mod.rs): the
  embedder's API (`run`, `resolve_host_call`, images);
  [`crates/djs-core/src/vm/dump.rs`](../../crates/djs-core/src/vm/dump.rs) and
  [`crates/djs-core/src/vm/dump_cont.rs`](../../crates/djs-core/src/vm/dump_cont.rs):
  writing continuation dumps (loading one is running it).
