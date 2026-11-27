# Snapshots: keeping a waiting run

A script may wait minutes for a tool, or for a person. Keeping its sandbox in memory all
that time is wasteful, and a restart of the host would lose it. While a run only waits
for its tools, djs can write it out as a **snapshot**: the run's state as data, its
objects, its functions stopped at their `await`s and its open tool calls. Loading the
snapshot into a fresh sandbox, later and in another process, continues the run where it
stopped. This page shows what a snapshot holds, how a host takes and loads one, what a
later build of djs can load, and what cannot be snapshotted.

> **In short.** A snapshot is taken at an **idle point**, where the run has nothing to do
> but wait for host calls. It holds every object and internal record the run can reach
> beyond what a fresh sandbox has, each written by its kind and its fields' names, the
> builtins by their names, and the host's values by the names the host gave them. Code
> is not in it: the loading engine compiles the scripts again and checks every function
> the run waits in against its **bytecode hash**, so a snapshot loads into any build of
> djs that compiles those functions the same way, and is refused, with the reason, by
> one that does not.

The word also names two other things in djs, both images of a VM's memory that stay
inside one build: the base snapshot every VM starts from, and the snapshot a Rust host
makes of its setup ([images](snapshots.md)). A snapshot of a run, the subject of this
page, is the form that outlives the process.

## A run that waits

On this site, a `debugger;` statement in a live example makes the run wait: the page
gives the sandbox a host function that the statement awaits, so the run is idle there,
exactly as if it waited for a tool. The page then writes the run out as a snapshot and
throws its sandbox away. **Continue** loads the snapshot into a fresh sandbox, which runs
on to the next `debugger;` or the end:

```js live
async function fetchPage(n) {
  const page = { n, items: [n * 10, n * 10 + 1] };
  debugger;            // a tool call would wait here
  return page;
}
const pages = [];
for (let n = 1; n <= 3; n++) {
  pages.push(await fetchPage(n));
  console.log("got page", n, "of 3");
}
const items = pages.flatMap((p) => p.items);
console.log(items.join(" "));
return items;
```
<!-- gen:run -->
```text
got page 1 of 3
got page 2 of 3
got page 3 of 3
10 11 20 21 30 31
```
<!-- /gen -->

At the first pause, three things are in the middle of running: `fetchPage(1)`, stopped
at its `debugger;`; the body, stopped at `await fetchPage(n)` halfway through
`pages.push(...)`; and the `for` loop, at `n = 1`. `describeSnapshot` says what the
snapshot at that pause holds, here as the guide's build writes it:

<!-- gen:snapshot -->
```text
2306 bytes
types          1772 bytes
strings         172 bytes
builtins          7 bytes
intrinsics        1 bytes
host              5 bytes
scripts          13 bytes
functions        29 bytes
records         228 bytes
vm               72 bytes
15 strings (156 bytes of text), 6 builtins, 0 intrinsics, 1 scripts, 2 functions, 21 records
builtins: Object.prototype Promise.prototype %AsyncFunctionPrototype% Array.prototype Array.prototype.push Function.prototype
flags and constants: Object BigInt Symbol PromiseStates PromiseFlags GeneratorStates GeneratorKinds
script 0: example.js (source by hash f2e1787ddc436dfa)
host values: console debugger() (and 0 members)
host data: 
function 0:0..151 (bytecode)
function 0:0..363 (bytecode)
       1 HostCall
       2 Object.Array
       2 Object.Function
       2 Object.Generator
       1 Object.Ordinary
       3 Object.Promise
       1 Patch.Global
       1 Patch.GlobalLex
       1 Patch.SymbolRegistry
       7 ValueArray
```
<!-- /gen -->

The snapshot names the example's script by its hash, not its text: the page keeps the
script, and the loading sandbox finds it there. The two functions with frames are the
body (bytes 0 to 363, the whole example) and `fetchPage` (0 to 151), each tied to its
span of the script and its bytecode hash. The records are the run's state: the two
suspended async functions (`Object.Generator`, with their frames in value arrays), the
promises between them, the `pages` array and the `page` object with its `items`, the
closures of the body and of `fetchPage`, the open host call of the pause, and the
patches to the objects every VM has of its own ([below](#what-a-snapshot-contains)).
`console` and `debugger` are the host's values, named. Most of the bytes of a snapshot
this small are its record types: the names of the kinds and fields that let a later
build read it.

Each Continue resumes from the snapshot alone; nothing else of the run survives between
the parts. A later snapshot grows with the run's data (one more page each), not with the
number of times the run was resumed, because a loaded run is the same heap the original
was, not a description of it.

## Idle points

A snapshot is taken only at an **idle point**: the run waits for one or more async host
calls, or it has ended, its job queue is empty, nothing waits in its inbox of host
results, and no function is on the stack. There, everything in progress is an async
function stopped at an `await` or a generator stopped at a `yield`, whose frame is a
record in the heap and whose code the loading engine can find again. A run that is
computing has frames on the interpreter's stack and may be in the middle of a builtin. A
host that must persist it gives it fuel until it reaches an idle point, or reruns it from
its last tool result, which [determinism](determinism.md) makes safe.

Inside one process, a run can stop anywhere, between two instructions, and be kept as an
image of its memory ([images](snapshots.md)); a snapshot is for keeping a run beyond the
process.

## Taking and loading a snapshot

A host has three ways to take one, all on [the JavaScript interface](js-api.md):

- **A task** ([tasks](js-api.md#tasks-suspend-and-resume)) is a run the sandbox drives.
  `task.suspend()` waits for the next idle point, writes the snapshot and stops the run;
  `task.checkpoint()` writes one and lets it go on. `Sandbox.resume(snapshot, options)`
  continues it in a new sandbox made with the same options.
- **A host-driven run** ([host-driven runs](js-api.md#host-driven-runs)) is one whose
  calls the host settles itself: while it is idle, `run.snapshot({ states })` writes it,
  and `sandbox.loadSnapshot(snapshot)` loads it into a fresh sandbox and returns the run
  with its open calls.
- **The command line** writes and loads snapshot files: `djs run --snapshot-after K OUT`,
  `djs resume`, `djs describe`, and the REPL's `.snapshot` and `.restore` ([the command
  line](cli.md#snapshots-on-the-command-line)).

The open host calls travel with the run, by the ids the VM gave them, and each carries
the host's own state of the call, JSON data such as a deadline or a request id. After a
load, the host continues each open call from that state, issues it again or rejects it:
the run itself only waits for the call's outcome. An async host function says how with
its hooks (`save`, `resume`, [tasks](js-api.md#tasks-suspend-and-resume)).

The fuel the run used travels too, so a budget holds across any number of snapshots and
loads ([limits](limits.md)).

## What a snapshot contains

A snapshot is binary and describes itself. It has:

- a header with the format version;
- the **record types** it uses: each kind's name, its fields' names and types, and a
  digest of both;
- the **strings** the records use, as their contents;
- the **builtins** it refers to, by path (`Promise.prototype.then`,
  `%ArrayIteratorPrototype%.next`), and the builtin closure kinds by name
  (`PromiseResolveFunction`);
- the **host values** it refers to, by the host's names;
- the **scripts**: name, parameters, compile options, and the source text or its hash;
- the **functions** it refers to, each as its script and its span of source text, with a
  hash ([below](#code-compiled-again));
- the **records**, one for every cell the run can reach beyond the builtins, each its
  type and its fields in the type's order;
- the **VM record**: the interpreter's state, the host call table, the embedder's
  handles, and the realm's own values (`Math.random`'s state, the clock, the time zone
  offset, the module registry).

Records follow the heap's own layout where it is plain data. The generic cell kinds (a
value array, a box for a captured variable, a host call, a BigInt) are written field by
field from the engine's layout table. Objects are written by class (`Object.Promise`,
`Object.Generator`), with their prototype, their own properties in order, their elements
and their internal slots by name (`Promise [state, result, reactions, flags]`). What
is an index or a cache rather than state is left out and rebuilt on load: shapes and
property tables (the loading heap adds the properties in order and builds its own), a
Map's or Set's hash index, a RegExp's compiled program. A string is its content, whatever
the form it had in the heap (a rope, a slice); an atom is its string.

The objects every VM has of its own, the global object, the global lexical scope and
`Symbol.for`'s registry, are written as patches over a fresh VM's: the keys the run
deleted and the properties it added or changed.

Values keep their form: an integer stays an integer and an internal reference stays
internal, so the loaded heap holds the same values the original did, only at other
offsets. Nothing in a snapshot is an offset, an opcode number, an atom id or an
intrinsic number of the build that wrote it.

## Code, compiled again

Code is never stored. A closure's function, the callee of a suspended frame and every
other reference to compiled code is written as a function: its script and the span of
source text it was compiled from, what `Function.prototype.toString` returns. The loading
engine compiles every script again and finds each function by its span.

A suspended frame is stored as it is: its callee, `this`, the arguments, the frame
header, the locals and the operand stack, and the position of the instruction it resumes
at. Nothing is described by role; the frame's numbers mean the same thing on load because
the code they index is checked to be the same. So each function carries one of two
hashes:

- one with a frame in the snapshot carries its **bytecode hash**: every instruction by
  name, with its offset and operands, atoms by their strings, intrinsics by name and
  constants by value; the frame's shape (parameters, locals, stack depth, captures); the
  exception handlers; the name. Line tables, local names and inline caches are left out:
  they change no pc's meaning;
- one held only as a closure carries its **interface hash**: parameters, `length`, name,
  flags and its captured variables by name. Its code may change: the loaded closure runs
  the new build's code over the same captured variables.

A test pins the bytecode hash of every function of the test corpus that can wait, so a
change to the compiler that would refuse such snapshots is one its author sees and
decides on.

## Scripts: text or hash

By default a snapshot stores the text of every script it refers to and loads anywhere.
With `sources: false` (an option of `suspend`, `checkpoint` and `snapshot`) it stores
each script's name and a hash of its text instead, which is smaller. The loading sandbox
finds the script among the ones its process ran, or asks its `sources(name)` option for
it, which a host that keeps its scripts answers; a text with another hash is refused.

## Host values

What the host gave the sandbox is named, never written: its host functions by identity,
its namespaces, `values`, the host modules' namespaces and the JSON boundary's converting
functions. The sandbox that loads a snapshot must be made with options that give the
same names, and it loads the run into its own setup: a script's `tools` is the loading
sandbox's `tools`. A name the loading sandbox lacks is a `misuse` error before anything
is loaded.

## Later builds

A snapshot names everything a build is free to renumber, so a later build of djs loads it
when it agrees on what the snapshot names, and refuses it with the reason when it does
not:

| a later build that changed | loads the snapshot? |
| --- | --- |
| opcode numbers, intrinsic numbers, atom ids, where builtins sit in the base | yes |
| the code of a function that is not waiting (a closure, a helper) | yes: the closure runs the new code |
| a cell's layout: fields moved, added, or removed while at their zero state | yes |
| the bytecode of a function the run waits in | no: "the bytecode of the function at 120..380 of script 0, which has a stored frame, differs in this build", with both hashes |
| a builtin's path, an intrinsic's name, a field that still holds state | no, naming it |

A field the snapshot lacks starts at zero in the loaded cell, the state every new cell
starts in; a field the snapshot has and the loading build lacks is refused unless it
holds that zero state. A refused load leaves the sandbox unusable, and the host disposes
of it. What it does with the run then is its own decision: keep the previous build for
the runs in flight, or treat the open calls as interrupted.

Loading checks what it builds before anything runs: every count and index in the bytes,
each object's internal slots against what its class's code relies on, each suspended
frame against its code (its pc on an instruction, its registers within the frame), and
then the whole heap as a restored image is checked ([images](snapshots.md#what-an-image-depends-on)).
That makes a damaged snapshot a refusal rather than a wrong run, but it is not a security
boundary: a snapshot is trusted input, and a host loads only snapshots it wrote.

> **Trade-off.** Records with named fields over an image of the heap. An image would
> capture any stop, including one in the middle of a builtin, and restore in a copy, but
> it holds the exact layouts, opcodes and internal ids of the build that wrote it, which
> change with nearly every change to the engine. Naming the fields and builtins and
> checking the code by hash keeps a snapshot loadable across every change that leaves
> the waiting functions' bytecode alone. The cost: snapshots exist only at idle points,
> a change to how a waiting function compiles refuses the snapshots that wait in it, and
> every kind of internal state needs a name.

## What cannot be snapshotted

Snapshotting refuses, with a `misuse` error that says what it met and where, a run that
holds:

- a `WeakRef` or a `FinalizationRegistry`: which of their targets would still be alive
  depends on the collector, and the snapshot would have to decide it;
- a builtin it changed: a property of `Array.prototype` written, a method replaced. The
  loading engine has its own builtins, and the snapshot names them rather than writing
  them. In a sandbox created with `frozenBuiltins: true` such a write fails where the
  script makes it, as on a frozen object, instead of a refusal later
  ([frozen builtins](builtins.md#frozen-builtins));
- a host value the host has no name for.

A task's `suspend` rejects in such a case and the task goes on; the host can snapshot it
at a later idle point, when the value is gone.

## Describing a snapshot

`sandbox.describeSnapshot(snapshot)` (and `djs describe file.snapshot`) describes one for
people, as above: its size and sections, the scripts, the host values it names, the
host's data (the open calls' states and the fuel used), the builtins and functions it
refers to (those it holds frames of marked `bytecode`), and its records by kind.

`sandbox.callSite(id)` says where the run waits on open call `id`: the file, line and
column of the `await` of the innermost activation waiting on it, directly or through
`Promise.all` and the like, in the script the run started with, also after any number of
loads. The live examples' pause lines come from it.

## Pauses at `debugger;`

A sandbox with the `debugger` option, a host function, makes `debugger;` in an async
function a pause: the statement awaits a call of that function, so the run is idle there
and can be snapshotted. The snapshot keeps the pause as any open call, and a sandbox
with the option that loads it continues on to the next `debugger;`. The live examples on
this site and the command line's `--debugger` work this way. Without the option,
`debugger;` does nothing.

## Where to look

- [`crates/djs-core/src/vm/snapshot/`](../../crates/djs-core/src/vm/snapshot/): snapshots. [`crates/djs-core/src/vm/snapshot/export.rs`](../../crates/djs-core/src/vm/snapshot/export.rs): the writer;
  [`crates/djs-core/src/vm/snapshot/import.rs`](../../crates/djs-core/src/vm/snapshot/import.rs): the loader and `describe`;
  [`crates/djs-core/src/vm/snapshot/check.rs`](../../crates/djs-core/src/vm/snapshot/check.rs): the checks on load;
  [`crates/djs-core/src/vm/snapshot/hash.rs`](../../crates/djs-core/src/vm/snapshot/hash.rs): the bytecode and interface hashes;
  [`crates/djs-core/src/vm/snapshot/schema.rs`](../../crates/djs-core/src/vm/snapshot/schema.rs): the slot names of every object class;
  [`crates/djs-core/src/vm/snapshot/wire.rs`](../../crates/djs-core/src/vm/snapshot/wire.rs): the encoding.
- [`js/src/sandbox.ts`](../../js/src/sandbox.ts): `Task.suspend`, `Run.snapshot`,
  `Sandbox.resume`, `loadSnapshot`, `describeSnapshot` and `callSite`.
- [`tests/snapshot/`](../../tests/snapshot/): the programs the snapshot tests stop at
  every idle point, and the pinned bytecode hashes of the corpus's waiting functions.
- [`examples/sleep-snapshot/`](../../examples/sleep-snapshot/): a complete host that
  suspends a run into a snapshot and resumes it in another process.
