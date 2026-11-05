# Suspending anywhere: fuel and host calls

A codemode script may loop forever by mistake, wait minutes for a tool, or be cancelled.
The host must be able to take control back at any moment. The goal to [suspend
anywhere](goals.md#suspend-anywhere) is that execution can stop **between any two
instructions**: in the middle of a `map` callback, inside a getter, in a promise job,
three generators deep, and continue later in the same process. Keeping a run beyond the
process is a different, narrower thing: it happens only while the run waits for its
tools ([persistence](goals.md#persistence), at the end of this chapter). This chapter
shows what stops a run, what a stop looks like, and how generators, async functions and
builtins fit in.

> **In short.** `run` executes until its fuel is used up, a sync host function is
> called, or there is nothing left to do, and then returns a status. Because the
> previous chapters put every piece of state in the arena, returning is all a stop
> takes: there is nothing to unwind and nothing to save. Fuel is a weighted instruction
> count, and an overshoot carries to the next run, so a budget stops at the same
> instruction however it is sliced. Generators and async functions suspend by copying
> their frame into a heap object.

## What a stop looks like

The host calls `run` with an amount of fuel. It returns one of:

| status | meaning | the host then |
| --- | --- | --- |
| `Done(v)` | the call returned and nothing is left: no jobs, no pending async host calls | reads `v` |
| `Threw(e)` | an uncaught exception, of the call or of a job | reads `e` |
| `OutOfFuel` | the fuel ran out at an instruction boundary | runs again later, or gives up |
| `HostCall` | a sync host function was called; the VM is parked on it | handles it, resolves it, runs again |
| `Idle` | nothing to run, but async host calls are pending | delivers a result, runs again |
| `HostOutOfMemory` | the host allocator refused memory (D1690) | discards the call; the VM is still usable |

In every case, the interpreter's registers have been written to the `Interp` cell, and
the arena is a complete description of the VM (D1151). Copying it gives an image, and
a fresh VM restored from that image inside the same process continues exactly where the
old one stopped. The examples in this guide show it: every output on these pages was
produced by a VM that was stopped every 50 units of fuel, copied into an image, and
restored into a new VM before it continued. On the docs site, the *Suspend* tab of a
live example shows the slices of a run in your browser.

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

Fuel is a counter that every instruction decrements by its cost before it runs. The
costs come in five classes, in the instruction table:

| class | fuel | instructions |
| --- | ---: | --- |
| `Free` | 0 | pseudo-ops that never execute, `nop` |
| `Basic` | 1 | stack shuffles, locals, constants, arithmetic |
| `Access` | 2 | property, global and iterator access |
| `Alloc` | 3 | instructions that allocate a cell |
| `Call` | 5 | calls and constructs |

Work that is not one instruction is charged in proportion to what it does: a leaf
intrinsic that copies a string charges for the units it copies, `JSON.parse` for the
bytes it reads, the regex matcher for every step it takes (D1711). A `@native` builtin
charges 4 per loop iteration and 2 per call it makes (D1633). So fuel bounds the *work*
a slice does, not just its instruction count: one instruction that parses a large tool
result costs as much fuel as the work it does.

An instruction is charged before it runs, so a slice can end with the fuel below zero
when its last instruction was expensive. That overshoot is stored in the heap as a debt
(`Interp::FUEL_DEBT`) and paid first by the next run (D1740, D1712). The result: a
budget of a million fuel stops at the same instruction whether it was given as one run
or as a thousand slices of a thousand, with or without snapshots in between.

> **Trade-off.** Fuel over wall-clock time. Time is what the host cares about, but a
> time limit makes the stopping point depend on the machine, its load, and V8's mood, so
> the same script with the same inputs could time out on one run and not the next. Fuel
> makes the stopping point a function of the program. The cost: fuel only approximates
> time, the weights are coarse, and every new native operation has to charge for its
> work or it becomes a way to stall the host. Hosts that need a time limit (pi does)
> poll the clock between slices: pi runs 10,000 fuel per poll (D1492).

## Safepoints

Stopping for fuel can happen at any instruction boundary. Collecting garbage can happen
only at **safepoints**, the subset of boundaries where the interpreter checks whether a
collection is pending: backward jumps, frame entry and return, the start of a run, and
the state transitions of native builtins (D1152). At a safepoint, every live value is on
the value stack or in a heap cell, the interpreter's cached offsets into the code are
re-derived afterwards, and Rust code holds no raw values. Between safepoints allocation
only bumps (see [the arena](arena.md)), which is what lets Rust code keep heap values in
local variables.

Collection points depend only on the program, not on how it was sliced: the threshold is
in the heap and the arena's growth does not depend on fuel (D1710). That is why the
images of two runs with different slicing are byte for byte the same, which the gate
checks (D1720; [Determinism](determinism.md)).

## Host calls

A **sync** host function (`console.log` in pi, `text()`, a blocking tool) parks the VM.
The call instruction records a pending call in the heap with its arguments, saves the
registers as if a JavaScript callee had been entered, and `run` returns `HostCall`. The
host reads the arguments, does whatever it needs (asynchronously on its side, if it
likes), and resolves the call: the result is pushed where a JavaScript callee's return
value would go, and the next `run` continues after the call instruction (D1152). An
error result is thrown at the call instruction, so the script can catch it.

An **async** host function (`tools.*` in pi) does not park. The call creates a promise,
records `{call id, function, arguments, promise}` in the pending host call table, and
continues. The host collects new calls after each `run`. Their results go into an
**inbox** in the heap and are applied one at a time, each when the job queue has run
empty, like I/O callbacks in an event loop (D1374, D1670). So what a script does depends
on the order of the results, never on when they arrived relative to the fuel slices.

Both kinds are heap state, so a VM that waits for ten tools is as complete a description
of the run as any other stop. [The host interface](host.md) has the protocol from the
host's side.

## Persisting a waiting run

Suspension is free inside a process, but an image does not survive the process well: it
holds the engine's own numbering (cell layouts, opcodes, internal ids), so it restores
only into a djs that numbers everything the same way, and about half of the merges to djs
so far changed some of that numbering ([images](snapshots.md#what-an-image-depends-on)). A run that waits
minutes for a tool or a person must survive restarts and upgrades. So persistence is
something else: a **continuation dump**, the run written out as an ordinary JavaScript
program that recreates it, in which every suspended function is a continuation, the rest
of its code from where it waits (D2560, D2830). Any later djs, or node, runs it.

A dump is taken only at an **idle point** (D2561): when `run` returned `Idle`, waiting on
async host calls with no jobs queued and no frame on the stack. There, every suspended
piece of the run is an async function or generator stopped at an `await` or `yield`,
which the compiler can describe by name (the bindings in scope, the values held for the
expression under way, the loops and statements around it), and which the dump writes as
the rest of its source from there. A run that is computing in the middle of an
instruction sequence has no such description; a host that must persist it gives it fuel
to reach an idle point, or reruns it from its last tool result, which determinism makes
safe. [Images and dumps](snapshots.md) shows what a dump holds.

## Generators and async functions

A generator's frame must outlive the call that created it, and an async function's frame
must survive every `await`. djs does not give them separate stacks. When a generator
yields or an async function awaits, the interpreter copies the frame's slice of the
value stack (arguments, header, locals, operand stack) into a `Generator` object in the
heap, and pops the frame. Resuming copies the slice back onto the top of the stack and
rewrites the four header words that link it to its new caller (D1375). A frame contains
no absolute positions besides those links, so it can be resumed at any depth.

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
function's result promise to the caller. When the promise settles, a promise job resumes
the state, and `await` pushes the value. Since closures capture boxes and never point
into frames (D1083), nothing else needs to happen.

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
> collector and snapshots would need to know about many of them. Copying keeps one value
> stack and makes a suspended generator an ordinary heap object. The cost is a copy of
> the frame on every `yield` and `await` and back on every resume, proportional to the
> frame's size, which is small for the generators and async functions scripts write.

## Builtins in the middle

The hardest stops are the ones inside builtins: a `map` over a million elements, a
`sort` with a comparator, a regular expression that backtracks. The [leaf
rule](builtins.md) handles the first two: hand-written Rust never runs user code, and a
builtin that does is either bytecode or a generated state machine whose every resume
point and loop back-edge is a place it can stop (D1631, D1632). The regex engine keeps
its backtracking stack in the arena and can suspend in the middle of a match (D1332), so
catastrophic backtracking is a timeout, not a hang. The remaining leaf operations (a big
`join`, `JSON.parse` of a large tool result) do not stop in the middle, but they are
bounded by their input and charge fuel for it.

> **Trade-off.** Long leaf operations are not resumable. A 10 MB `JSON.parse` runs to
> completion inside one instruction and then charges its fuel, overshooting the slice.
> Making every leaf resumable would need a state machine per operation, as the regex
> engine has. Since the overshoot is carried as debt, the budget is still exact; only
> the latency of one slice grows. If that ever matters, the operation gets a resumable
> state of its own.

## Where to look

- [`crates/djs-core/src/interp/run.rs`](../../crates/djs-core/src/interp/run.rs): the
  run loop, statuses, fuel and safepoints.
- [`crates/djs-core/src/interp/run/suspend.rs`](../../crates/djs-core/src/interp/run/suspend.rs):
  generators and async functions;
  [`run/promise.rs`](../../crates/djs-core/src/interp/run/promise.rs): jobs, host calls
  and the inbox.
- [`crates/djs-core/src/vm/mod.rs`](../../crates/djs-core/src/vm/mod.rs): the embedder's
  API (`run`, `resolve_host_call`, images);
  [`vm/dump.rs`](../../crates/djs-core/src/vm/dump.rs) and
  [`vm/dump_cont.rs`](../../crates/djs-core/src/vm/dump_cont.rs): writing continuation
  dumps (loading one is running it).

Decisions on this page:

<!-- gen:decisions D1151 D1152 D1690 D1711 D1633 D1740 D1712 D1492 D1710 D1720 D1374 D1670 D1370 D1375 D1083 D1631 D1632 D1332 D2560 D2561 D2830 -->
- D1151: Interpreter state in the arena
- D1152: Host call protocol and fuel semantics
- D1690: Running out of host memory is not a JS out-of-memory error
- D1711: Regexp fuel can overdraw; backreferences are charged in full
- D1633: Fuel per transition; the backend is part of the engine's identity
- D1740: Fuel overshoot carries to the next run; a suspended intrinsic is charged once
- D1712: A suspended intrinsic's refund carries to the next run as a credit
- D1492: pi adapter: 10,000 fuel per interrupt poll
- D1710: GC points and raw images do not depend on the fuel slice
- D1720: Determinism checks in the gate: heap images across schedules and builds
- D1374: Async host functions: a promise plus an entry in the pending host call table
- D1670: Async host results are macrotasks: the host inbox
- D1370: The job queue, and when `run` reports Done
- D1375: Suspendable frames: frame slices in Generator objects, tail calls for await
- D1083: Captured bindings live in heap Box cells
- D1631: Native frames are ordinary frame records with a header bit
- D1632: The bgen Rust backend: states are basic blocks, calls go through call_intrinsic
- D1332: RegExp objects, suspendable matching and regex literals
- D2560: A waiting session persists as a continuation dump; heap images stay inside a process
- D2561: Dumps are taken only at idle points
- D2830: Continuation dumps become plain JavaScript
<!-- /gen -->
