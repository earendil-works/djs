# The JavaScript interface

A host uses djs through the `@earendil-works/djs` package: one class, `Sandbox`, over the
djs-wasm module. This page covers all of it, from running one script to runs that wait
on tools, are written out as a snapshot and continue in another process. Each sandbox is its own wasm instance with
one VM, so sandboxes share nothing and a fault in one cannot touch another.

> **In short.** `Sandbox.create(options)` makes a sandbox; `run` and `runAsync` run a
> script as a codemode body with parameters and return its result as a plain host value.
> Host functions are given by name in the options and bound as globals or imports.
> Errors are `SandboxError`s with a `kind`. For runs that wait on tools, `begin` hands
> every call to the host, which settles it and advances the run; an idle run can be
> written as a [snapshot](snapshots.md) and loaded into another sandbox, and a
> deterministic one can be run again from its script and its results.

## Installing

```sh
npm install @earendil-works/djs
```

The package has no dependencies and carries the wasm module itself: one module,
<!--n wasm/bytes mb-->1.2 MB<!--/n-->, <!--n wasm/gzip kb-->498 KB<!--/n--> with gzip, which every host uses, the
ones that persist runs and the ones that never do, because it has
[snapshots](snapshots.md). It runs in node 20 or newer, Bun, Deno, browsers and web
workers: the module has no imports, so it runs in any wasm runtime. The package has one
entry point, `@earendil-works/djs`, which exports the whole API. The wire encoder and the
heap reader the `Sandbox` uses are internal and not exported.

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
| `globals` | where scripts find them as globals ([below](#host-functions)) |
| `modules` | the host modules bodies import, resolved by specifier ([below](#host-modules)) |
| `values` | host data bound as frozen globals, which snapshots name instead of writing ([below](#host-data)) |
| `jsonBoundary` | convert what crosses to the host as JSON would, for all host functions or all but some ([below](#the-json-boundary)) |
| `missingMember` | the error a read of a member a namespace lacks throws ([below](#namespaces)) |
| `aliases` | more member names of a namespace's functions ([below](#namespaces)) |
| `console` | receives `console.log`, `info`, `warn`, `error` and `debug`; default the host's `console` |
| `memoryLimit` | the heap cap in bytes; default 4 GiB ([limits](limits.md#memory)) |
| `fuelSlice` | fuel per step: how often the host gets control back; default 1,000,000 |
| `maxFuel` | the fuel budget of one run, after which it fails; default none |
| `now` | the clock `Date` reads: a fixed time or a function; default `Date.now` |
| `timeZoneOffset` | the local time zone in minutes east of UTC; default the host's |
| `randomSeed` | the seed of `Math.random`; default from `crypto.getRandomValues` |
| `deterministic` | `{ startedAt, seed }`: repeatable runs ([below](#deterministic-runs)) |
| `unhandledRejections` | `"warn"` (default), `"throw"` or `"ignore"` |
| `frozenBuiltins` | the builtins behave as frozen ([the language](language.md#frozen-builtins)) |
| `typescript` | compile scripts as TypeScript ([TypeScript](typescript.md)) |
| `debugger` | `debugger;` in an async function pauses the run ([below](#snapshots)) |

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

Both are loops over a [run the host drives](#runs-the-host-drives): the script runs until
it is idle, waiting on nothing but async host calls; then the sandbox calls the async
host functions it called, settles each call with what it returned (`runAsync`: when its
promise settles), and lets the script go on. A run ends when the body's result settles.
Async calls still running then are dropped: the run does not wait for them and their
results are ignored. A script that wants a call finished awaits it.

A sandbox runs one thing at a time, and it keeps its state between runs: what a script
puts on `globalThis` is there for the next one. `dispose()` releases the instance; a
disposed sandbox throws on every use.

## Host functions

A host function has a **name**, chosen by the host: `tools.read`, `log`,
`@sdk/our-stuff#myTool`. The name identifies it in calls, journals and snapshots; where a
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
  an exception the script can catch, and so does an argument the host cannot read, such
  as a cycle (`TypeError: argument 1: cyclic value at $.self`): the function is not
  called. An async host function's call returns a promise rejected with that error.
  `console` gets such an argument as the text of the reason instead. Under `runAsync` it may return a promise; under
  `run`, that is an error.
- An **async host function** (`asyncFunctions`) returns a promise in the VM at once, and
  the script goes on, so several calls can be open together, as in
  `Promise.all(paths.map(tools.read))`. Results are applied one at a time in the order
  they are settled, each when the script has nothing else to run, like I/O callbacks in
  an event loop ([determinism](determinism.md)).

Everything a sandbox defines comes from its options: there is no adding functions to a
sandbox that exists, so what a run can call is fixed before it starts.

**Where scripts find them.** By default, a name like `tools.read` makes a global
namespace object `tools` with a member `read`, a name that is an identifier becomes a
global of that name, and `console` is a namespace like `tools`. `globals` replaces that
rule with an explicit table, global name to function or namespace name. A function with
no global, such as `@sdk/our-stuff#myTool`, is one scripts [import](#host-modules).

```ts
const sandbox = await Sandbox.create({
  functions: { "@sdk/our-stuff#myTool": (x) => `tool(${x})` },
  modules: (spec) => (spec === "@sdk/our-stuff" ? { exports: { myTool: "@sdk/our-stuff#myTool" } } : undefined),
});
sandbox.run(`import { myTool } from "@sdk/our-stuff"; return myTool('x')`);   // "tool(x)"; no global myTool
```

The globals the sandbox binds (functions, namespaces, `values`, the `globals` table) are
read-only and cannot be deleted: `globalThis.tools = null` throws a `TypeError` in a
codemode body, which is strict code, and `tools` stays. A loading sandbox binds them the
same way. Globals that scripts make are ordinary.

### Namespaces

A namespace such as `tools` or `console` is an object the sandbox makes, with a null
prototype and its functions as enumerable members in the order they were defined, so
`Object.keys(tools)` lists them and `tools.toString` is `undefined`. The sandbox freezes
it once it has its members, before any script runs: a script can neither add, change nor
delete a member (and a codemode body is strict code, so trying throws a `TypeError`).

Two options shape what a script sees of them:

```ts
const sandbox = await Sandbox.create({
  asyncFunctions: { "tools.my_tool": myTool, "tools.read": read },
  aliases: { "tools.my_tool": ["my-tool"] },
  missingMember: (ns, member) => `${ns}.${member} does not exist. Available: ${known(ns).join(", ")}.`,
});
await sandbox.runAsync(`return tools["my-tool"] === tools.my_tool`);  // true
await sandbox.runAsync(`return tools.Read("a.txt")`);  // threw: TypeError: tools.Read does not exist. Available: ...
```

- `aliases` gives a member more names, which must not be identifiers: a tool named
  `my-tool` that scripts also reach as `tools.my_tool`. Each alias follows its member
  among the namespace's keys and holds the same function, which keeps its one identity:
  calls, journals and snapshots name it `tools.my_tool`.
- `missingMember(namespace, member)` answers a read of a member the namespace does not
  have: the read throws a `TypeError` with the returned message, where otherwise the
  script would get `undefined` and fail later with `not a function`. Returning
  `undefined` lets the read give `undefined`. Symbols, the names of `Object.prototype`'s
  members, `then` and `toJSON` are never asked about (and read `undefined`), so `await`,
  `JSON.stringify` and `in` behave as usual. The namespaces are then proxies over their frozen member objects, made by
  the sandbox; a snapshot still names each by its identity, and a loading sandbox with the
  option gives its own. It applies to the namespaces of dotted names, `console`
  included, not to `values` or the namespaces of host modules.

**Errors with data.** An error a host function throws, or an async one rejects with,
reaches the script as an `Error` of the same kind (`TypeError`, `RangeError`, else
`Error`) with its message, and with the host error's own enumerable properties but
`name`, `message` and `stack`, as values in: a tool's `{ code, structuredOutput }` is
there in the script's `catch`. A property the wire cannot carry, such as a function, is
left out.

**Ending the run.** A host function that throws `new RunExit(value)` ends the run where
it is: the script cannot catch it, no `finally` runs and no more of its code runs. The run
is done with `value`: `run` and `runAsync` return it, and a host-driven run is `done`
with `exited` set. That is how an agent's `exit()` helper is
written. An async host function may throw or reject with it too; the run ends when that
result would be applied.

### Host modules

A body can import host functions from modules the host defines
([imports in a body](language.md#imports-in-a-body)). The `modules` option resolves a
specifier to the module's exports, and the sandbox asks it once per specifier, only for
those the scripts import, so a host with a hundred tool servers lists none up front:

```ts
const asyncFunctions = {};
for (const [spec, server] of mcpServers) {                 // "@mcp/github"
  for (const t of server.tools) asyncFunctions[`${spec}#${t.name}`] = (args) => server.call(t.name, args);
}
const sandbox = await Sandbox.create({
  asyncFunctions,
  modules: (specifier) => {
    const server = mcpServers.get(specifier);
    if (server === undefined) return undefined;            // Cannot find module
    return { exports: Object.fromEntries(server.tools.map((t) => [t.name, `${specifier}#${t.name}`])) };
  },
});
await sandbox.runAsync(`
  import { search_issues } from "@mcp/github";
  return (await search_issues({ q: "snapshots" })).length;
`);
```

Each export names a host function, namespace or value of the sandbox, such as
`"@mcp/github#search_issues"` or `"tools.read"`; the function keeps that name in calls
and snapshots. An unknown module or export is a `syntax` error of the run, with the
specifier or the name and their position: `SyntaxError: Cannot find module '@mcp/nope'
at 1:19`. The imported names are the body's parameters after `params`, so a name taken
twice is one too. A namespace import (`import * as linear from "@mcp/linear"`) gives one
frozen object per specifier in the sandbox, with every export as a member, named by the
specifier in snapshots; a specifier that is also the name of a namespace such as `tools`
cannot be imported as a namespace. A sandbox that loads a snapshot makes the module
namespaces the snapshot names before it loads it, through the same `modules`.

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
  asyncFunctions: Object.fromEntries([...servers].flatMap(([spec, tools]) =>
    tools.map((t) => [`${spec}#${t.name}`, (args) => callTool(spec, t.name, args)]))),
  modules: (specifier) => {
    const tools = servers.get(specifier);
    return tools && { exports: Object.fromEntries(tools.map((t) => [t.name, `${specifier}#${t.name}`])) };
  },
});
```

### Host data

`values` binds host data as globals: each is copied in, deep-frozen, and is a host value
of its name, like a namespace, so a module can export it too. A list of the tools a script
may call is the typical one:

```ts
const sandbox = await Sandbox.create({ values: { ALL_TOOLS: tools.map(({ name, description }) => ({ name, description })) } });
```

A snapshot does not write such data: it refers to it by name, so the sandbox
that loads it must be given the same values again; an object the script took out of them
(`ALL_TOOLS[0]`) is named by its path and is the loading sandbox's own after the load. A name that is also a host function's or
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
function is not called. A result that cannot be converted fails the run, and a thrown
value that is not an `Error` is converted too, to `undefined` if JSON cannot carry it.
`console` is not converted, and neither are the functions `jsonBoundary: { except:
[...] }` lists by identity: output and storage functions that format values themselves
see `NaN` or a function as what it is, where JSON would make it `null` or leave it out.

The conversion is done by functions the sandbox makes in the VM, one per host function,
which scripts see in its place. They count as host values of the functions' names, so a
snapshot refers to them by name and a loading sandbox with the boundary supplies its own:
a function the script holds is still `===` to `tools.read` after a load, and the
conversion's code is never in a snapshot. They take the builtins they use when the sandbox is
made, so a script that replaces `JSON.stringify` does not change what the host receives.

## Values in and out

**In.** Parameters, `values` and host function results go into the VM in a
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
| errors | host `Error`s with the VM error's `name`, `message`, `stack` and `cause`, and its own enumerable data properties |
| `Date`, `Map`, `Set` | a `Date` of the same time; a `Map` or `Set` of the entries, each read like the rest |
| functions, symbols, other objects | `Opaque` markers, with the object's class name or the symbol's description |

The result is a tree, like JSON: an object referenced twice is read twice, and a cycle is
a `TypeError` naming its path. A value budget turns a blow-up (a small structure
referenced many times) into an error. An error's `name` and `message` are read as the
data properties the error or its prototypes hold, a number or another primitive as
`String()` makes it text; an accessor or an object would have to run code, so the name is
then `Error` and the message empty. Its `stack` is what scripts see as `e.stack`: the
VM's trace, frames only, or the string a script set:

```ts
try {
  sandbox.run("function f() { throw new TypeError('boom'); }\nf()");
} catch (e) {
  const v = (e as SandboxError).value as Error;          // v.name "TypeError", v.message "boom"
  console.log(`${v.name}: ${v.message}\n${v.stack}`);   // TypeError: boom\n    at f (sandbox.js:1:35)\n    at <eval> ...
}
```

`console`'s methods get their arguments the same way, except that a value with no host
form arrives as the text `[Function]`, `[WeakMap]` and so on, where a result has an
`Opaque` marker (exported by the package).

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
| `threw` | the script threw, or its promise rejected; `message` is the error's display text (`TypeError: bad`), `value` the thrown value as a host value (an `Error` with the VM's `name`, `message` and `stack`) | usable |
| `syntax` | the code did not compile; `message` gives the position (`SyntaxError: ... at 1:11`) | usable |
| `fuel` | the run used up `maxFuel` (`out of fuel after N units`), or a step its own budget | usable |
| `aborted` | a step's `signal` was aborted ([runs the host drives](#runs-the-host-drives)) | usable |
| `memory` | the host could not provide memory below the cap, or a host-side allocation failed | usable |
| `trap` | the engine trapped: a bug, never something a script can cause | unusable |
| `misuse` | the API was used wrongly: a run already in progress, a snapshot of a run that holds what snapshots refuse, a module without a needed export | depends |
| `diverged` | a re-run or a replay did not do what the recorded run did ([deterministic runs](#deterministic-runs)) | usable |

Running out of the heap cap is not a `memory` error: the script gets a catchable
`InternalError: out of memory` and may recover, and only if it does not is the run
`threw` ([limits](limits.md#memory)).

A promise that is rejected with no handler, once the job queue is empty, is reported as
QuickJS reports it: with `unhandledRejections: "warn"` its reason goes to
`console.warn` as `Possibly unhandled promise rejection: Error: lost`; `"throw"` fails
the run; `"ignore"` drops them.

## Limits

`maxFuel` bounds a run's work, `memoryLimit` its heap, and the VM's stack is bounded by
itself; [limits](limits.md) explains each. `fuelSlice` sets how often the host gets
control back. `maxFuel` is exact: the last slice is cut to what is left of it.

```ts
const sandbox = await Sandbox.create({ maxFuel: 10_000_000, fuelSlice: 100_000, memoryLimit: 64 << 20 });
try {
  sandbox.run("while (true) {}");
} catch (e) {
  if (e instanceof SandboxError && e.kind === "fuel") console.log(e.message); // out of fuel after 10000000 units
}
```

## Runs the host drives

A host that wants to decide every call itself, for instance to keep each call in a
durable queue of its own, drives the run:

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
`advance()` runs to the next idle point or the end, applying the settled results one at a
time in the order they were settled. Sync host functions are still called as the script calls them. While the run is
idle, `run.snapshot({ states })` writes it as a snapshot, with the host's state of each
open call by id, and `sandbox.loadSnapshot(snapshot)` loads one into a fresh sandbox and
returns the run with its open calls (same ids) and their states, for the host to settle,
issue again or reject. Settling a call with `{ ok: false, value: new RunExit() }` ends
the run as a host function's `RunExit` does.

The run is `done` (or `threw`) as soon as the body's result settles, also while calls
are open: `run.dropped` lists those, for the host to cancel, and settling one of them
afterwards does nothing.

```ts
const run = sandbox.begin("tools.log('started'); return 'x'");
run.state;                                  // "done", run.value "x"
for (const call of run.dropped) cancel(call.id);
```

`maxFuel` bounds the run over all its steps, and `run.fuelUsed` says how much it has
used. A snapshot carries the count: the loaded run goes on counting from it, so the
budget holds across any number of snapshots and loads.

A `debugger;` statement in an async function is a pause when the sandbox has the
`debugger` option: the statement awaits a call of the async host function `debugger`,
listed in `run.calls` like any other, so the run is idle there and can be snapshotted;
settling the call goes on. `run` and `runAsync` call the option's function, or with
`debugger: true` go on at once. Without the option, `debugger;` does nothing.

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
is `threw`, and the sandbox can run again. A run can only be snapshotted at an idle
point, so a host that persists runs goes on from the last snapshot it kept. `begin` takes
`signal` and `fuel` for its first step too, but it returns synchronously and cannot
yield; with `start: false` it only compiles and starts the body, and the first `advance`
runs it.

## Snapshots

A snapshot is a waiting run as data: every object and internal record the run can reach
beyond what a fresh sandbox has, written by kind and field names, the functions it
stopped in named by their place in the script and checked by their bytecode, the
builtins by their names. Code is not in it: the loading sandbox compiles the scripts
again. What the options and methods do:

- The sandbox's own values are named, not written: its host functions by identity, its
  namespaces (with their aliases and `missingMember` guards), `values`, module namespaces
  and the JSON boundary's converting functions by name. The sandbox that loads a
  snapshot must be made with options that give the same names; it loads into its own
  setup, so a script's `tools` is the loading sandbox's `tools`. A name it lacks is a
  `misuse` error before anything is loaded.
- The scripts' texts are in the snapshot, so it loads anywhere the host gives the same
  names.
- Loading checks every waiting function's bytecode against the loading build's: a djs
  that compiles one of them differently refuses the snapshot, naming the function, and
  so does one without a builtin, record field or named constant the snapshot uses. A
  refused load leaves the sandbox unusable; dispose it.
- A snapshot cannot hold a `WeakRef` or a `FinalizationRegistry`, nor a run whose script
  changed a builtin object: snapshotting such a run throws a `misuse` error that names
  what it met, and the run goes on.
- `sandbox.describeSnapshot(snapshot)` describes one for people: the run in JavaScript
  terms (each open call, the activations waiting on it with their source lines and
  variables, up to the run's result), what a loading djs must have, the size in words, and
  the records as djs stores them. `sandbox.inspectSnapshot(snapshot)` returns the same as
  data (`SnapshotInspection`: sections, types, strings, scripts, functions, records with
  labels and notes, the run as a tree), as `djs describe --json` writes it
  ([describing a snapshot](snapshots.md#describing-a-snapshot)).
- `sandbox.callSite(id)` says where the run waits on open call `id`: `{ file, line,
  column }` of the `await` of the innermost activation waiting on it, directly or through
  `Promise.all` and the like, in the script the run started with, also after any number
  of loads. It returns `undefined` when nothing waits on the call.
- Those three are tools for people and need the docs module
  ([the module builds](host.md#the-module-builds)); with the shipped module they throw a
  `misuse` error. `djs describe file.snapshot` describes a snapshot file.

A snapshot is trusted input: load only snapshots you wrote. [Snapshots: keeping a waiting
run](snapshots.md) explains what a snapshot holds, what a later build of djs can load,
and why.

## Deterministic runs

A snapshot loads only into a build of djs that compiles its waiting functions alike. When
a newer build refuses one, the run is not lost if it is deterministic: the host begins it
again from its script and gives it the results it had, and the run is back where it was.

```ts
const options = { deterministic: { startedAt: Date.now(), seed: 42n }, ...rest };
const sandbox = await Sandbox.create(options);
const run = sandbox.begin(code, params);
// ... the host settles calls and advances; with each snapshot it also keeps:
save({ snapshot: run.snapshot(), deliveries: run.deliveries });

// Later, when `loadSnapshot` refuses the snapshot: the same script, parameters and options.
const again = await Sandbox.create(options);
const rerun = again.begin(code, params, { deliveries });   // idle where the run was, or `diverged`
```

With `deterministic`, `Math.random` is seeded from `seed`, the time zone is UTC, weak
references are strong, and the clock starts at `startedAt` and moves only when an async
result is applied: to the time of `now` when it was settled, never back. `run.deliveries`
lists the results the run applied, in order, each with its call (`id`, `fn`, `args`),
its outcome and the clock it set. `begin` with `deliveries` applies them again, each once
its call is open, checked against it, and returns the run idle where the earlier one was
after its last delivery, with the same open calls; from there the host goes on as before.
If the script, its parameters or the options changed so that the run makes another call,
or the same call with other arguments, `begin` throws a `diverged` error naming the first
difference. No async host function runs during a re-run; the sync ones run as they did,
so a host stubs those whose effects it has already (output it showed, data it stored)
and gives the others the answers they gave. A host continuing such a run by hand can
settle with the recorded clock too: `settle(id, { ok, value, now })`. A snapshot of a
deterministic run carries its clock and random state, and loads only into a sandbox that
is deterministic too.

A run with a `journal` records more: every sync host call and its outcome, every result,
and the VM's `canonicalHash()` at every idle point. `replay(journal)` runs it again in a
fresh sandbox made the same way without calling any host function but `console`, and
checks every step:

```ts
const journal: JournalEntry[] = [];
await sandbox.runAsync(code, params, { journal });       // or begin(code, params, { journal })
await (await Sandbox.create(options)).replay(journal);   // the same result, or a `diverged` error
```

[Determinism](determinism.md) explains what that guarantees.

## Inspecting a sandbox

`heapSize` is the arena's size in bytes, `collect()` runs the garbage collector, and
`heapView()` returns the arena itself as a `Uint8Array`, valid until the next call into
the sandbox, for tools that walk the heap with the layout tables of
[`js/src/layout.generated.ts`](../../js/src/layout.generated.ts) (this site's live
examples do).

## Where to look

- [`js/src/sandbox.ts`](../../js/src/sandbox.ts): the `Sandbox` and `Run` classes and
  every option, with their doc comments.
- [`js/src/wire.ts`](../../js/src/wire.ts) and [`js/src/reader.ts`](../../js/src/reader.ts):
  values in and out.
- [`scripts/npm-pack.sh`](../../scripts/npm-pack.sh) and
  [`scripts/npm-test.sh`](../../scripts/npm-test.sh): how the package is built, and how
  the packed package is tested in node, Bun, Deno and a browser, bundled and not.
- [`js/test/`](../../js/test/): the package's tests, which use every part of it.
- [`examples/sleep-snapshot/`](../../examples/sleep-snapshot/): a complete host that
  snapshots a waiting run and continues it in another process.
