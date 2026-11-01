# The host interface: wire in, reader out

djs never runs on its own. A host (pi's codemode, a test runner, a browser page) loads
the djs-wasm module, gives it a script and the tools the script may call, and reads the
result. This page explains the boundary between the two: the module's exports, how a run
proceeds, how values cross in each direction, how tools are called, and how pi's codemode
sits on top.

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
as data instead. The clock is set before each slice (`vm_set_now`, D1383), the random
seed and the time zone are set explicitly, and there are no files or network.

The exports take and return `i32`s (and an `f64` for the clock). Data crosses through two
buffers in linear memory: the host writes into a *staging* buffer and then calls the
export that consumes it, and exports that return data fill an *out* buffer. VMs and the
values the host keeps across calls are small integers: a value is a *handle*, an index
into a table of GC roots, freed with `vm_handle_free`. A VM created with `vm_new` is a
copy of the base snapshot (D1321), so creating one is a memory copy, not an
initialization.

A panic in the engine (a bug, never something a script can cause: the heap is bounds
checked and every limit is a thrown error) records its message and traps. The host reads
the message and discards the instance. Every sandbox gets its own instance, so one trap
cannot affect another.

docs/host.md has the full export table and the call protocol.

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
milliseconds of work (about 3 ms on a tight loop, measured for the pi adapter in
docs/host.md); a long leaf operation can overshoot its slice, and the overshoot is
charged to the next one (D1740). Between slices the host can check a deadline, cancel
the run with `vm_abort`, or take a snapshot of a run that waits for a slow tool (G5).
`vm_run` also runs the promise job queue to empty (D1370), so DONE means the script and
everything it started have finished. One more status ends a run without an error from
the script: HOST_OUT_OF_MEMORY, when the host cannot back the heap below its cap (D1690).
The call is abandoned and the VM stays usable.

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
const snap = sb.bytes(); // the VM image plus the names of the host functions
```

## Values in: the wire format

Tool results are the data a codemode script receives: long strings, arrays of similar
objects, base64 images. Converting them into VM values is on the hot path (G1). The usual
way, `JSON.stringify` on the host and `JSON.parse` in the VM, parses text that the host
has just produced, and has to escape and copy every string twice.

djs uses its own binary format instead (docs/wire.md, D1180). `js/src/wire.ts` encodes a
host value in one pass: tagged numbers (`int8`, `int32`, `float64`), strings as UTF-8
written with `TextEncoder.encodeInto`, arrays, objects whose repeated keys are interned in
a key table, and `Uint8Array`s as raw bytes. The VM's decoder (`vm_decode_wire`) allocates
every cell at its final size in one pass and transcodes UTF-8 to the VM's one-byte or
two-byte strings natively. The format is validated as it is decoded: malformed input is
an error, never a corrupt heap.

> **Trade-off.** A binary format over JSON. Measured end to end in wasm, it beats
> `JSON.stringify` plus `JSON.parse` by 1.4x on objects up to 9x on one large string (a
> large string becomes two memory copies instead of an escaping serializer and a parser).
> The one place it loses is the encoder on many small objects, where V8's native
> `JSON.stringify` is faster than the JavaScript encoder; the decoder more than makes up
> for it. The cost: a format and two implementations (TypeScript and Rust) to keep in
> agreement, which the gate checks with cross-language tests.

## Values out: the reader

Going the other way, the VM does no work at all. The host reads a value straight out of
the arena with `js/src/reader.ts`: a value is 64 bits (see [Values](values.md)), a
reference is an offset into the arena, and the reader decodes cells in place. A one-byte
string becomes a host string through `String.fromCharCode` over the bytes, a two-byte one
through `TextDecoder("utf-16le")`, and objects and arrays are walked.

The reader hard-codes nothing about the heap. It uses `js/src/layout.generated.ts`, which
is exported from the same `cells!` table the collector and the image verifier use, so a
change to a cell layout is a regeneration, not a reader rewrite (D1210). The reader
checks the layout hash in the roots cell and refuses an arena from another build.

What the host gets is a tree, like JSON: plain objects with the own enumerable
string-keyed data properties, arrays with their holes, typed arrays as copies, BigInts
as `bigint`. Functions, symbols and other objects come back as `Opaque` markers. A cycle
is a `TypeError` naming its path.

> **Trade-off.** Reading the arena directly over serializing in the VM. Structured data
> reads 3x to 130x faster than producing JSON in the VM and parsing it on the host, and a
> 1 MiB string reads at memory-copy speed. The cost: the reader depends on the heap
> layout (made safe by generating it from the engine's own table), the VM must not run
> while a read is in progress (a collection moves cells), and sharing is not preserved,
> so a value with the same object at many places is read once per reference (a value
> budget turns a blow-up into an error).

## Host calls

A tool is a host function, installed as a global or passed as a parameter. There are
two kinds:

- A **sync host function** (`vm_new_host_function`) parks the VM. `vm_run` returns
  HOST_CALL, the host reads the arguments with the reader, calls the function, sends
  the result in through the wire format and resolves the call. While the VM is parked
  the host may also snapshot it: the pending call is part of the image.
- An **async host function** (`vm_new_async_host_function`, D1374) returns a promise in
  the VM at once, and the script goes on. Several calls can be outstanding together, as
  in `Promise.all(files.map(readFile))`. When nothing but pending calls is left,
  `vm_run` returns IDLE and the host waits for one.

Results of async calls can come back at any time: after a fuel slice, while the VM is
parked on another call, several at once. If the VM applied each result as soon as the
host delivered it, the order in which promise callbacks run would depend on the fuel
slice and on network timing. So results are **macrotasks** (D1670): they wait in an
inbox in the heap, and `vm_run` takes the oldest one only when the job queue is empty,
then drains the queue again before the next. What a script does depends only on the
order of the results, as with I/O callbacks in an event loop.

That property is what deterministic mode builds on (D1680): with a seeded `Math.random`,
a virtual clock that moves only when a result is delivered, and weak references made
strong or absent, the same code and the same results in the same order give the same
run. A run can be recorded as a journal and replayed, checked by canonical heap hashes
at every idle point (D1730). [Determinism](determinism.md) covers this in depth;
docs/host.md has the journal format.

> **Trade-off.** Results as macrotasks over delivering them immediately. Ordering by
> delivery, not by arrival time, makes runs reproducible and lets a replay deliver the
> results one at a time at any fuel slice. The cost: a result that arrives while the
> script is busy waits until its current job queue drains, which is the same latency an
> event loop has, and the host must not assume that a resolved call has run its
> callbacks by the time `vm_resolve_host_call` returns.

> **Trade-off.** Sync host functions park the whole VM. That keeps them simple to
> implement and to snapshot (a parked call is just a pending entry in the heap), but
> only one can be outstanding at a time. Scripts that call tools concurrently need
> async host functions.

## The pi adapter

pi's codemode runs scripts on QuickJS through one module, `src/runtime/vm.ts`, with a
small contract: start a VM for one script, pass messages to the host (`post`), take
results (`deliver`), and poll whether to stop. `js/pi/vm.ts` implements the same
contract on djs and runs pi's unmodified prelude, so trying djs in pi means swapping
that one module. `js/pi/engine.ts` is a handle-level client of the exports, because pi
keeps its prelude's functions alive across calls, which the `Sandbox` (whole function
bodies only) does not do.

What the adapter adds over QuickJS:

- **Prelude images.** Running pi's prelude, which freezes about 700 objects, is most of
  a start. The adapter keeps the image taken right after it, collected first (D1890),
  keyed by everything the prelude saw, and restores it on the next start with the same
  inputs (D1495). The prelude's own work is not counted against the script's interrupt
  budget, so cold and warm starts behave the same (D1660). A restore is a copy plus a
  check: a script starts in about 1 ms instead of 7 ms, against about 4 ms on
  quickjs-wasi.
- **Snapshots of a waiting session.** `snapshot()` and `resumeVm()` save and restore a
  session while it waits for a tool, which QuickJS cannot do. pi uses deterministic
  replay for durability today; images would be an alternative.

pi's own test suite runs against djs through this adapter (`node js/pi/pi-tests.ts`):
221 of its 226 tests pass. The rest come from djs being always strict and from a test
that expects QuickJS's fingerprint (docs/host.md "pi's own tests on djs").

> **Trade-off.** Implementing pi's existing VM contract over designing a new one around
> djs. pi can adopt djs behind a flag by swapping one module, and its tests run
> unchanged. The cost: the adapter carries pi's protocol (a sync `bridge` function,
> polling between fuel slices instead of an interrupt handler), and the benefits djs
> could offer beyond QuickJS, such as snapshots of waiting sessions, are extras that pi
> would have to opt into.

## Where to look

- [docs/host.md](../host.md): the ABI table, the Sandbox API, journals, the pi adapter
  and its numbers.
- [docs/wire.md](../wire.md): the wire format specification and the reader's mapping,
  with benchmarks.
- `crates/djs-wasm/src/lib.rs`: the exports. `crates/djs-core/src/wire/`: the decoder.
- `js/src/`: `sandbox.ts`, `wire.ts` (encoder), `reader.ts`, `layout.generated.ts`.
- `js/pi/`: the pi adapter and the runner for pi's own tests.
- ARCH.md, "Host Interface".

Decisions on this page:

<!-- gen:decisions D1180 D1210 D1321 D1370 D1374 D1383 D1495 D1660 D1670 D1680 D1690 D1730 D1740 D1890 -->
- D1180: Inbound wire format, version 1
- D1210: Exported heap layout drives the host-side reader
- D1321: The base snapshot (djs-prelude)
- D1370: The job queue, and when `run` reports Done
- D1374: Async host functions: a promise plus an entry in the pending host call table
- D1383: Date reads the host's clock and a fixed local offset
- D1495: pi adapter: reuse the VM image taken after the prelude
- D1660: pi adapter: the prelude is not polled; the fingerprint hashes the host globals
- D1670: Async host results are macrotasks: the host inbox
- D1680: Deterministic mode: weak references collected, strong or absent; the Sandbox's deterministic option
- D1690: Running out of host memory is not a JS out-of-memory error
- D1730: Canonical hash and run journals
- D1740: Fuel overshoot carries to the next run; a suspended intrinsic is charged once
- D1890: pi adapter: snapshot() and the prelude image collect first
<!-- /gen -->
