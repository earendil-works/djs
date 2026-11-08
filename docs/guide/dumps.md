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

## The dump format

This section is for a host that loads dumps itself, with or without djs: what the
Script looks like, line by line, and what each name it calls must be.

### The shape of the Script

A dump is a strict-mode Script. Its statements come in this order:

1. **The header**, the directive `"djs-dump";`. It marks the text as a dump and is
   otherwise an ordinary directive. A run whose script was TypeScript is written as
   JavaScript, its types blanked; only TypeScript that is more than types (an `enum`, a
   `namespace`) stays, and then the header is `"djs-dump typescript";` and the Script
   has to be compiled as TypeScript.
2. **`$djs.seed(...)`**, only where the run's engine state needs it: before any line that
   could draw a random number.
3. **`globalThis.$dump = {};`**, only when a value is used more than once.
4. **The data and the continuations**, in an order where every line finds what it uses
   already made. Data is the lines `$dump.n4 = ...` that make the shared values, the
   run's global bindings as declarations (`var`, `function`, `class`) of the Script, and
   the patches that close cycles. A continuation is called where the activation it
   stands for was: as an argument, an element of the array a `Promise.all` gets, or a
   statement of its own. Its prefix runs while the Script runs, up to the `await` the
   original waits at, so when the Script ends the loaded run waits exactly where the
   dumped one did.
5. **The completion value**, the last statement: the run's result promise, which the
   host takes as the run. A host that named several values to keep when it wrote the
   dump gets an object of them by name, `({ __proto__: null, run: ..., other: ... })`;
   one that named none gets `void 0`.

The Script has no `"use strict"`, because djs compiles all code strict. Another engine
needs the directive put in front.

Here two calls of `attempt` wait at once, the run holds `console.log` in a variable, an
object is reached from two places, and `Math.random` has been called:

```js live
const log = console.log;              // a host value, kept in a variable
const job = { id: 7, step: "fetch" };
const byId = new Map([[job.id, job]]);  // the same object, reached twice
const jitter = Math.random();          // Math.random's sequence has moved on
async function attempt(name) {
  debugger;                            // the run waits here, as on a tool call
  return name + " ok";
}
const results = await Promise.all([attempt("a"), attempt("b")]);
log(results.join(", "), byId.get(7) === job, jitter < 1);
return results;
```
<!-- gen:run -->
```text
a ok, b ok true true
```
<!-- /gen -->

<!-- gen:dump -->
```js
"djs-dump";
$djs.seed(-29772990, 66527711);
globalThis.$dump = {};
$dump.n3 = async function attempt(name) {
  debugger;                            // the run waits here, as on a tool call
  return name + " ok";
};
$dump.n8 = { id: 7, step: "fetch" };
$djs.k(async function (log, job, byId, jitter) {
const results = await Promise.all([$djs.k(async function (name) {
await $djs.resume(1, "debugger", []);
return name + " ok";
}, $dump.n3)("a"), $djs.k(async function (name) {
await $djs.resume(2, "debugger", []);
return name + " ok";
}, $dump.n3)("b")]);
log(results.join(", "), byId.get(7) === job, jitter < 1);
return results;
}, $djs.source("example.js", "c8828614"))($djs.host("console.log"), $dump.n8, new Map([[7, $dump.n8]]), 0.05279087335850807);
```
<!-- /gen -->

Line by line:

- `$djs.seed(-29772990, 66527711)` puts `Math.random` back where the run left it, since
  the run drew a number. It comes first, before anything could draw another.
- `$dump.n3` is `attempt`, written once as its source text because two continuations
  name it as their origin. `$dump.n8` is `job`, which the body holds and the `Map` holds
  too: written once, it stays one object after the load.
- The last statement calls the body's continuation, an async function whose parameters
  are the bindings the rest of the body reads. Its arguments are the values: `log` is the
  host's function `console.log`, found again by `$djs.host`, the `Map` is written where
  it is used, and `jitter` is a number. `$djs.k(K, origin)` attaches the origin record,
  here the host's script by reference, and returns `K`.
- Inside it, `Promise.all` gets the two continuations of `attempt`, each awaiting its
  open call, `$djs.resume(1, ...)` and `$djs.resume(2, ...)`, in the order the calls
  were made. When the Script ends, both calls are open again and the body waits on the
  `Promise.all`, as before the dump.
- The Script's value is the body's promise: the run.

### `$djs`: what the loading host provides

A dump reaches its host through one global object, `$djs`, which the host defines
before it runs the dump and deletes afterwards, together with the dump's `$dump`. It
has two kinds of members: four **hooks**, which only the host can answer, and the
**runtime helpers**, small fixed functions that the host provides so that dumps do not
carry them. Besides `$djs`, the dump's code reads the same globals the run read: the
host's functions and namespaces bound as globals (a `tools` object, a `console`), under
the same names.

Every hook is called synchronously while the Script runs and must return at once.

**`$djs.seed(lo, hi, next)`** restores engine state. `lo` and `hi` are the low and high
halves, as signed 32-bit integers, of the 64-bit state of `Math.random`'s xorshift64\*
generator; `next` is the id the run's next host call gets. Either part is written only
where it is needed: `seed(lo, hi)` when the next id is one past the highest id the dump
reopens (the default a host assumes), `seed(next)` for the id alone. A host that cannot
set its engine's `Math.random` ignores the first two: the run goes on with other random
numbers than the uninterrupted run would draw.

**`$djs.resume(id, name, args, state)`** reopens an open host call and returns its
promise.

- `id` is the number the run gave the call. Host calls are numbered from 1 in the order
  the run made them, sync calls such as `console.log` included, and a host that answers
  calls by id keeps it.
- `name` is the host's name for the function, its identity: `"tools.read"`, or
  `"debugger"` for a pause on this site.
- `args` is the array of the call's arguments, written as data like the rest of the
  run, so an argument that is also held elsewhere is the same object.
- `state` is the JSON value the host gave the call when the dump was written, as a
  literal, and left out when the call has none.

The hook returns a promise of the realm's own `Promise`, which settles with the call's
outcome once the host answers it. The dump awaits it, passes it to a combinator or calls
`then` on it, as the run did with the original call's promise.

**`$djs.source(name, hash, params, wrapper)`** returns the function the host's script
compiles to, the origin of the continuation of the script's body. `name` is the file
name the run was started under, `params` the body's parameter names joined by commas
(left out when it has none), and `hash` eight hex digits: FNV-1a 64 over the UTF-8 of the
parameters, a NUL byte, the compile options as a decimal number, a NUL byte and the
script's text, folded to 32 bits by XOR of its halves (`scriptHash` in
[`js/src/sandbox.ts`](../../js/src/sandbox.ts)). A djs host compiles the script it has
under that name, checks the hash and returns the body. A fourth argument, `1`, marks a
script that is a wrapper, one parenthesized function expression such as
`(async (tools, console) => { ... })`, as some hosts compile their bodies: the hook then
returns the function that evaluating the script gives. The dump never calls the function:
it is the origin a later dump reads the code from. A host that never dumps the run again
may return any function. A dump written with `inlineSources` has no `$djs.source`: the
body's text is in the dump as a function expression in its place.

**`$djs.host(name)`** returns the host value the run held under `name`: a host function
by its identity (`"tools.read"`, `"console.log"`), or a namespace object of host
functions (`"tools"`, `"console"`). It must be the very object the run's globals and
parameters give, since the run may compare them or keep them in a `Map`.

**The runtime helpers** are the same for every dump. Each is a plain function that uses
neither `this` nor another member of `$djs`, so a continuation copies the ones it calls
into locals (`const $djs_L = $djs.L;`) and keeps working after the host deletes `$djs`.
They are defined once, in [`crates/djs-core/src/vm/dump_runtime.rs`](../../crates/djs-core/src/vm/dump_runtime.rs),
which is also written out as [`js/src/dump-runtime.js`](../../js/src/dump-runtime.js):
`helpers` there is the object of all of them, `script` the same as a Script whose value is
that object, and `scriptFor(dump)` a Script of only the helpers a given dump names.

| helper | what it is for |
| --- | --- |
| `R`, `G`, `C` | `Reflect.apply`, `Reflect.get`, `Reflect.construct`: a call, a property read or a `new` whose callee, receiver or key the run had evaluated before it stopped |
| `D(o, k, v)`, `DD(o, src)`, `A` | go on with an object literal the run had started: define a property, copy the methods and accessors of a literal of their own, a spread (`Object.assign`) |
| `Q(a, ...v)` | append to an argument list or array literal the run had started |
| `P(k)`, `S(o, excluded)` | a computed key converted as the language converts it; a pattern's rest property |
| `L(it, next)`, `I(it, next)`, `E()` | iterables over a saved iterator and its `next`: with a placeholder first step, for the rest of the iteration in flight; without one; and one that is done |
| `N`, `X`, `O`, `fromSync`, `closeThrow` | a `for await` in flight: its iterator, closing it, the check of a step's result, and the wrapper over a sync iterator |
| `Y(it, next, a, ...first)` | the inner iterator of a `yield*` in flight |
| `U(value, method, kind)` | a `using` resource, added again with its saved dispose method |
| `advance(it, n)`, `forIn(o, keys)`, `regexpIterator(re, s, f)` | an iterator stepped `n` times, the keys a `for in` has left, a `matchAll` iterator |
| `M`, `FM`, `CC`, `CI`, `HD` | iterator helpers in flight: a callback with its counter shifted, a `flatMap` inside its inner iterator, an `Iterator.concat` inside an item, a concat item, a finished helper |
| `fromAsync(...)` | an `Array.fromAsync` waiting at an `await`, its closure made again from its state |
| `spent(kind)`, `rejected(e)` | a resolving function whose promise is settled; a rejected promise, marked handled |
| `bytes(base64)` | an `ArrayBuffer` from base64 |
| `fn(o)` | the method, getter or setter of a one-property literal, as the function it is |
| `keep(o, keys)` | delete the own properties of a class that the run had deleted |
| `k(K, f, at, r, h, p)` | attach a continuation's origin record and return `K` |

`$djs.k` deserves a word, because every continuation passes through it. `f` is the
original function: a value of the dump, or `$djs.source(...)`. `at` is the site the
continuation stands for, a number for the nth `await` and a name such as `"yield#1"`
otherwise; `r` and `h` record how the run's values shaped the continuation's text; `p` is
a generator continuation's prototype where it is not `f.prototype`. Arguments at their
defaults are left out at the end (`at` 1, `r` 0, `h` empty). The helper stores
`[f, at, r, h]` as a hidden property of `K` and gives a generator continuation the
original's prototype, so `instanceof` and `Object.getPrototypeOf` see the original
function. The record means nothing to the loader; it is there for the next dump
([origin tracking](continuations.md#origin-tracking)).

### Loading a dump without djs

Every djs sandbox does all of this by itself. A host without djs loads a dump in five
steps:

1. Make a fresh realm, or a fresh process, and bind the host's functions and namespaces
   under the names the run had: the dump's code calls them by those names.
2. Define `globalThis.$djs` as the runtime helpers plus the four hooks.
3. Run the dump as a strict Script and keep its completion value, the run's promise.
4. Delete `$djs` and `$dump`. The prefixes of the continuations have run by then, and
   the run waits at its `await`s.
5. Answer the reopened calls as they finish: each `resume` promise settles, and the run
   goes on as it would have.

In node:

```js
import { Script } from "node:vm";
import { helpers } from "@earendil-works/djs/dump-runtime";

export function loadDump(text, { tools, reopen }) {
  const host = { tools, console, "console.log": console.log };
  for (const [name, f] of Object.entries(tools)) host[`tools.${name}`] = f;
  globalThis.tools = tools;
  globalThis.$djs = {
    ...helpers,
    seed() {},                          // node's Math.random has no seed to set
    resume: (id, name, args, state) => reopen(name, args, state),  // a Promise
    source: () => async function body() {},  // only an origin: never called
    host(name) {
      if (!Object.hasOwn(host, name)) throw new Error(`no host value ${name}`);
      return host[name];
    },
  };
  try {
    return new Script(`"use strict";\n${text}`, { filename: "dump.js" }).runInThisContext();
  } finally {
    delete globalThis.$djs;
    delete globalThis.$dump;
  }
}
```

`reopen` decides what an open call becomes: it can wait for the rest of a timer whose
due time the state holds, poll a request by an id the state names, or call the tool
again. [`examples/sleep-dump/`](../../examples/sleep-dump/) continues a dump this way.
A few helpers call recent builtins (`Iterator.concat`, `Uint8Array.fromBase64`, the
iterator helpers); an engine without them still loads every dump that does not name
those helpers.

## Writing and resuming a dump

The [JavaScript interface](js-api.md#tasks-suspend-and-resume) has two ways to persist a
run. The simple one is a **task**: the sandbox drives the run and calls the host
functions, and `suspend()` writes the dump at the next idle point.

```ts
import { Sandbox, type AsyncHostFunction } from "@earendil-works/djs";

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
level 3, which node has built in. Measured over <!--n dumps/corpus-cases-->300<!--/n--> real codemode scripts and
their <!--n dumps/idle-points-->1,063<!--/n--> idle points, with a dump at each:

| | median | 99th percentile |
| --- | ---: | ---: |
| dump text | <!--n dumps/raw-p50 kb.1-->2.6 KB<!--/n--> | <!--n dumps/raw-p99 kb.1-->12.4 KB<!--/n--> |
| zstd level 3 | <!--n dumps/zstd-p50 kb.1-->1.0 KB<!--/n--> | <!--n dumps/zstd-p99 kb.1-->3.0 KB<!--/n--> |
| writing the dump | <!--n dumps/write-ms-p50 .2-->0.26<!--/n--> ms | <!--n dumps/write-ms-p99 .1-->1.7<!--/n--> ms |
| resuming it, a new sandbox included | <!--n dumps/resume-ms-p50 .2-->0.39<!--/n--> ms | <!--n dumps/resume-ms-p99 .1-->2.5<!--/n--> ms |

Times are in wasm. A run that holds a large log is about as large as the log: one that
keeps a 20,000-line log across its tool calls dumps to <!--n dumps/log-raw mb-->1.8 MB<!--/n--> and
<!--n dumps/log-zstd kb-->403 KB<!--/n--> compressed.

A dump is code: a host loads only dumps it wrote or trusts, into a fresh sandbox.

## What a dump needs

**Frozen builtins.** Only a sandbox made with `frozenBuiltins` can write dumps
([frozen builtins](language.md#frozen-builtins)). The writer relies on no builtin
changing between the moment the run read a value and the load: it writes a builtin the
run looked up as the expression that finds it again, and a method a half-done call had
already read (`pages.push` above) as a second read of it. Without frozen builtins,
writing a dump throws a `misuse` error that says so. The live examples with `debugger;`
run with frozen builtins for this reason.

**The dumper.** The code that writes dumps nearly doubles the module (<!--n dump/extra-gzip kb-->339 KB<!--/n--> more
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
