# What djs is

> **Experimental.** djs is under active development. Expect bugs and changes to APIs
> and behavior.

djs is a small JavaScript and TypeScript engine written in Rust. It runs short scripts
that call tools, written by a language model or a person, inside a WebAssembly sandbox,
and it can stop such a script at any moment, write it out while it waits for a tool, and
continue it later, in another process or with a later version of djs. This page explains
what kind of script djs is built for, the parts it consists of, and how to read the rest
of the guide.

> **In short.** djs is an engine for an agent's *codemode*: small programs that call a
> host's tools, combine their results and return what matters. It keeps all of a run's
> state in one byte array, so a run can stop between any two instructions; it lets the
> host read results straight out of that memory; it compiles in one pass; and while a
> run only waits for tools, it can write the run out as plain JavaScript that recreates
> it.

## A codemode script

Instead of calling tools one at a time, a model writes a small program that calls them,
combines their results and returns what it wants to show. That program runs in a
sandbox: the host decides which tools exist, and nothing else of the outside world is
reachable.

```js live
// A stand-in for a tool the host provides, such as `tools.search`.
const search = async (q) => [
  { name: "djs", stars: 120, lang: "Rust" },
  { name: "quickjs", stars: 9000, lang: "C" },
];
const hits = await search("small js engines");
const rust = hits.filter((h) => h.lang === "Rust");
console.log(`${rust.length} of ${hits.length} in Rust:`, rust.map((h) => h.name).join(", "));
return rust;
```
<!-- gen:run -->
```text
1 of 2 in Rust: djs
```
<!-- /gen -->

A script like this is the body of an async function: `await` and `return` work at the
top level, and the value it returns is the result the host reads. Such scripts have a
particular profile, and each part of it became a design goal:

| What a codemode script does | What djs needs to do | Goal |
| --- | --- | --- |
| Receives tool results: long strings, arrays of similar objects, base64 images | Get data in and out with as little copying and conversion as possible | [cheap data passing](goals.md#cheap-data-passing) |
| Runs for milliseconds, once, then is thrown away | Start in microseconds; compile fast; keep the wasm module small | [fast startup in wasm](goals.md#fast-startup-in-wasm) |
| Is written fresh every time | Compile in one pass, straight to bytecode | [no syntax tree](goals.md#no-syntax-tree) |
| Waits for tools that may take minutes | Write the run out while it waits, continue it elsewhere, also in a later djs | [persistence](goals.md#persistence) |
| Must not hog the host, may loop forever, may be cancelled | Stop between any two instructions and resume later, even inside a `map` callback or a getter | [suspend anywhere](goals.md#suspend-anywhere) |

These five goals are ranked; [what djs optimizes for](goals.md) explains each of them.
Four requirements hold under all of them: a script is [isolated](goals.md#isolation)
from everything its host did not give it, a run is
[deterministic](goals.md#determinism), [wasm32 comes first](goals.md#wasm32-first), and
behavior [matches QuickJS](goals.md#quickjss-behavior), down to error messages, because
models read those messages and the hosts' tests expect them.

## The shape of the system

![The parts of djs and the arena they share](img/overview.svg)

- The **compiler** turns source text (JavaScript or TypeScript; types are erased while
  parsing) into bytecode in a single pass, without building a syntax tree.
- The **interpreter** runs that bytecode. JavaScript calls do not recurse on the Rust
  stack: every frame lives in the arena, so the interpreter can return to the host after
  any instruction and pick up exactly where it stopped.
- The **arena** holds everything: objects, strings, bytecode, the value stack, the call
  frames, the promise job queue, the pending tool calls. Values refer to each other by
  `u32` offsets, never by pointers, so the arena can move, be copied and be read from
  the outside.
- The **garbage collector** compacts the arena in place, which keeps it small and makes
  the heap of a run a deterministic function of the run.
- The **builtins** (`Array.prototype.map`, `JSON.parse`, ...) are written in a strict
  subset of TypeScript and compiled twice: to bytecode that ships in the base snapshot,
  and, for the hot ones, to Rust state machines that can suspend in the middle.
- The **dumper** writes a waiting run as a *continuation dump*: a JavaScript program
  that recreates the run's data, its functions stopped at their `await`s and its open
  tool calls. Loading a dump is running it.
- The **host** loads the djs-wasm module, sends values in through a compact wire format
  and reads results by walking the arena directly, with a reader generated from the same
  layout tables as the engine. The `@earendil-works/djs` package wraps all of that in one class,
  `Sandbox`.

## What it looks like from a host

The host package is on npm (`npm install @earendil-works/djs`, see
[installing](js-api.md#installing)). A host creates a sandbox, gives it the tools a
script may call, and runs the script:

```ts
import { Sandbox } from "@earendil-works/djs";

const sandbox = await Sandbox.create({
  asyncFunctions: { "tools.search": async (q) => searchTheWeb(q as string) },
  maxFuel: 50_000_000,
});
const result = await sandbox.runAsync(
  `const hits = await tools.search(query); return hits.slice(0, 3);`,
  { query: "small js engines" },
);
```

[The JavaScript interface](js-api.md) covers everything the host package offers, from
this one call to runs that are suspended into a dump and continued in another process.

## How to read this guide

The first part, *Using djs*, describes what a user of djs sees: the
[language](language.md) and its [TypeScript](typescript.md) support, the [JavaScript
interface](js-api.md) for hosts, [continuation dumps](dumps.md), [limits](limits.md) on
fuel, memory and depth, [determinism](determinism.md), and [the command line and the
REPL](cli.md). The last page of the part shows how [one agent host](pi.md) wires djs up.

The second part, *How it works*, goes from the bottom up: the arena and values, cells and
objects, the compiler, the interpreter, suspension, the collector, images, the builtins
and the wasm boundary. Each chapter explains its **trade-offs**: what the design costs,
and the alternatives that were not taken. The reference part lists the opcodes, cell
layouts and intrinsics, generated from the engine's own tables.

Code samples marked *live* run in djs-wasm in your browser on the docs site: edit them
and they run again. The output shown under each sample was produced by running it in
this build of djs when the guide was checked, so it cannot drift from what djs actually
does. A sample with a `debugger;` statement pauses there and shows the continuation dump
of the run at that point.
