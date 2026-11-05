# What djs optimizes for

Every unusual choice in djs serves one of a few goals. This page states each of them:
what it means for someone who uses djs, why djs chose it, and what it costs or rules
out. Five goals are ranked: when two of them pull in different directions, the one
listed first wins. Four requirements hold under all of them. The other chapters refer to
the goals by the names used here.

> **In short.** In priority order: **cheap data passing** between the sandbox and its
> host, **fast startup in wasm**, compiling with **no syntax tree**, **persistence** of a
> waiting run, and the ability to **suspend anywhere**. Under all of them: a script is
> **isolated** from everything its host did not give it, a run is **deterministic**,
> **wasm32 comes first**, and behavior **matches QuickJS**.

## The ranked goals

### Cheap data passing

**For you:** tool results go into a script, and its output comes back to the host, with
as little copying and converting as possible. A 10 MB string costs about two memory
copies, not a serializer and a parser.

**Why:** a codemode script exists to combine tool results: long strings, arrays of
similar objects, base64 images. For many scripts, moving that data is most of the work,
so it ranks first.

**How:** a value encoding the host can decode without help ([values](values.md)), a
heap in one byte array the host reads in place ([the arena](arena.md)), and a compact
binary format for values coming in ([the wasm boundary](host.md)). [The JavaScript
interface](js-api.md#values-in-and-out) says what crosses and how.

**The cost:** the host's reader depends on the engine's heap layout, so it is generated
from the engine's own tables; and what comes out is a tree of plain values, not live
objects.

### Fast startup in wasm

**For you:** a new sandbox is ready in well under a millisecond, the wasm module is
small to download and compile, and each sandbox takes a few hundred KB of memory.

**Why:** a codemode script runs once, for milliseconds, and is thrown away. Whatever
happens before its first instruction (instantiating the engine, building the global
object, compiling the script) is pure latency, and every sandbox's memory counts when a
host runs many.

**How:** the initialized realm ships as a prebuilt image, so creating a VM is a copy
([images and the base snapshot](snapshots.md#the-base-snapshot)); the compiler works in
one pass (next goal); and most builtins are compact bytecode rather than Rust
([builtins](builtins.md)), with a slim build for hosts that count every kilobyte
([the language](language.md#the-slim-build)).

**The cost:** the base image is about a quarter of the module, so every builtin added
has a size price that is weighed; and code that would be faster as native Rust often
stays bytecode.

### No syntax tree

**For you:** scripts compile fast and in little memory, and TypeScript runs directly:
its types are skipped while parsing, with no separate transpile step.

**Why:** scripts are written fresh every time, so compile time is part of every run, and
the models that write them often write TypeScript.

**How:** like QuickJS, the parser emits bytecode while it parses, without building a
syntax tree first ([the compiler](compiler.md)), and the same pass erases types
([TypeScript](typescript.md)).

**The cost:** the parser must settle every ambiguity before it emits code, which makes
some constructs (arrow functions, destructuring, TypeScript's generics) harder to
compile, and optimizations that need a whole function's tree are out of reach.

### Persistence

**For you:** a run that waits for a tool, minutes or days, can be written out, the
sandbox thrown away, and the run continued later, in another process or with a later
version of djs.

**Why:** a script may wait for a slow tool or a person. Keeping its sandbox in memory all
that time is wasteful, and a restart or an upgrade would lose it.

**How:** at an **idle point**, where the run only waits for its tools, djs writes it as a
[continuation dump](dumps.md), a JavaScript program that recreates it and any later djs
runs. Inside one process a copy of the arena does the same job for any stop ([the
arena](arena.md)), but such an image is tied to how one engine build numbers its
internals, so it is not what a run is kept in.

**The cost:** a run can be persisted only while it waits, not in the middle of a
computation; writing dumps needs frozen builtins and the dumper, a second module; and a
few values (weak collections, objects with private fields) cannot be written.

### Suspend anywhere

**For you:** the host keeps control. A script that loops forever, runs too long or
should be cancelled is stopped between two instructions, also inside a `map` callback,
a getter or a promise job, and can continue later in the same process.

**Why:** scripts are untrusted and may be wrong, and the thread that runs them belongs
to the host.

**How:** an interpreter that never keeps JavaScript state on the Rust stack ([the
interpreter](interpreter.md)), fuel as the unit of work ([limits](limits.md),
[suspending anywhere](suspension.md)), and builtins that can stop in the middle
([builtins](builtins.md)).

**The cost:** every call goes through frames in the heap instead of the native stack,
and every builtin that runs user code has to be written so that it can stop.

## Under every goal

### Isolation

**For you:** a script reaches nothing its host did not give it. It has no file system,
network, clock or randomness of its own, and it cannot take more memory, time or stack
than the host allows.

**Why:** the scripts are written by a model, from inputs that may be hostile, and run
inside the host's process.

**How:** the engine does no I/O at all: the clock, the random seed and every tool are
inputs from the host ([the language](language.md#what-is-left-out-and-why)). It runs in
its own wasm instance with no imports, every heap access is bounds checked, and fuel,
a memory cap and a stack limit bound a run ([limits](limits.md)). An engine bug traps
the instance instead of corrupting the host.

**The cost:** everything a script may do outside its memory is a host function the host
has to write; and the engine has no timers or event loop of its own.

### Determinism

**For you:** the same script with the same inputs, delivered in the same order, does the
same thing every time, however it was sliced, stopped and resumed. A run can be
recorded and replayed.

**Why:** it makes suspension testable (a run stopped a thousand times must equal a run
in one go), makes a misbehaving script reproducible, and makes replaying a run after a
crash safe.

**How:** every source of nondeterminism is an input the host sets, tool results take
effect in delivery order, and the heap is laid out as a function of the program
([determinism](determinism.md)).

**The cost:** some common engine techniques are ruled out (hashing by address, weak
shape transitions, collections at moments that depend on timing), and weak references
are made strong or absent in deterministic mode.

### wasm32 first

**For you:** djs is at its best inside a JavaScript host as a WebAssembly module, in
node, a browser or another wasm runtime.

**Why:** that is where codemode sandboxes run.

**How:** memory and speed are judged in V8's wasm tier, not natively. Layouts assume a
32-bit address space, memory grows in 64 KiB pages and never shrinks, and code size
matters as much as speed.

**The cost:** a native build works and is tested, but it is not tuned for; and a heap
is limited to what 32-bit offsets reach.

### QuickJS's behavior

**For you:** what a script prints, including the text of error messages, is what
QuickJS prints for it, and the language is current ECMAScript in strict mode.

**Why:** djs takes the place of QuickJS in agents' codemode, and both the models and the
hosts' tests read what the engine prints.

**How:** where the specification leaves room, djs does what QuickJS does, and every
deliberate difference is declared in the tests ([how djs is tested](testing.md)).

**The cost:** some messages and formats follow QuickJS where another engine's would be
clearer.

## Non-goals

djs is always in strict mode. It has no `with`, no direct `eval`, no ES modules
(`import()` compiles and rejects), no `Intl`, `Temporal`, `SharedArrayBuffer` or
workers, and no JIT. Each of these would cost size or complexity for scripts that do not
need them ([the language](language.md) lists what is left out and why). Its conformance
target is the part of the test262 conformance suite that codemode scripts can reach, not
test262 for its own sake.

djs is not a general-purpose runtime either: it has no event loop of its own, no timers,
no file system and no network. Everything a script may do outside its own memory is a
host function the host chose to give it.
