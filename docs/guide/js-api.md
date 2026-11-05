# The JavaScript interface

A host uses djs through the `@djs/host` package: one class, `Sandbox`, over the djs-wasm
module. This page covers all of it, from running one script to runs that are suspended
into a dump and continued in another process. Each sandbox is its own wasm instance with
one VM, so sandboxes share nothing and a fault in one cannot touch another.

> **In short.** `Sandbox.create(options)` makes a sandbox; `run` and `runAsync` run a
> script as a codemode body with parameters and return its result as a plain host value.
> Host functions are given by name and bound as globals or parameters. Errors are
> `SandboxError`s with a `kind`. For runs that wait on tools, `start` returns a task that
> can be suspended into a [continuation dump](dumps.md) and resumed with
> `Sandbox.resume`; `begin` hands every call to the host instead.

## Loading the module

```ts
import { Sandbox, SandboxError, loadModule } from "@djs/host";

const module = await loadModule();                       // compiled once, cached
const sandbox = await Sandbox.create({ module });
```

`loadModule(location?)` compiles djs-wasm once per location and caches the
`WebAssembly.Module`; every sandbox then gets its own instance of it. The default
location is `$DJS_WASM`, else the build output of the repository; in a browser, pass a
URL. `Sandbox.create` loads the default module itself when no `module` is given. The
constructor, `new Sandbox(options)`, works synchronously once a module is loaded.

The module has no imports, so it runs in node, in a browser or in any wasm runtime. The
package is TypeScript that node runs directly; its entry points are
[`js/src/sandbox.ts`](../../js/src/sandbox.ts) (the `Sandbox`), `@djs/host/wire` (the
encoder), `@djs/host/reader` (the heap reader) and `@djs/host/dump-runtime` (the
helpers a [dump](dumps.md#djs-what-the-loading-host-provides) needs in a host without
djs).

## Creating a sandbox

All options are optional:

| option | what it does |
| --- | --- |
| `module` | the compiled djs-wasm module; default `loadModule()` |
| `functions`, `asyncFunctions` | the host functions, by name ([below](#host-functions)) |
| `globals`, `bindings` | where scripts find them: as globals, or as parameters of every body |
| `console` | receives `console.log`, `info`, `warn` and `error`; default the host's `console` |
| `memoryLimit` | the heap cap in bytes; default 4 GiB ([limits](limits.md#memory)) |
| `fuelSlice` | fuel per step: how often the host gets control back; default 1,000,000 |
| `maxFuel` | the fuel budget of one run, after which it fails; default none |
| `now` | the clock `Date` reads: a fixed time or a function; default `Date.now` |
| `timeZoneOffset` | the local time zone in minutes east of UTC; default the host's |
| `randomSeed` | the seed of `Math.random`; default from `crypto.getRandomValues` |
| `weakRefs` | `"collected"` (default), `"strong"` or `"absent"`: what `WeakRef` and `FinalizationRegistry` do |
| `deterministic` | `{ startedAt, seed }`: repeatable runs ([below](#deterministic-runs-and-journals)) |
| `unhandledRejections` | `"warn"` (default), `"throw"` or `"collect"` |
| `frozenBuiltins` | the builtins behave as frozen ([the language](language.md#frozen-builtins)); dumps need it |
| `typescript`, `experimentalDecorators`, `emitDecoratorMetadata` | compile scripts as TypeScript ([TypeScript](typescript.md)) |
| `dumpModule`, `sources`, `inlineSources`, `debugger` | for [continuation dumps](#continuation-dumps) |

## Running code

```ts
const sandbox = await Sandbox.create();
sandbox.run("return a + b", { a: 40, b: 2 });            // 42
await sandbox.runAsync("const r = await fetchJson(url); return r.items.length", { url });
```

`run(code, params)` runs `code` as a codemode body: the body of an async function whose
parameters are the keys of `params`, so `await` and `return` work at the top level. It
returns what the body returns, as a host value; a returned promise is unwrapped. It
drives the VM to the end synchronously, so its host functions must return synchronously
too. `runAsync` is the same but awaits host functions that return promises, and lets
async host functions run concurrently while the script goes on.

A sandbox runs one thing at a time, and it keeps its state between runs: what a script
puts on `globalThis` is there for the next one. `setGlobal(name, value)` defines a global
holding a copy of a host value. `dispose()` releases the instance; a disposed sandbox
throws on every use.

## Host functions

A host function has a **name**, chosen by the host: `tools.read`, `log`,
`@sdk/our-stuff#myTool`. The name identifies it in calls, journals and dumps; where a
script finds the function is a separate choice.

```ts
const sandbox = await Sandbox.create({
  functions: { "tools.add": (a, b) => (a as number) + (b as number) },
  asyncFunctions: { "tools.read": async (path) => fs.readFile(path as string, "utf8") },
});
sandbox.run("return tools.add(40, 2)");                  // 42
await sandbox.runAsync("return (await tools.read('a.txt')).length");
```

- A **sync host function** (`functions`) parks the VM: the script waits at the call
  until the function returns, and the value is the call's result. A thrown error becomes
  an exception the script can catch. Under `runAsync` it may return a promise; under
  `run`, that is an error.
- An **async host function** (`asyncFunctions`) returns a promise in the VM at once, and
  the script goes on, so several calls can be open together, as in
  `Promise.all(paths.map(tools.read))`. Results are applied in the order they are
  delivered, each when the script's job queue is empty, like I/O callbacks in an event
  loop ([determinism](determinism.md)).

**Where scripts find them.** By default, a name like `tools.read` makes a global
namespace object `tools` with a member `read`, a name that is an identifier becomes a
global of that name, and `console` is a namespace like `tools`. `globals` replaces that
rule with an explicit table, global name to function or namespace name, and `bindings`
passes them as parameters of every body instead, after the run's own `params`: a stand-in
for imports.

```ts
const sandbox = await Sandbox.create({
  functions: { "@sdk/our-stuff#myTool": (x) => `tool(${x})` },
  bindings: { myTool: "@sdk/our-stuff#myTool" },
});
sandbox.run("return myTool('x')");                       // "tool(x)"; no global myTool
```

`defineFunction(name, f)` and `defineAsyncFunction(name, f)` add functions to a sandbox
that exists, bound by the default rule.

## Values in and out

**In.** Parameters, `setGlobal` values and host function results go into the VM in a
compact binary format that the VM decodes in one pass. It carries `undefined`, `null`,
booleans, numbers, strings, arrays (holes included), `Uint8Array`s as raw bytes, and
plain objects (prototype `Object.prototype` or `null`; their own enumerable string
keys). Anything else is a `TypeError` that names its path: `cannot encode Map at $`,
`cannot encode a function at $.a[1]`. A cycle is one too. `toJSON` is not called.

**Out.** Results, arguments of host calls and thrown values are read straight out of
the VM's memory, without running any code in the VM:

| VM value | host value |
| --- | --- |
| primitives | the same (`-0`, `NaN` and `undefined` included); a BigInt as `bigint` |
| plain objects, class instances | plain objects of their own enumerable string-keyed data properties |
| arrays | arrays, with their holes |
| typed arrays | copies, of the same type |
| functions, symbols, `Map`, `Date`, errors, other objects | `Opaque` markers, with the object's class name or the symbol's description |

The result is a tree, like JSON: an object referenced twice is read twice, and a cycle is
a `TypeError` naming its path. A value budget turns a blow-up (a small structure
referenced many times) into an error. To get a `Map` or a `Date` out, a script converts
it: `Object.fromEntries(map)`, `date.toISOString()`.

> **Trade-off.** Reading the VM's memory over serializing in the VM. Reading a value
> takes less time than the VM alone needs to turn it into JSON, before the host has
> parsed anything: <!--n wire/read-vs-json-objects .1-->1.9<!--/n--> times less for many small objects, <!--n wire/read-vs-json-text .0-->10<!--/n--> times less for text with
> non-ASCII characters, and a 1 MiB string reads at memory-copy speed. The cost: no sharing or identity crosses
> the boundary, and what crosses is data, not live objects.

## Errors

Everything that goes wrong in a run is a `SandboxError` with a `kind`, a `message` and,
for an exception, the thrown `value`:

| kind | when | the sandbox afterwards |
| --- | --- | --- |
| `threw` | the script threw, or its promise rejected; `message` is the error's display text (`TypeError: bad`), `value` the thrown value as a host value (an `Opaque` for an `Error`) | usable |
| `syntax` | the code did not compile; `message` gives the position (`SyntaxError: ... at 1:11`) | usable |
| `fuel` | the run used up `maxFuel` (`out of fuel after N units`) | usable |
| `memory` | the host could not provide memory below the cap, or a host-side allocation failed | usable |
| `trap` | the engine trapped: a bug, never something a script can cause | unusable |
| `misuse` | the API was used wrongly: a run already in progress, a dump without frozen builtins, a module without a needed export | depends |
| `diverged` | a replay did not match its journal | usable |
| `suspended` | a task's result, after the task was suspended into a dump | (the task is over) |

Running out of the heap cap is not a `memory` error: the script gets a catchable
`InternalError: out of memory` and may recover, and only if it does not is the run
`threw` ([limits](limits.md#memory)).

A promise that is rejected with no handler, once the job queue is empty, is reported as
QuickJS reports it: with `unhandledRejections: "warn"` its reason goes to
`console.warn` as `Possibly unhandled promise rejection: Error: lost`; `"throw"` fails
the run; `"collect"` keeps the reasons for `takeUnhandledRejections()`.

## Limits

`maxFuel` bounds a run's work, `memoryLimit` its heap, and the VM's stack is bounded by
itself; [limits](limits.md) explains each. `fuelSlice` sets how often the host gets
control back. A sandbox checks `maxFuel` between slices, so a budget is exact to the
slice: for a small budget, use a small slice.

```ts
const sandbox = await Sandbox.create({ maxFuel: 10_000_000, fuelSlice: 100_000, memoryLimit: 64 << 20 });
try {
  sandbox.run("while (true) {}");
} catch (e) {
  if (e instanceof SandboxError && e.kind === "fuel") console.log(e.message); // out of fuel after 10000000 units
}
```

## Tasks: suspend and resume

A **task** is a run the sandbox drives, like `runAsync`, that can be suspended while it
waits on host calls and continued later from the dump:

```ts
const task = sandbox.start(code, params, { filename: "job.js" });
// ... the run calls async host functions and waits ...
const dump = await task.suspend();         // at the next idle point; the task stops
const resumed = await Sandbox.resume(dump, options);
const result = await resumed.result;
```

- `start(code, params, { filename })` runs the body to its first stop; a compile error
  throws. `task.result` is a promise of the outcome; `task.calls` lists the open host
  calls.
- `task.suspend()` waits for the next idle point, writes the dump and stops the run:
  `task.result` rejects with kind `suspended`, and the open calls' abort signals fire.
  If the run cannot be dumped there, `suspend()` rejects and the task goes on.
- `task.checkpoint()` writes a dump at the next idle point and lets the task go on.
- `Sandbox.resume(dump, options)` makes a new sandbox with the same host functions (by
  name) and continues the run in it; `sandbox.resume(dump)` does the same in a fresh
  sandbox the host made itself.

An async host function that should survive a dump is an object with hooks, an
`AsyncHostFunction`:

```ts
const sleep: AsyncHostFunction<{ due: number }> = {
  async call(ms) {                         // the script called it; `this` is the call
    this.state = { due: Date.now() + Number(ms) };
    await until(this.state.due, this.signal);
  },
  save(call) { return call.state; },       // optional: the state a dump keeps (the default)
  async resume({ due }, call) {            // after a load, with that state
    await until(due, call.signal);
  },
};
```

`call` starts the call; `this` is a `HostCall`, with the call's `id`, `fn`, `args`,
`startedAt`, its `state` (JSON data a dump keeps) and a `signal` that aborts when the
sandbox stops waiting for it. After a load, `resume(state, call)` continues each open
call; a function without `resume` is called again, and a plain function's open call is
rejected. [Continuation dumps](dumps.md) shows the whole cycle.

## Host-driven runs

A host that wants to decide every call itself, for instance to keep each call in a
durable queue of its own, drives the run instead:

```ts
const run = sandbox.begin(code, params, { filename: "job.js" });
while (run.state === "idle") {
  for (const call of run.calls) queue.add(call);         // { id, fn, args }
  const { id, ok, value } = await queue.nextResult();
  run.settle(id, { ok, value });
  await run.advance();                                   // to the next idle point
}
if (run.state === "done") use(run.value);
else throw run.error;
```

`begin` runs the body to its first idle point and never calls async host functions:
`run.calls` lists them, oldest first. `settle(id, { ok, value })` delivers an outcome;
`advance()` applies the settled results in order and runs to the next idle point or the
end. Sync host functions are still called as the script calls them. While the run is
idle, `run.dump(states)` writes it as a dump, with the host's state of each open call by
id, and `sandbox.loadRun(dump)` loads one into a fresh sandbox and returns the run with
its open calls and their states, for the host to settle, issue again or reject.

## Continuation dumps

Dumps are the subject of [their own page](dumps.md). What the options do:

- `frozenBuiltins: true` is required to write dumps.
- `dumpModule` is the build of djs-wasm with the dumper, for a `module` without it:
  dumping copies the VM into an instance of the dump module. Loading needs neither.
- `sources(name)` returns the code of the script a dump refers to by name, for a dump
  from another process; `inlineSources: true` writes the script into the dump instead.
- `debugger`, a host function, makes `debugger;` in an async function a pause: the
  statement awaits a call of this function, so the run is idle there and can be dumped.
  It needs a module with the dumper. Without the option, `debugger;` does nothing.
- `sandbox.callSite(id)` says where the run waits on open call `id`: `{ file, line,
  column }` of the `await` of the innermost activation waiting on it, directly or through
  `Promise.all` and the like. The file is the name the code was compiled as: the run's
  `filename` for the code it started with, `dump.js` for code a dump carried, because a
  run continued from a dump runs the dump's text. It is part of the dump module only and
  returns `undefined` elsewhere, and when nothing waits on the call.

`sandbox.dump(names?)` writes the sandbox's globals (or the named ones) as a dump of
data and functions, without a run; `sandbox.load(dump)` runs such a dump in a fresh
sandbox and returns the open calls it recreated.

## Deterministic runs and journals

```ts
const sandbox = await Sandbox.create({ deterministic: { startedAt: 0, seed: 42n }, now: 0 });
const journal: JournalEntry[] = [];
const result = await sandbox.runAsync(code, params, { journal });
// later, in a fresh sandbox made the same way:
const again = await Sandbox.create({ deterministic: { startedAt: 0, seed: 42n }, now: 0 });
await again.replay(journal);                             // the same result, or a `diverged` error
```

With `deterministic`, `Math.random` is seeded from `seed`, the time zone is UTC, weak
references are strong, and the clock starts at `startedAt` and moves only when an async
result is delivered. A run with a `journal` records everything it took from outside;
`replay(journal)` runs it again without calling any host function and checks the
`canonicalHash()` of the VM at every idle point. [Determinism](determinism.md) explains
what that guarantees.

## Inspecting a sandbox

`heapSize` is the arena's size in bytes, `collect()` runs the garbage collector, and
`heapView()` returns the arena itself as a `Uint8Array`, valid until the next call into
the sandbox, for tools that walk the heap with the layout tables the package exports
(this site's live examples do).

## Where to look

- [`js/src/sandbox.ts`](../../js/src/sandbox.ts): the `Sandbox`, `Task` and `Run`
  classes and every option, with their doc comments.
- [`js/src/wire.ts`](../../js/src/wire.ts) and [`js/src/reader.ts`](../../js/src/reader.ts):
  values in and out.
- [`js/test/`](../../js/test/): the package's tests, which use every part of it.
- [`examples/sleep-dump/`](../../examples/sleep-dump/): a complete host that suspends
  and resumes a run.
