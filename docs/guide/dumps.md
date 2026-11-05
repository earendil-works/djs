# Continuation dumps

A script may wait minutes for a tool, or for a person. Keeping its sandbox in memory all
that time is wasteful, and a restart of the host or an upgrade of djs would lose it.
While a run only waits for its tools, djs can write it out as a **continuation dump**: a
JavaScript program that recreates the run, its data, its functions stopped at their
`await`s and its open tool calls. Loading the dump, in a new sandbox, another process or
a later djs, continues the run where it stopped. This page shows what a dump contains,
how a host writes and resumes one, and what cannot be dumped.

> **In short.** A dump is taken at an **idle point**, where the run has nothing to do
> but wait for host calls. It is a plain Script: every suspended function is written as
> its **continuation**, an ordinary function holding the rest of its code from the
> `await` on, and the open calls are calls of a hook the host answers. Any djs loads it
> by running it; a host without djs can run it too. Dumps need frozen builtins, and a
> host stores them compressed, typically a couple of KB.

## A run that waits

On this site, a `debugger;` statement in a live example makes the run wait: the page
gives the sandbox a host function that the statement awaits, so the run is idle there,
exactly as if it waited for a tool. The page then writes the run out as a dump and
throws its sandbox away. **Continue** loads the dump into a fresh sandbox, which runs on
to the next `debugger;` or the end. The output of all the parts reads as one run, with a
line at each pause that says where it stopped: a line and column of the example in the
first part, and of the dump a later part continued from, since that part runs the dump's
text (the link shows that dump with the statement marked):

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
`pages.push(...)`; and the `for` loop, at `n = 1`. The dump says all of that in
JavaScript:

<!-- gen:dump -->
```js
"djs-dump";
$djs.k(async function (fetchPage, pages) {
{ let n = 1;
pages.push(await $djs.k(async function (page) {
await $djs.resume(1, "debugger", []);
return page;
}, fetchPage)({ n: 1, items: [10, 11] }));
console.log("got page", n, "of 3");
for (n++; n <= 3; n++) {
  pages.push(await fetchPage(n));
  console.log("got page", n, "of 3");
} }
const items = pages.flatMap((p) => p.items);
console.log(items.join(" "));
return items;
}, $djs.source("example.js", "f5e6bedc"), 1, 12)(async function fetchPage(n) {
  const page = { n, items: [n * 10, n * 10 + 1] };
  debugger;            // a tool call would wait here
  return page;
}, []);
```
<!-- /gen -->

Read it from the bottom. The Script's last expression calls the **continuation** of the
body: an async function whose code is the rest of the body from the `await` it stopped
at, given the values that rest still needs (the function `fetchPage` and the `pages`
array) as arguments. It first finishes the loop iteration under way, with `n` as it
was, and what it awaits there is the continuation of `fetchPage(1)`, written in place:
it holds only `page`, since nothing after the pause reads `n`, and its first `await` is
the open call of the pause. Then the loop goes on as written. Edit the example and run
it again to see how the dump follows: add a variable, a `Map`, a nested call.

Each Continue resumes from the dump alone; nothing else of the run survives between the
parts. The later dumps grow only with the run's data (one more page each), not with the
number of times the run was resumed: a dump of a resumed run writes its continuations
again from the original code, not from the continuations of the previous dump.

## Idle points

A dump is taken only at an **idle point**: the run waits for one or more async host
calls, its job queue is empty, and no function is on the stack. There, everything in
progress is a function stopped at an `await` (or a generator at a `yield`), which the
compiler can describe by name: the bindings in scope, the values held for the expression
under way, the loops and blocks around it. A run that is computing in the middle of an
instruction sequence has no such description. A host that must persist it gives it fuel
until it reaches an idle point, or reruns it from its last tool result, which
[determinism](determinism.md) makes safe.

Inside one process, a run can stop anywhere and be kept as an image of its memory
([images](snapshots.md)); a dump is for keeping a run beyond the process.

## What a dump contains

A dump is an ordinary strict-mode Script that starts with a directive marking it as a
dump. It holds:

- **Data** as the literals and calls that make it: objects, arrays, `Map`s, class
  instances, binary data as base64, closures that share variables as one factory that
  makes them. A value used more than once is kept on a global object `$dump` and
  referred to from there; one used once is written where it is used.
- **Functions** as their source text. A builtin is the expression that finds it
  (`Array.prototype.push`), never its contents.
- **Suspended functions** as continuations, written where they are called, each with a
  small origin record that ties it to the function it stands for. The record lets a
  later dump of the resumed run go back to the original code.
- **Open host calls** as calls of `$djs.resume(id, name, args, state)`: the call's id,
  the host's name for the function (`tools.read`), its arguments, and the host's state
  of the call.
- **The script** the run was started with, by reference: `$djs.source(name, hash,
  params)` names it by its file name and a 32-bit hash of its text instead of carrying
  it a second time. The loading host compiles that script again and checks the hash.
- **Host values** the run holds, such as a tool function it put in a variable or the
  `tools` object itself, as `$djs.host(name)`, by the name the host gave them rather
  than by where the script found them.
- **Engine state** where it matters: `Math.random`'s position in its sequence, if the
  run drew random numbers, and the id the next host call gets, through `$djs.seed(...)`.

The Script's completion value is the run's result promise, which the host takes as the
run. Nothing in a dump refers to djs's compiled code, its bytecode or its internal
numbering, so any later djs runs it as it runs any script.

> **Trade-off.** JavaScript over a binary image of the heap. An image would capture any
> stop, including one in the middle of a builtin, and restore in a copy, but it is tied
> to the exact layouts, opcodes and internal ids of the build that wrote it, which
> change often. A dump says what the run *means*, so it survives every change to the
> engine that keeps the meaning of the script's code, it can be read, and it is smaller
> once compressed. The cost: it exists only at idle points, the writer has to know how
> to re-enact every kind of in-flight builtin state, and it refuses what it cannot
> write.

## `$djs`: what the loading host provides

A dump reaches its host through one global object, `$djs`, which the host defines
before it runs the dump and deletes afterwards, together with the dump's `$dump`:

| member | what the host does |
| --- | --- |
| `seed(...)` | restores `Math.random`'s state and the next host call's id; written only where needed |
| `resume(id, name, args, state)` | opens call `id` of its function `name` again under the same id, and returns its promise |
| `source(name, hash, params)` | returns the function the script `name` compiles to, after checking the hash |
| `host(name)` | returns the host value bound under `name`: the very object the host gave the sandbox |
| the runtime helpers | small functions continuations call (to rebuild an iterator in flight, give a continuation its origin record, ...) |

The helpers are the same for every dump, so the host provides them instead of each dump
carrying them. Every djs sandbox does all of this by itself. A host without djs imports
the helpers from [`js/src/dump-runtime.js`](../../js/src/dump-runtime.js) and writes
the four hooks; [`examples/sleep-dump/`](../../examples/sleep-dump/) continues a dump
in node that way.

## Writing and resuming a dump

The [JavaScript interface](js-api.md#tasks-suspend-and-resume) has two ways to persist a
run. The simple one is a **task**: the sandbox drives the run and calls the host
functions, and `suspend()` writes the dump at the next idle point.

```ts
import { Sandbox, type AsyncHostFunction } from "@djs/host";

const sleep: AsyncHostFunction<{ due: number }> = {
  async call(ms) {                         // the script called it
    const due = Date.now() + Number(ms);
    this.state = { due };                  // the call's state: a dump keeps it
    await until(due, this.signal);
    return "slept";
  },
  async resume({ due }, call) {            // after a load, with that state
    await until(due, call.signal);
    return "slept";
  },
};
const options = { frozenBuiltins: true, asyncFunctions: { "tools.sleep": sleep } };

const sandbox = await Sandbox.create(options);
const task = sandbox.start(code, params, { filename: "job.js" });
const dump = await task.suspend();         // at the next idle point; the run stops
sandbox.dispose();

// Any time later, in any process:
const resumed = await Sandbox.resume(dump, { ...options, sources: (name) => load(name) });
console.log(await resumed.result);
```

`suspend()` waits for the next idle point, asks every open call for its state, writes
the dump and stops the run; the calls' abort signals tell the host to stop waiting for
them. `Sandbox.resume` loads the dump into a new sandbox and continues each open call:

- through its function's `resume` hook, with the state it was dumped with;
- by calling the function again, if it has no `resume` hook, which suits calls without
  side effects;
- not at all, if the function is a plain function without hooks: the sandbox cannot
  know whether calling it twice is safe, so the call rejects.

A host that wants to decide every call itself drives the run instead: `begin` runs the
script to its first idle point and lists the open calls, `Run.dump(states)` writes the
dump with the states the host gives, and `loadRun` loads one and returns the run with
its open calls and their states. For each, the host delivers a result it recorded,
issues the call again, or rejects it ([host-driven runs](js-api.md#host-driven-runs)).

The command line does the same with files: `djs run --dump-after K out.js` writes a
dump at the Kth idle point, `djs resume out.js` continues it, and `djs resume --explain`
describes what a dump holds ([the command line](cli.md#dumps-on-the-command-line)).

### The host's state of a call

What is not in the VM is not in the dump by itself: a timer, a request in flight, a
person's pending answer. Those belong to the host, so each open call can carry a piece of
host state, JSON that the dump writes next to the call and the load gives back: when a
sleep is due, which request to poll, which message the answer will come in. The engine
only checks that the text is JSON, so a state cannot smuggle code into the dump.

### The script, by reference

A dump names the script it continues by `$djs.source(name, hash, params)`. A sandbox
knows the scripts the sandboxes of its process ran; one in another process gets them from
the `sources` option, which returns the code of the script `name` (the `filename` the
run was started with). A text whose hash differs, or that was compiled with other
parameters or options, is refused, because the continuations would not fit it. A host
that would rather not keep scripts sets `inlineSources`, and its dumps carry the text.

## Storing dumps

A dump is plain text, and it compresses well: hosts store it compressed with zstd at
level 3, which node has built in. Measured over 300 real codemode scripts and their
1,063 idle points:

| | median | 99th percentile |
| --- | ---: | ---: |
| dump text | 6 KB | |
| zstd level 3 | 1.7 KB | 17 KB |

A run that holds a large log is about as large as the log: one holding 1.8 MB of text
dumps to 1.8 MB and 394 KB compressed. Writing a dump takes about 0.4 ms at the median in
wasm (3 ms at the 99th percentile), and resuming one, a new sandbox included, under a
millisecond.

A dump is code: a host loads only dumps it wrote or trusts, into a fresh sandbox.

## What a dump needs

**Frozen builtins.** Only a sandbox made with `frozenBuiltins` can write dumps
([frozen builtins](language.md#frozen-builtins)). The writer relies on no builtin
changing between the moment the run read a value and the load: it writes a builtin the
run looked up as the expression that finds it again, and a method a half-done call had
already read (`pages.push` above) as a second read of it. Without frozen builtins,
writing a dump throws a `misuse` error that says so. The live examples with `debugger;`
run with frozen builtins for this reason.

**The dumper.** The code that writes dumps nearly doubles the module (about 340 KB more
with gzip), and only hosts which persist runs need it, so the module djs ships leaves it
out. Such a host loads the same
build with the dumper as a second module and passes it as `dumpModule`: to dump, the
sandbox copies the VM's memory into an instance of that module and writes the dump
there, and the process keeps that instance for the next dumps. **Loading** a dump needs
neither the dumper nor frozen builtins: it is running a Script.

## What cannot be dumped

The writer refuses what it cannot write as JavaScript that recreates it, and says what
and where, by a path from the run's roots. The run goes on; only the dump fails. The
refusals that a script can meet:

- **Weak collections** (`WeakMap`, `WeakSet`): their contents are not ordinary state of
  the run but depend on what else is still reachable, and nothing in JavaScript can
  list them.
- **Objects carrying a class's private fields or methods**, created by the class's own
  construction: only that construction can add a private name to an object, so no other
  code in a dump could make the object again. Classes with static private members are
  fine, as long as a static private field holds the value its literal initializer gave
  it.
- **A revoked `Proxy`**, and a `for in` loop in progress over a `Proxy`, whose remaining
  keys would have to come from calls of its traps.
- **Builtin state in flight that has no JavaScript form**, such as a `yield*` delegation
  in progress or a then-reaction on a promise of another constructor.

```js live
const seen = new WeakMap();
const job = { id: 7 };
seen.set(job, "started");
console.log("before the pause");
debugger;
console.log("after:", seen.get(job));
```
<!-- gen:run -->
```text
before the pause
after: started
```
<!-- /gen -->

On this site, the run then continues in the same sandbox. A host that gets the
refusal from `suspend()` decides what to do: keep the run in memory, or let it reach a
later idle point where the value is gone.

## Where to look

- [`js/src/sandbox.ts`](../../js/src/sandbox.ts): tasks (`start`, `suspend`,
  `Sandbox.resume`), `begin`, `Run.dump`, `loadRun`, and the dump module.
- [`js/src/dump-runtime.js`](../../js/src/dump-runtime.js): the helpers a dump calls,
  for hosts without djs.
- [`examples/sleep-dump/`](../../examples/sleep-dump/): a host with a tool that sleeps,
  suspended in one process and continued in another, or in node.
- [`crates/djs-core/src/vm/dump.rs`](../../crates/djs-core/src/vm/dump.rs): the writer;
  [how continuation dumps are written](continuations.md) explains how it reads frames
  and writes continuations.
- [`tests/dump/`](../../tests/dump/): golden dumps that every later djs must still load.
