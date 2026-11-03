# What djs optimizes for

Every unusual choice in djs serves one of five goals, and when two goals pull in
different directions, the one listed first wins. This page states each goal, why a
codemode script needs it, and which chapters show how djs meets it. Two constraints and
a list of non-goals complete the picture. The other chapters refer to the goals by the
names used here.

> **In short.** In priority order: **cheap data passing** between the sandbox and its
> host, **fast startup in wasm**, compiling with **no syntax tree**, **persistence** of a
> waiting run, and the ability to **suspend anywhere**. Under all of them: wasm32 is
> the target that counts, and behavior matches QuickJS.

## Cheap data passing

A codemode script exists to combine tool results: long strings, arrays of similar
objects, base64 images. They have to get into the VM, and the script's output has to get
out, with as little copying and conversion as possible, because for many scripts moving
data is most of the work.

djs meets it with a value encoding the host can decode without help
([values](values.md)), a heap in one byte array the host reads in place ([the
arena](arena.md)), and a compact binary format for values coming in ([the host
interface](host.md)).

## Fast startup in wasm

A codemode script runs once, for milliseconds, and is thrown away. Whatever happens
before its first instruction (instantiating the engine, building the global object,
compiling the script) is pure latency, and a large wasm module costs download and
compile time in the host.

djs meets it by shipping its initialized realm as a prebuilt image, so creating a VM is
a copy ([images](snapshots.md#the-base-snapshot)), by compiling in one pass (next goal), and by
counting every kilobyte of the module, which is why most builtins are bytecode rather
than Rust ([builtins](builtins.md)).

## No syntax tree

Scripts are written fresh every time, so compile time is part of every run. djs compiles
the way QuickJS does: a parser that emits bytecode while it parses, without building a
syntax tree first. That keeps compilation fast and its memory small, at the price of a
parser that must settle every ambiguity before it emits code ([the
compiler](compiler.md)).

## Persistence

A script may wait minutes for a tool, or a person. The host should be able to write the
run out while it waits (its data, the functions stopped at their `await`s, the promises
between them, the open tool calls), release the memory, and continue later, in another
process or after an upgrade of djs.

djs meets it at **idle points**, where a run only waits for its tools: there it writes the
run as a continuation dump, a JavaScript program that recreates it, which any later djs
loads and continues or refuses with a reason ([suspension](suspension.md#persisting-a-waiting-run),
[images and dumps](snapshots.md)). Inside one process a copy of the arena does the same
job for any stop ([the arena](arena.md)), but such an image is tied to the engine build's
numbering, so it is not what a run is kept in.

## Suspend anywhere

The host must keep control: a script may loop forever, must not hog the thread that runs
it, and may be cancelled. Execution therefore has to be able to stop between any two
instructions, also inside a `map` callback, a getter, a `toString` call or a promise
job, and continue later in the same process.

djs meets it with an interpreter that never keeps JavaScript state on the Rust stack
([the interpreter](interpreter.md)), with fuel as the unit of work ([suspending
anywhere](suspension.md)), and with builtins that can stop in the middle
([builtins](builtins.md)).

## Two constraints

**wasm32 is the target that counts.** djs runs mainly as a WebAssembly module inside a
JavaScript host, so memory and speed are judged in V8's wasm tier, not natively (D016).
Layouts assume a 32-bit address space, memory grows in 64 KiB pages and never shrinks,
and code size matters as much as speed.

**Behavior matches QuickJS.** djs replaces QuickJS in pi's codemode, and both models and
pi's tests read what the engine prints, down to the text of error messages. Where the
specification leaves room, djs does what QuickJS does, and every deliberate difference
is declared in the tests ([how djs is tested](testing.md)).

## Non-goals

djs is always in strict mode. It has no `with`, no direct `eval`, no ES modules
(`import()` compiles and rejects), no `Intl`, `Temporal`, `SharedArrayBuffer` or
workers, and no JIT. Each of these would cost size or complexity for scripts that do not
need them. Its conformance target is the part of the test262 conformance suite that
codemode scripts can reach, not test262 for its own sake.

Decisions on this page:

<!-- gen:decisions D016 D008 D1100 -->
- D016: wasm32 is the primary target for performance and memory design
- D008: Codemode top level is a function body
- D1100: Difftest compares what pi's codemode shows the model
<!-- /gen -->
