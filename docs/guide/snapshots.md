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

A snapshot is not an image: an image is a copy of a VM's memory that stays inside one
build and one process, such as the base image every VM starts from or the image a Rust
host makes of its setup ([images](images.md)). A snapshot is the form of a run that
outlives the process.

## A run that waits

On this site, a `debugger;` statement in a live example makes the run wait: with the
sandbox's `debugger` option the statement awaits a host call that the page settles when
you continue, so the run is idle there, exactly as if it waited for a tool. The page
then writes the run out as a snapshot and throws its sandbox away. Run this one, and open
its **Snapshot** tab at the pause: it shows what the snapshot holds, in JavaScript terms
first. **Continue** loads the snapshot into a fresh sandbox, which runs on to the next
`debugger;` or the end:

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
`pages.push(...)`; and the `for` loop, at `n = 1`. In the Snapshot tab, the title of a
stopped function marks where it waits in the code, each `?` says what a part is and why
the snapshot holds it, hovering a value lights up every place the same object is
reached from, and clicking one opens its record. Below the run come what the snapshot
names instead of storing, its size in words, and two layers you can open: how djs
stores it, record by record with named fields, and the bytes themselves, the selected
record marked.

### The same steps as host code

The buttons at each pause do what a host does with a run that waits. **Copy** gives the
snapshot's bytes as base64, **Download** as a file, **Link** puts them in the fragment of
a link to [this page's restore box](#restoring-a-snapshot-here), which no server sees;
**Continue** is a load and a settled call. With the JavaScript interface ([runs the host
drives](js-api.md#runs-the-host-drives)), in node:

```ts
const options = { debugger: true, console };          // the setup of this site's examples
const sandbox = await Sandbox.create(options);
const run = sandbox.begin(code, {}, { filename: "example.js" });  // to the first pause
const snapshot = run.snapshot();                      // the bytes: Download
const text = Buffer.from(snapshot).toString("base64"); // as text: Copy, Link
sandbox.dispose();                                     // the run lives on only as `text`
// ... stored anywhere: a file, a database row, a queue; read back later, elsewhere:
const again = await Sandbox.create(options);           // the same options
const resumed = again.loadSnapshot(Buffer.from(text, "base64"));
for (const call of resumed.calls) resumed.settle(call.id, { ok: true, value: undefined });
await resumed.advance();                               // Continue: to the next pause or the end
```

The command line reads the same snapshot ([the command line](cli.md#snapshots-on-the-command-line)):
decode what Copy gave, describe it, and go on from it, pausing at the next `debugger;`
and from there to the end:

```sh
$ pbpaste | base64 -d > pause.snapshot
$ djs describe pause.snapshot
$ djs resume --debugger next.snapshot pause.snapshot
got page 1 of 3
djs: paused at example.js:3:3, wrote next.snapshot
$ djs resume next.snapshot
got page 2 of 3
got page 3 of 3
10 11 20 21 30 31
```

`djs resume` gives the loaded run the host values the site's examples have, `console`
and `debugger`, so a snapshot of this page continues there; the second `resume` goes on
through the pauses after it at once.

### What the snapshot holds

`describeSnapshot` (and `djs describe`) says in text what the Snapshot tab shows, here
for the first pause as the guide's build writes it:

<!-- gen:snapshot -->
```text
(the run ended without pausing)
```
<!-- /gen -->

Read it from the top. The one open call is the pause itself. `fetchPage(1)` is stopped at
its `debugger;` with its variable `page`; the body is waiting for it at `await
fetchPage(n)`, halfway through `pages.push(...)`: what the body had evaluated of that
call, the method and the array, sits on its operand stack and travels with it. The
body's promise is the run's result, still pending. That chain, from the open call through
each activation that awaits the one before to the run's result, is what a run that waits
looks like; with `Promise.all` it fans in, and the text shows each open call's way up to
where they meet.

Then what the snapshot names instead of storing: the code of the two stopped functions,
which a loading djs must compile to the same bytecode (their hashes), the builtins every
djs has, and the host's `console` and `debugger`. Most of the bytes of a snapshot this
small name the kinds of data and their fields, which lets a later djs read it; the run's
own data is a few hundred bytes. The indented `·` lines and the record list at the end
are how djs stores it: each object, promise, activation and frame a record with its kind
and named fields (`#14 Object.Ordinary` is `page`), the frames as value arrays of callee,
`this`, arguments, a header, the variables and the operand stack ([below](#what-a-snapshot-contains)).

Each Continue resumes from the snapshot alone; nothing else of the run survives between
the parts. A later snapshot grows with the run's data (one more page each), not with the
number of times the run was resumed, because a loaded run is the same heap the original
was, not a description of it.

## Restoring a snapshot here

Paste a snapshot as base64 below, from a live example's Copy, from `base64
pause.snapshot`, or follow a Link, which fills it in. Loading it shows what it holds in
the Snapshot tab, puts its script into the editor, and loads it into a fresh sandbox of
this site's setup, from where Continue goes on. Without a snapshot, the example below
runs as any other; its two calls wait at once, so its snapshot shows how `Promise.all`
waits for both:

```js live restore
const cart = new Map([["apple", 2]]);
let total = 0;
async function add(item, n) {
  cart.set(item, (cart.get(item) ?? 0) + n);
  debugger;            // both calls wait here at once
  total += n;
}
await Promise.all([add("pear", 1), add("apple", 3)]);
console.log([...cart].join(" "), total);
```
<!-- gen:run -->
```text
apple,5 pear,1 4
```
<!-- /gen -->

A snapshot this page's djs cannot load is refused with the engine's reason, and its
contents still show: one written by another build whose waiting functions compile
differently ([later builds](#later-builds)), or one that names a host value this site
does not give, such as a snapshot of `djs run --body`, whose run has `tools` and `print`
([host values](#host-values)).

## Idle points

A snapshot is taken only at an **idle point**: the run waits for one or more async host
calls, or it has ended, its job queue is empty, nothing waits in its inbox of host
results, and no function is on the stack. There, everything in progress is an async
function stopped at an `await` or a generator stopped at a `yield`, whose frame is a
record in the heap and whose code the loading engine can find again. A run that is
computing has frames on the interpreter's stack and may be in the middle of a builtin. A
host that must persist it gives it fuel until it reaches an idle point, or keeps the last
snapshot it took, from which the run goes on the same way.

Inside one process, a run can stop anywhere, between two instructions, and be kept as an
image of its memory ([images](images.md)); a snapshot is for keeping a run beyond the
process.

## Taking and loading a snapshot

A host has two ways to take one:

- **A run the host drives** ([the JavaScript interface](js-api.md#runs-the-host-drives))
  is one whose async calls the host settles itself: while it is idle,
  `run.snapshot({ states })` writes it, and `sandbox.loadSnapshot(snapshot)` loads it
  into a fresh sandbox made with the same options and returns the run with its open
  calls.
- **The command line** writes and loads snapshot files: `djs run --snapshot-after K OUT`,
  `djs resume`, `djs describe`, and the REPL's `.snapshot` and `.restore` ([the command
  line](cli.md#snapshots-on-the-command-line)).

The open host calls travel with the run, by the ids the VM gave them, and each carries
the host's own state of the call, JSON data such as a deadline or a request id. After a
load, the host continues each open call from that state, issues it again or rejects it:
the run itself only waits for the call's outcome.

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
- the **scripts**: name, parameters, compile options, and the source text;
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

The [restore box](#restoring-a-snapshot-here) above loads snapshots with this page's
build, so a snapshot copied from an older version of this site shows which case it is:
it loads, or the box says why it does not.

A field the snapshot lacks starts at zero in the loaded cell, the state every new cell
starts in; a field the snapshot has and the loading build lacks is refused unless it
holds that zero state. A refused load leaves the sandbox unusable, and the host disposes
of it. What it does with the run then is its own decision: keep the previous build for
the runs in flight, treat the open calls as interrupted, or, for a deterministic run,
begin it again from its script with the results it had, which brings it back to where
it was in the new build ([determinism](determinism.md#when-a-snapshot-is-refused)).

Loading checks what it builds before anything runs: every count and index in the bytes,
each object's internal slots against what its class's code relies on, each suspended
frame against its code (its pc on an instruction, its registers within the frame), and
then the whole heap as a restored image is checked ([images](images.md#what-an-image-depends-on)).
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

The run goes on; the host can snapshot it at a later idle point, when the value is gone.

## Describing a snapshot

`djs describe file.snapshot` (and, with the docs module,
`sandbox.describeSnapshot(snapshot)`) describes one for people, as above. It leads with
the run in JavaScript terms: each open call, the activation that awaits it with its
source line, its parameters, variables and captured variables as JavaScript values, what
the expression or loop it waits in is doing, and on up the await chain to the run's
result; activations no open call leads to (a generator stopped at a `yield`) come after.
Then what a loading djs must have, the size in words, and the storage layer: every record
by id, kind, size and fields. Storage details stay on the indented `·` lines, so the
first reading needs none of them. Local and parameter names and source positions are not
in the snapshot: the describer compiles the stored script again, as a load does, and
reads them from its debug information; a snapshot that names its script by hash shows
its frames by their slots.

`djs describe --json file.snapshot` (and `sandbox.inspectSnapshot(snapshot)`) gives the
same as data, for tools and for this site's snapshot view: the sections with their byte
ranges, the record types, strings, builtins, host values, the scripts with their text,
the functions with their names, spans and hashes, every record with its fields by name,
its byte range, what holds it, a label, a one-line note on what it is and a JavaScript
rendering, the run as a tree of plain sentences with notes, and the sizes in words. Its
`format` is the snapshot format's version, and its shape changes with the format.

`sandbox.callSite(id)` says where the run waits on open call `id`: the file, line and
column of the `await` of the innermost activation waiting on it, directly or through
`Promise.all` and the like, in the script the run started with, also after any number of
loads. The live examples' pause lines come from it. Both are in the docs module only
([the module builds](host.md#the-module-builds)): the shipped module, which hosts persist
runs with, leaves out what only people read.

## Pauses at `debugger;`

A sandbox with the `debugger` option makes `debugger;` in an async function a pause: the
statement awaits a call of the async host function `debugger`, so the run is idle there
and can be snapshotted. The snapshot keeps the pause as any open call; a sandbox with the
option that loads it settles the call to go on, and pauses again at the next
`debugger;`. The live examples on
this site and the command line's `--debugger` work this way. Without the option,
`debugger;` does nothing.

## Where to look

- [`crates/djs-core/src/vm/snapshot/`](../../crates/djs-core/src/vm/snapshot/): snapshots. [`crates/djs-core/src/vm/snapshot/export.rs`](../../crates/djs-core/src/vm/snapshot/export.rs): the writer;
  [`crates/djs-core/src/vm/snapshot/import.rs`](../../crates/djs-core/src/vm/snapshot/import.rs): the loader;
  [`crates/djs-core/src/vm/snapshot/inspect/`](../../crates/djs-core/src/vm/snapshot/inspect/): `describe` and the JSON, the run in JavaScript terms;
  [`crates/djs-core/src/vm/snapshot/check.rs`](../../crates/djs-core/src/vm/snapshot/check.rs): the checks on load;
  [`crates/djs-core/src/vm/snapshot/hash.rs`](../../crates/djs-core/src/vm/snapshot/hash.rs): the bytecode and interface hashes;
  [`crates/djs-core/src/vm/snapshot/schema.rs`](../../crates/djs-core/src/vm/snapshot/schema.rs): the slot names of every object class;
  [`crates/djs-core/src/vm/snapshot/wire.rs`](../../crates/djs-core/src/vm/snapshot/wire.rs): the encoding.
- [`js/src/sandbox.ts`](../../js/src/sandbox.ts): `Run.snapshot`, `loadSnapshot`,
  `describeSnapshot`, `inspectSnapshot` and `callSite`.
- [`tests/snapshot/`](../../tests/snapshot/): the programs the snapshot tests stop at
  every idle point, and the pinned bytecode hashes of the corpus's waiting functions.
- [`examples/sleep-snapshot/`](../../examples/sleep-snapshot/): a complete host that
  snapshots a waiting run and continues it in another process.
