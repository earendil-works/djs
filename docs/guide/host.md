# The host interface: wire in, reader out

djs never runs on its own. A host (pi's codemode, a test runner, a browser page) loads
the djs-wasm module, gives it a script and the tools the script may call, and reads the
result. This page explains the boundary between the two: the module's exports, how a run
proceeds, how values cross in each direction, how tools are called, and how pi's
codemode sits on top.

> **In short.** djs-wasm is a WebAssembly module with no imports and a small ABI of
> `i32` functions. The host drives a run by calling `vm_run` with a fuel slice and
> acting on the status it returns. Values go in through a compact binary *wire format*
> that the VM decodes in one pass, and come out through the *reader*, which walks the
> VM's arena directly. Tool calls park the VM or return promises, and their results are
> applied as macrotasks, so a run does not depend on when results arrive.

## A module that needs nothing

`djs_wasm.wasm` (`crates/djs-wasm`) is a thin export layer over the Rust `Vm`. It has no
imports at all: `WebAssembly.instantiate(module, {})` works in node, in a browser or in
any other wasm runtime. Everything a VM would normally ask its environment for comes in
as data instead. The clock is set before each slice (`vm_set_now`), the random
seed and the time zone are set explicitly, and there are no files or network.

The exports take and return `i32`s (and an `f64` for the clock). Data crosses through
two buffers in linear memory: the host writes into a *staging* buffer and then calls the
export that consumes it, and exports that return data fill an *out* buffer. VMs and the
values the host keeps across calls are small integers: a value is a *handle*, an index
into a table of GC roots, freed with `vm_handle_free`. A VM created with `vm_new` is a
copy of the base snapshot, so creating one is a memory copy, not an
initialization. The module carries that snapshot compressed with its own small codec, to
keep the download small, in a custom section named `djs_base`, outside the
instance's linear memory: a copy in memory would cost every instance a page for bytes it
reads once. So a host hands the section to a fresh instance
(`djs_base_coded_ptr`, then write the bytes), and the instance decodes it when it creates
its first VM, which takes a few milliseconds. Because every sandbox is its own instance,
the host decodes it only once per compiled module: it keeps the first instance's decoded
copy and writes it into each new instance instead, which checks its length and hash
before using it. The decoded or written bytes then become the first VM's arena as they
are: the instance keeps no copy of its own, 0.31 MiB less wasm memory per sandbox
. Only an instance that makes a second VM decodes the base again (from the coded
section, handed over once more), and keeps that copy for any further VMs. A fresh
sandbox is 11 wasm pages (704 KiB): a 128 KiB stack, the static data, the allocator's
first pages and the base.

A panic in the engine (a bug, never something a script can cause: the heap is bounds
checked and every limit is a thrown error) records a message and traps. The host reads
the message and discards the instance. The shipped module records a fixed message: it
carries no panic messages, panic locations or formatting code, to stay small. A debug or native build of the engine reproduces the real message. Every sandbox gets its own instance, so one trap
cannot affect another. One trap is not a bug: the parser's nesting limits keep its
recursion within the 128 KiB wasm stack, but should some path still exhaust it, the
stack pointer wraps below 0 and the instance traps with "memory access out of bounds".
The module exports its stack pointer so that the host can tell: the `Sandbox` and pi's
adapter then report "stack overflow: the engine's wasm stack is exhausted" instead of a
bare out-of-bounds access, and never call into that instance again.

There are about 45 exports; the `Sandbox` class below wraps them, so most hosts never
call them directly.

> **Trade-off.** An import-free ABI of integer functions over a richer binding such as
> the component model or wasm-bindgen. The module runs anywhere with nothing to wire up,
> and the boundary is small enough to check in a table. The cost: the host does the
> bookkeeping (handles, buffers, re-deriving its views of memory after every call, since
> any call can grow memory), which is why the `Sandbox` class exists.

## Driving a run

A run is a loop on the host side. The host compiles the script as a codemode function
body (`vm_compile`), starts the call (`vm_call`) and then calls `vm_run` with a fuel
slice. `vm_run` returns a status, the host acts on it and runs again:

![How a host drives a run](img/run-loop.svg)

The fuel slice is how the host keeps control. A slice of a million fuel takes a few
milliseconds of work (about 3 ms on a tight loop, measured through the pi adapter); a
long leaf operation can overshoot its slice, and the overshoot is charged to the next
one. Between slices the host can check a deadline, cancel the run with
`vm_abort`, or simply leave the run where it stopped (the goal to [suspend
anywhere](goals.md#suspend-anywhere)). `vm_run` also runs the promise job queue to empty
, so DONE means the script and everything it started have finished. One more
status ends a run without an error from the script: HOST_OUT_OF_MEMORY, when the host
cannot back the heap below its cap. The call is abandoned and the VM stays
usable.

`js/src/sandbox.ts` (the `@djs/host` package) wraps this loop:

```ts
import { Sandbox } from "@djs/host";

const sb = await Sandbox.create({
  functions: { readFile: (p) => fs.readFileSync(p as string, "utf8") },
  asyncFunctions: { fetchJson: (url) => fetch(url as string).then((r) => r.json()) },
  memoryLimit: 64 << 20,
  fuelSlice: 1_000_000,
});
sb.run("return readFile(path).length", { path: "README.md" });
await sb.runAsync("return await fetchJson(url)", { url });
```

## Values in: the wire format

Tool results are the data a codemode script receives: long strings, arrays of similar
objects, base64 images. Converting them into VM values is on the hot path (the goal of
[cheap data passing](goals.md#cheap-data-passing)). The usual way, `JSON.stringify` on
the host and `JSON.parse` in the VM, parses text that the host has just produced, and
has to escape and copy every string twice.

djs uses its own binary format instead. `js/src/wire.ts` encodes a host value in
one pass: tagged numbers (`int8`, `int32`, `float64`), strings as UTF-8 written with
`TextEncoder.encodeInto`, arrays, objects whose repeated keys are interned in a key
table, and `Uint8Array`s as raw bytes. The VM's decoder (`vm_decode_wire`) allocates
every cell at its final size in one pass and transcodes UTF-8 to the VM's one-byte or
two-byte strings natively. The format is validated as it is decoded: malformed input is
an error, never a corrupt heap.

A large string on its own skips the staging buffer when it is ASCII, which is the common
case for a tool result that pi passes as JSON text. The encoder's output already
shows it: the UTF-8 is exactly as long as the string. The host then asks the VM for an
empty one-byte string of that length (`vm_string_reserve`), gets the address of its
characters, and copies the encoded bytes straight there. That saves the staging copy and
the transcode, and the staging buffer, which would otherwise keep the size of the
largest result for the life of the instance, does not grow: a 10 MiB result costs a
sandbox 11.4 instead of 21.4 MiB of wasm memory. Any other string goes through the
wire, so text with a single non-ASCII character pays nothing for the attempt.

> **Trade-off.** A binary format over JSON. Measured end to end in wasm, it beats
> `JSON.stringify` plus `JSON.parse` by 1.4x on objects up to 9x on one large string (a
> large string becomes two memory copies instead of an escaping serializer and a
> parser). The one place it loses is the encoder on many small objects, where V8's
> native `JSON.stringify` is faster than the JavaScript encoder; the decoder more than
> makes up for it. The cost: a format and two implementations (TypeScript and Rust) to
> keep in agreement, which the gate checks with cross-language tests.

## Values out: the reader

Going the other way, the VM does no work at all. The host reads a value straight out of
the arena with `js/src/reader.ts`: a value is 64 bits (see [Values](values.md)), a
reference is an offset into the arena, and the reader decodes cells in place. A one-byte
string becomes a host string through `String.fromCharCode` over the bytes, a two-byte
one through `TextDecoder("utf-16le")`, and objects and arrays are walked.

The reader hard-codes nothing about the heap. It uses `js/src/layout.generated.ts`,
which is exported from the same `cells!` table the collector and the image verifier use,
so a change to a cell layout is a regeneration, not a reader rewrite. The reader
checks the layout hash in the roots cell and refuses an arena from another build.

What the host gets is a tree, like JSON: plain objects with the own enumerable
string-keyed data properties, arrays with their holes, typed arrays as copies, BigInts
as `bigint`. Functions, symbols and other objects come back as `Opaque` markers. A cycle
is a `TypeError` naming its path.

> **Trade-off.** Reading the arena directly over serializing in the VM. Structured data
> reads 3x to 130x faster than producing JSON in the VM and parsing it on the host, and
> a 1 MiB string reads at memory-copy speed. The cost: the reader depends on the heap
> layout (made safe by generating it from the engine's own table), the VM must not run
> while a read is in progress (a collection moves cells), and sharing is not preserved,
> so a value with the same object at many places is read once per reference (a value
> budget turns a blow-up into an error).

## Host calls

A tool is a host function. What identifies it is a name its host chooses, such as
`tools.read` or `@sdk/our-stuff#myTool`, not where scripts find it: the host
binds it wherever it likes, as a global, as a member of a namespace object like `tools`,
or as a parameter of the script's body, so a script can call a bare `myTool("x")` as it
later will after `import { myTool } from "@sdk/our-stuff"`. The Sandbox takes
the functions by name and two small tables, `globals` and `bindings`, that say where they
appear; without `globals`, a name like `tools.read` makes a global `tools` holding
`read`. Calls, journals and dumps always use the name. There are two kinds:

- A **sync host function** (`vm_new_host_function`) parks the VM. `vm_run` returns
  HOST_CALL, the host reads the arguments with the reader, calls the function, sends the
  result in through the wire format and resolves the call. The pending call is part of
  the heap, like everything else.
- An **async host function** (`vm_new_async_host_function`) returns a promise in
  the VM at once, and the script goes on. Several calls can be outstanding together, as
  in `Promise.all(files.map(readFile))`. When nothing but pending calls is left,
  `vm_run` returns IDLE and the host waits for one.

Results of async calls can come back at any time: after a fuel slice, while the VM is
parked on another call, several at once. If the VM applied each result as soon as the
host delivered it, the order in which promise callbacks run would depend on the fuel
slice and on network timing. So results are **macrotasks**: they wait in an
inbox in the heap, and `vm_run` takes the oldest one only when the job queue is empty,
then drains the queue again before the next. What a script does depends only on the
order of the results, as with I/O callbacks in an event loop.

That property is what deterministic mode builds on: with a seeded `Math.random`,
a virtual clock that moves only when a result is delivered, and weak references made
strong or absent, the same code and the same results in the same order give the same
run. A run can be recorded as a journal and replayed, checked by canonical heap hashes
at every idle point. [Determinism](determinism.md) covers this in depth.

> **Trade-off.** Results as macrotasks over delivering them immediately. Ordering by
> delivery, not by arrival time, makes runs reproducible and lets a replay deliver the
> results one at a time at any fuel slice. The cost: a result that arrives while the
> script is busy waits until its current job queue drains, which is the same latency an
> event loop has, and the host must not assume that a resolved call has run its
> callbacks by the time `vm_resolve_host_call` returns.

> **Trade-off.** Sync host functions park the whole VM. That keeps them simple to
> implement and to suspend (a parked call is just a pending entry in the heap), but
> only one can be outstanding at a time. Scripts that call tools concurrently need async
> host functions.

## Persisting a run that waits

An async call can take longer than the process that made it should live: a tool that
waits for a person, or a host that restarts. While a run waits only on async calls, the
VM is idle: no frame on the stack, no job queued. At such a point the run can be written
out as JavaScript, a continuation dump: its data, the async functions stopped at their
`await`s, the open calls by the host's names for them, and the run's result promise.
Loaded in a new sandbox, perhaps in another process, the run waits on the same calls
with the same ids.

What is not in the VM is not in the dump by itself: a timer, a request in flight, a
person's pending answer. Those are the host's, so each open call can carry a piece of
host state, JSON the dump writes next to the call and the load gives back: when a
sleep is due, which request to poll. The engine keeps none of it between dumps; it only
checks that the text is JSON, so it cannot smuggle code into the dump.

The Sandbox turns that into a few lines of host code. An async host function may
be declared with hooks:

```ts
const sleep = {
  async call(ms) {                          // the script called it
    const due = Date.now() + Number(ms);
    this.state = { due };                   // the call's state: a dump keeps it
    await until(due, this.signal);
  },
  async resume({ due }, call) {             // after a load, with that state
    await until(due, call.signal);
  },
};
const task = sandbox.start(code, params);   // runs like runAsync
const dump = await task.suspend();          // at the next idle point
// ... any time later, any process:
const result = await (await Sandbox.resume(dump, options)).result;
```

`start` runs the script and calls the host functions as the script calls them.
`suspend()` waits for the next idle point, asks every open call for its state (a `save`
hook, or the state the call recorded), writes the dump and stops the run; the calls'
abort signals tell the host to stop waiting for them. `Sandbox.resume` loads the dump
into a new sandbox and continues each open call through its function's `resume` hook. A
function without `resume` is called again, which suits calls without side effects; a
plain function, without hooks, has its call rejected, because the sandbox cannot know
whether calling it twice is safe. [`examples/sleep-dump/`](../../examples/sleep-dump/)
is such a host, with a tool that sleeps a second on the host side, in one process or
two.

A host that wants to decide every call itself drives the run instead: `begin`
runs the script to its first idle point and lists the open calls instead of calling the
functions, the host answers each with `settle` and moves on with `advance`, and while
the run is idle `dump(states)` writes it out with the states the host gives. `loadRun`
loads such a dump into a fresh sandbox and returns the run with its open calls and their
states; for each, the host delivers a result it recorded, issues the call again or
rejects it.

A dump is plain JavaScript and stays plain: the host compresses it when it stores it,
with zstd at level 3, which node has built in. Over a corpus of 300 real codemode
scripts, a dump at an idle point is 6 KB raw and under 2 KB compressed at the median, and
a session holding a large log a few hundred KB. Any zstd reads it back, and any later
djs loads it.

Dumps are **plain JavaScript**: the sandbox writes each stopped function as a
continuation, an ordinary function holding the rest of its code from the `await` it waits
at, and the whole dump as a Script that needs nothing from djs. The Script reaches the
host through one global object, `$djs`, which the loading host defines and deletes when
the Script has run, with the dump's own `$dump`: `$djs.seed(lo, hi, next)` hands over `Math.random`'s
state and the id of the run's next host call, each only where needed (the state where the
run called `Math.random`, the id where it is not one past the highest reopened call's;
`$djs.seed(next)` for the id alone), `$djs.resume(id, name, args, state)`
stands for each open call, and `$djs.source(name, hash, params)` names the script the run
was started with by its name and a 32-bit hash, which the dump refers to instead
of carrying its text a second time: the sandbox compiles that script again and checks it
is the same one. A dump is strict code without a `"use strict"` directive: every
djs host compiles it strict.
`$djs.host(name)` stands for a host value the run holds, such as a tool function it put
in a variable or the `tools` object itself: the dump names it by the name the host gave
it, never by where the script found it, and the host hands back the very object it bound
under that name, so a dump does not depend on a host function being a global. The
sandbox answers `resume` by opening the call again under
the same id, with the same arguments and state, and continues it through the function's
hooks as above; once the Script has stopped it sets the call counter from `next`, so the
run's later calls get the ids an uninterrupted run gives them. The same object
holds the small helpers continuations call, the dump runtime, which the host provides so
that dumps do not carry them. A host other than djs defines `$djs` itself:
[`examples/sleep-dump/`](../../examples/sleep-dump/) continues the same dump in node.

Only a sandbox made with frozen builtins can dump: the writer relies on no
builtin changing between the moment the run read a value and the load. Without them,
`dump`, `Run.dump` and a task's `suspend` and `checkpoint` throw a `misuse` error that
says so. pi's sessions run with frozen builtins anyway. Loading a dump needs neither
frozen builtins nor the dumper.

The dumper is about 300 KB of code (108 KB compressed) that only hosts which persist runs
need, so the shipped module leaves it out. Such a host loads a second module, the
same build with the dumper, and passes it to the sandbox: dumping copies the VM's image
into an instance of that module and writes the dump there, and the process keeps that
instance, shared by its sandboxes, for the next dumps. The image never
leaves the process, so neither module verifies it cell by cell, only that it was made by
the same build's tables. Loading is running the dump, so the shipped module does it
alone.

pi loads the same texts again at every idle point, each time in a new instance: the
script the run started with (the answer to `$djs.source`) and small fixed snippets, and a
dump needs the suspension maps of that same script. The dump module therefore keeps a
**compile cache** across VMs. A compile done through `vm_compile_cached`
records what it did: the cells it appended to the arena and every atom it interned, in
order; the host keeps that record per module, keyed by the text, its file name, parameters
and options. The next VM imports the record instead of compiling: it copies the cells,
rebases their references, and interns the same names at the same points, so it ends up as
if it had compiled the text itself, byte for byte (the difftest's determinism check
compares the two heaps for every case). Suspension maps travel the same way, as bytes
from one VM's dump to the next VM (`vm_dump_maps_export`, `vm_dump_maps_import`). In pi's
sessions this saves about 6% of the instructions per session; the import costs about a
quarter of a compile.

## Where to look

- [`crates/djs-wasm/src/lib.rs`](../../crates/djs-wasm/src/lib.rs): the exports.
  [`crates/djs-core/src/wire/`](../../crates/djs-core/src/wire/): the decoder.
- [`js/src/`](../../js/src/): `sandbox.ts`, `wire.ts` (encoder), `reader.ts`,
  `layout.generated.ts`.
- [`js/pi/`](../../js/pi/): the pi adapter and the runner for pi's own tests.
