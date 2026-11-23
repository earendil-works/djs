# The JavaScript interface

A host uses djs through the `@earendil-works/djs` package: one class, `Sandbox`, over the
djs-wasm module. This page covers all of it, from running one script to runs that are
suspended into a dump and continued in another process. Each sandbox is its own wasm instance with
one VM, so sandboxes share nothing and a fault in one cannot touch another.

> **In short.** `Sandbox.create(options)` makes a sandbox; `run` and `runAsync` run a
> script as a codemode body with parameters and return its result as a plain host value.
> Host functions are given by name and bound as globals or parameters. Errors are
> `SandboxError`s with a `kind`. For runs that wait on tools, `start` returns a task that
> can be suspended into a [continuation dump](dumps.md) and resumed with
> `Sandbox.resume`; `begin` hands every call to the host instead.

## Installing

```sh
npm install @earendil-works/djs
```

The package has no dependencies and carries the wasm module itself: one module,
<!--n wasm/bytes mb-->1.9 MB<!--/n-->, <!--n wasm/gzip kb-->768 KB<!--/n--> with gzip, which every host uses, the
ones that persist runs and the ones that never do, because it has the
[dumper](dumps.md#what-a-dump-needs). It runs in node 20 or newer, Bun, Deno, browsers and web
workers: the module has no imports, so it runs in any wasm runtime. The package has one
entry point, `@earendil-works/djs`, which exports the whole API and the dump runtime's
`helpers`, `script` and `scriptFor`, for a host that continues a
[dump](dumps.md#djs-what-the-loading-host-provides) in its own JavaScript engine. The
wire encoder and the heap reader the `Sandbox` uses are internal and not exported.

## Loading the module

```ts
import { Sandbox, SandboxError, loadModule } from "@earendil-works/djs";

const module = await loadModule();                       // compiled once, cached
const sandbox = await Sandbox.create({ module });
```

`loadModule(location?)` compiles djs-wasm once per location and caches the
`WebAssembly.Module`; every sandbox then gets its own instance of it. Without a location
it loads the package's module, from its `wasm/` directory; in node, Bun and Deno,
`$DJS_WASM` names another module instead. `Sandbox.create` loads the
default module itself when no `module` is given. The constructor, `new Sandbox(options)`,
works synchronously once a module is loaded.

A location is a path or a URL: node, Bun and Deno read a path or a `file:` URL from the
disk (Deno needs `--allow-read` for the package's directory), anything else is fetched.
The entry point names its module as `new URL("../wasm/djs.wasm", import.meta.url)`, a
pattern that webpack and Vite recognize: they copy the module into the build and rewrite
the URL. Vite's development server is the exception, because it pre-bundles dependencies
away from their files; exclude the package there with `optimizeDeps: { exclude:
["@earendil-works/djs"] }`. In the browser without a bundler, the package's files work as
they are, served with the module next to them; with a bundler that does not know the
pattern, serve the `.wasm` file yourself and pass its URL to `loadModule`.

The package's source is [`js/src/`](../../js/src/): [`sandbox.ts`](../../js/src/sandbox.ts)
holds the `Sandbox`, and [`index.ts`](../../js/src/index.ts) is the entry point.

## Creating a sandbox

All options are optional:

| option | what it does |
| --- | --- |
| `module` | the compiled djs-wasm module; default `loadModule()` |
| `functions`, `asyncFunctions` | the host functions, by name ([below](#host-functions)) |
| `globals`, `bindings` | where scripts find them: as globals, or as parameters of every body |
| `modules` | the host modules bodies import, resolved by specifier ([below](#host-modules)) |
| `values` | host data bound as frozen globals, which dumps name instead of writing ([below](#host-data)) |
| `jsonBoundary` | convert what crosses to the host as JSON would ([below](#the-json-boundary)) |
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
| `sources`, `inlineSources`, `debugger` | for [continuation dumps](#continuation-dumps) |

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
that exists, bound by the default rule. Or scripts import them from [host
modules](#host-modules).

**Errors with data.** An error a host function throws, or an async one rejects with,
reaches the script as an `Error` of the same kind (`TypeError`, `RangeError`, else
`Error`) with its message, and with the host error's own enumerable properties but
`name`, `message` and `stack`, as values in: a tool's `{ code, structuredOutput }` is
there in the script's `catch`. A property the wire cannot carry, such as a function, is
left out.

**Ending the run.** A host function that throws `new RunExit(value)` ends the run where
it is: the script cannot catch it, no `finally` runs and no more of its code runs. The run
is done with `value`: `run` and `runAsync` return it, a task's result is it, and a
host-driven run is `done` with `exited` set. That is how an agent's `exit()` helper is
written. An async host function may throw or reject with it too; the run ends when that
result would be applied.

### Host modules

A body can import host functions from modules the host defines
([imports in a body](language.md#imports-in-a-body)). The `modules` option resolves a
specifier to the module's exports, and the sandbox asks it once per specifier, only for
those the scripts import, so a host with a hundred tool servers lists none up front:

```ts
const sandbox = await Sandbox.create({
  modules: (specifier) => {
    const server = mcpServers.get(specifier);              // "@mcp/github"
    if (server === undefined) return undefined;            // Cannot find module
    return { exports: Object.fromEntries(server.tools.map((t) => [t.name, { fn: (args) => server.call(t.name, args) }])) };
  },
});
await sandbox.runAsync(`
  import { search_issues } from "@mcp/github";
  return (await search_issues({ q: "dumps" })).length;
`);
```

Each export is one of:

- an object `{ fn, sync }`: a host function the sandbox defines on the first import, with
  the name `specifier#export` (`@mcp/github#search_issues`), async unless `sync` is set.
  `fn` may be left out where the host drives the run itself, which never calls async host
  functions: the calls show up in `run.calls` under that name;
- a string: the name of a host function, namespace or value the sandbox has already, such
  as `"tools.read"`.

An unknown module or export is a `syntax` error of the run, with the specifier or the
name and their position: `SyntaxError: Cannot find module '@mcp/nope' at 1:19`. The
imported names are the body's parameters after `params` and `bindings`, so a name taken
twice is one too. A namespace import (`import * as linear from "@mcp/linear"`) gives one
frozen object per specifier in the sandbox, with every export as a member, named by the
specifier in dumps; a specifier that is also the name of a namespace such as `tools`
cannot be imported as a namespace. A sandbox that loads a dump makes the module's values
the dump names before it runs it, through the same `modules`.

The resolver's descriptor can serve the scripts' documentation too. A host that lets the
model read a virtual `.d.ts` file per module writes it from the same table, so what the
model reads and what the sandbox binds cannot drift (djs itself never reads `.d.ts`
text):

```ts
interface ToolSpec { name: string; params: string; result: string; doc: string }
const servers = new Map<string, ToolSpec[]>([
  ["@mcp/github", [{ name: "search_issues", params: "{ q: string }", result: "{ title: string }[]", doc: "Searches issues." }]],
]);

/** What `cat node_modules/@mcp/github/index.d.ts` shows the model. */
function declarations(specifier: string): string | undefined {
  return servers.get(specifier)?.map((t) => `/** ${t.doc} */\nexport function ${t.name}(args: ${t.params}): Promise<${t.result}>;`).join("\n");
}

const sandbox = await Sandbox.create({
  modules: (specifier) => {
    const tools = servers.get(specifier);
    return tools && { exports: Object.fromEntries(tools.map((t) => [t.name, { fn: (args) => callTool(specifier, t.name, args) }])) };
  },
});
```

### Host data

`values` binds host data as globals: each is copied in, deep-frozen, and is a host value
of its name, like a namespace, so `bindings` can name it too. A list of the tools a script
may call is the typical one:

```ts
const sandbox = await Sandbox.create({ values: { ALL_TOOLS: tools.map(({ name, description }) => ({ name, description })) } });
```

A dump does not write such data: it refers to it by name, so the sandbox that loads the
dump must be given the same values again. A name that is also a host function's or
namespace's, or a global the sandbox binds, is a `misuse` error.

### The JSON boundary

By default, values cross to the host as the reader reads them, so a `Date` or a `Map` an
argument holds arrives as an `Opaque` marker. With `jsonBoundary: true`, the arguments of
every call of the host's functions, and every run's result, are converted inside the VM
as `JSON.parse(JSON.stringify(value))` would: `toJSON` is called, a `Date` becomes its
ISO string, a `Map` `{}`, `undefined` and functions are left out of objects, and an
`undefined` argument stays `undefined`. A value JSON cannot carry, a cycle or a BigInt,
fails at the call: a sync function's call throws the `TypeError`, where the script can
catch it, an async function's call returns a promise rejected with it, and the host
function is not called. A result that cannot be converted fails the run. `console` is
not converted.

The conversion is done by functions the sandbox makes in the VM, one per host function,
which scripts see in its place. They count as host values of the functions' names, so a
dump refers to them by name and a loading sandbox with the boundary supplies its own: a
function the script holds is still `===` to `tools.read` after a load, and the
conversion's code is never in a dump. They take the builtins they use when the sandbox is
made, so a script that replaces `JSON.stringify` does not change what the host receives.

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
it: `Object.fromEntries(map)`, `date.toISOString()`, or the host converts everything
that crosses with [the JSON boundary](#the-json-boundary).

> **Trade-off.** Reading the VM's memory over serializing in the VM. Reading a value
> takes less time than the VM alone needs to turn it into JSON, before the host has
> parsed anything: <!--n wire/read-vs-json-objects .1-->1.5<!--/n--> times less for many small objects, <!--n wire/read-vs-json-text .0-->9<!--/n--> times less for text with
> non-ASCII characters, and a 1 MiB string reads at memory-copy speed. The cost: no sharing or identity crosses
> the boundary, and what crosses is data, not live objects.

## Errors

Everything that goes wrong in a run is a `SandboxError` with a `kind`, a `message` and,
for an exception, the thrown `value`:

| kind | when | the sandbox afterwards |
| --- | --- | --- |
| `threw` | the script threw, or its promise rejected; `message` is the error's display text (`TypeError: bad`), `value` the thrown value as a host value (an `Opaque` for an `Error`) | usable |
| `syntax` | the code did not compile; `message` gives the position (`SyntaxError: ... at 1:11`) | usable |
| `fuel` | the run used up `maxFuel` (`out of fuel after N units`), or a step its own budget | usable |
| `aborted` | a step's `signal` was aborted ([host-driven runs](#host-driven-runs)) | usable |
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
its open calls and their states, for the host to settle, issue again or reject. Settling
a call with `{ ok: false, value: new RunExit() }` ends the run as a host function's
`RunExit` does.

**Bounded steps.** A step can be bounded, so that a script that computes for long does
not hold the host:

```ts
const run = sandbox.begin(code, params, { start: false });   // state "ready": nothing ran yet
await run.advance({ signal, fuel: 50_000_000, yieldMs: 10 });
```

- `signal`, an `AbortSignal`, is checked before every fuel slice; once it is aborted the
  step fails with kind `aborted`.
- `fuel` is the step's own budget, beside the run's `maxFuel`; past it the step fails
  with kind `fuel`.
- `yieldMs` lets the host's event loop run (a macrotask) between slices once the step
  has run that many milliseconds since it started or last yielded: timers and I/O of a
  host on the same thread go on, and a `signal` can be aborted meanwhile, while
  `while (true) i++` runs. A small `fuelSlice` makes the yields more frequent.

A step stopped by a bound ends the run: `advance` rejects with the error, the run's state
is `threw`, and the sandbox can run again. A run can only be dumped at an idle point, so
a host that persists runs goes on from the last dump it kept. `begin` takes `signal` and
`fuel` for its first step too, but it returns synchronously and cannot yield; with
`start: false` it only compiles and starts the body, and the first `advance` runs it.

## Continuation dumps

Dumps are the subject of [their own page](dumps.md). What the options do:

- `frozenBuiltins: true` is required to write dumps.
- Writing a dump needs a module with the dumper, as the package's is; a build of
  djs-wasm without it throws a `misuse` error that says so. Loading needs neither the
  dumper nor frozen builtins.
- `sources(name)` returns the code of the script a dump refers to by name, for a dump
  from another process; `inlineSources: true` writes the script into the dump instead.
- `debugger`, a host function, makes `debugger;` in an async function a pause: the
  statement awaits a call of this function, so the run is idle there and can be dumped.
  It needs a module with the dumper. Without the option, `debugger;` does nothing.
- `sandbox.callSite(id)` says where the run waits on open call `id`: `{ file, line,
  column }` of the `await` of the innermost activation waiting on it, directly or through
  `Promise.all` and the like. The file is the name the code was compiled as: the run's
  `filename` for the code it started with, `dump.js` for code a dump carried, because a
  run continued from a dump runs the dump's text. It returns `undefined` in a module
  without the dumper, and when nothing waits on the call.

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
the sandbox, for tools that walk the heap with the layout tables of
[`js/src/layout.generated.ts`](../../js/src/layout.generated.ts) (this site's live
examples do).

## Where to look

- [`js/src/sandbox.ts`](../../js/src/sandbox.ts): the `Sandbox`, `Task` and `Run`
  classes and every option, with their doc comments.
- [`js/src/wire.ts`](../../js/src/wire.ts) and [`js/src/reader.ts`](../../js/src/reader.ts):
  values in and out.
- [`scripts/npm-pack.sh`](../../scripts/npm-pack.sh) and
  [`scripts/npm-test.sh`](../../scripts/npm-test.sh): how the package is built, and how
  the packed package is tested in node, Bun, Deno and a browser, bundled and not.
- [`js/test/`](../../js/test/): the package's tests, which use every part of it.
- [`examples/sleep-dump/`](../../examples/sleep-dump/): a complete host that suspends
  and resumes a run.
