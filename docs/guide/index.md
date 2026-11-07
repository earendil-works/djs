# djs

<p class="lead">A small JavaScript and TypeScript engine for the scripts an agent writes:
programs that call tools, combine their results and return what matters. djs runs them
in a WebAssembly sandbox, and it can stop a run at any instruction, write it out while it
waits for a tool, and continue it later, somewhere else.</p>

<p class="cta"><a class="button" href="introduction.html">Read the guide</a><a class="button secondary" href="https://github.com/earendil-works/djs">Source on GitHub</a></p>

```ts
import { Sandbox } from "@earendil-works/djs";

const sandbox = await Sandbox.create({
  asyncFunctions: { search: async (q) => mySearch(q as string) },
});
const count = await sandbox.runAsync(
  "const hits = await search(q); return hits.length",
  { q: "small js engines" },
);
```

<div class="features">

- **[One arena](arena.md)** All of a run's state (objects, strings, bytecode, call
  frames, pending tool calls) lives in one byte array addressed by offsets. A new VM is
  a copy of a prebuilt image, made in microseconds.
- **[Suspend anywhere](goals.md#suspend-anywhere)** The interpreter never recurses on
  the native stack for JavaScript calls, so it can stop between any two instructions and
  resume, also inside a getter, a `map` callback or an async function.
- **[Persistence](goals.md#persistence)** A run that waits on its tools can be written
  out as a [continuation dump](dumps.md): a plain JavaScript program that recreates it,
  in another process or a later djs.
- **[Cheap data passing](goals.md#cheap-data-passing)** Tool results go in through a
  compact wire format; results come out by reading the heap from the host, with no
  serialization step inside the VM.
- **[No syntax tree](goals.md#no-syntax-tree)** A single pass turns source into stack
  bytecode, the way QuickJS does. TypeScript's types are erased while parsing.
- **[Determinism](goals.md#determinism)** Time, randomness and weak references come
  from the host, so the same inputs give byte-identical heaps, natively and in wasm.

</div>

djs is written in `no_std` Rust and behaves like QuickJS, down to its error messages. It
passes <!--n test262/passed-->38,973<!--/n--> of the <!--n test262/active-->39,017<!--/n-->
test262 tests it targets, and its wasm module is <!--n wasm/brotli kb-->350 KB<!--/n-->
with brotli. [What djs is](introduction.md) is the place to start;
[the JavaScript interface](js-api.md) shows how to embed it.
