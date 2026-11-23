# Images and the base snapshot

A host needs to capture a whole VM and bring it back: to stop a run between two fuel
slices and continue it in a fresh VM, to start every script from a ready realm, to move a
VM into a second wasm module. djs does all of this with one mechanism, the **image**: the
arena's bytes, taken as they are. This chapter shows what an image is, what it depends on,
the one image every VM starts from, and how to look inside one.

> **In short.** An image is the arena from offset 0 to its end. Taking one is a copy;
> restoring one is a check of the header and a copy, with no fixups, because every
> reference is an offset into the same bytes. An image loads only into the build that
> made it: its roots carry that build's **build hash**. Every VM starts as a copy of the **base
> snapshot**, an image of the initialized realm made at build time, which is why creating
> a VM takes microseconds. Images stay inside a process; a run that must outlive it is
> written as a [continuation dump](dumps.md) instead.

## An image is the arena

`Vm::image()` (in wasm, `vm_arena_ptr` and `vm_arena_len`) is the image: the arena from
offset 0 to its end. It can be taken whenever `run` has returned: after `Done`, while
`OutOfFuel` in the middle of a loop, while parked on a host call, while idle with ten
tools pending. Nothing is serialized, because nothing needs to be:

- every reference is an offset into the same bytes ([the arena](arena.md));
- the interpreter's registers, the value stack and the frames are cells ([the
  interpreter](interpreter.md));
- pending host calls, queued jobs, the inbox of tool results and the fuel debt are heap
  state ([suspension](suspension.md));
- native code is referred to by index (builtins) or by a host-assigned id (host
  functions), and the embedder's handles are slots in a table in the arena.

Restoring is `Vm::from_bytes`: check the header and the build hash, verify the
cells where the caller asks for it (below), take the bytes as the new arena, and continue.
The host registers its host functions again under the same ids, and its handles are
still valid, because they were slot numbers all along.

```js live
// At every stop of this example (every 50 fuel), the VM below was saved to an image and
// restored into a fresh one: the generator, the closure, the Map, the pending promise
// jobs and the frames in the middle of `map` all travelled through the bytes.
function* ids() { let i = 100; while (true) yield i++; }
const gen = ids();
const seen = new Map();
const tag = (name) => { const id = gen.next().value; seen.set(id, name); return id; };
const tagged = await Promise.all(["a", "b", "c"].map(async (n) => tag(n)));
console.log(tagged.join(" "), [...seen.values()].join(""));
```
<!-- gen:run -->
```text
100 101 102 abc
```
<!-- /gen -->

Taking an image right after a full collection makes it small: no garbage, the live cells
packed in their allocation order ([the garbage collector](gc.md)). Embedders collect
before they keep an image.

An image holds nothing a run merely left behind. The interpreter leaves popped values in
the value stack's slots above the stack pointer and overwrites them later; `Vm::image()`
zeroes them, and so does every collection before it traces the stack. A stack left behind
when the interpreter grows it gets its dead slots zeroed too. So two runs with the same
history give the same image bytes whether, and wherever, they were stopped and restored;
the stress tests compare images taken at arbitrary stops byte for byte
([determinism](determinism.md)). A collection the embedder asks for also shrinks a stack
that grew for a deep recursion back to what it holds: 2,000 nested calls leave a stack of
32K slots (256 KB) that every later image would otherwise carry.

An image also holds the collector's state: the next collection threshold, where the
[old region](gc.md#the-old-region) ends and how many minor collections ran since the last
full one. A restored VM goes on collecting exactly as the original would have.

## What an image depends on

An image contains bytecode, intrinsic ids, object class numbers, slot meanings and cell
layouts. A build of djs that numbers any of those differently, or reads a slot
differently, would misread it. Images never leave the process that made them, so the
simplest key is also the safest: the roots cell carries the engine's **build hash**
(`djs_build_hash` in wasm), and an image loads only into the build with that hash. The
hash covers everything that can change behavior: the cell layouts and well-known atoms,
the opcode and intrinsic tables, djs-core's sources and generated prelude, the sources of
its math library (djs-libm) and the enabled features that change results. It is also
what a run journal needs to replay step by step ([determinism](determinism.md)).

Loading checks the layout hash first (`VERSION`, which the host's reader checks too, since
the layouts are all it reads), then the build hash. An image of another build is a
`LoadError::OtherBuild` with both hashes: "the image is from another djs build (1a2b3c4d;
this build: 5e6f7a8b)".

After the build hash, `from_bytes` **verifies** the image: it walks every cell, checks
every traced field (a valid value pointing at the start of a cell of a fitting kind), the
invariants of each kind, and that prototype chains and ropes are acyclic. The stress
tests and the REPL's heap report load images that way, which makes every restore a check
of the heap. The wasm module does not: the only images it loads were made in the same
process, so it checks the header and the build hash and leaves the verifier out of the
shipped code. A bad image could still only cause a panic (a trap) or a wrong result
inside that one instance: the shipped modules leave heap bounds checks to wasm, which
confines every access to the instance's memory, and every other build checks each
access.

> **Trade-off.** Raw images over a serialization format, inside a process. Images cost no
> code to write or read, restore in a copy, and capture everything, including frames in
> the middle of a builtin, which is what the base snapshot, the stress tests and a stop
> between fuel slices need. Keyed on the build, they need no rule for which engine change
> alters what an image means. A run that has to outlive the process needs something that
> states its meaning instead, which is what a dump is ([below](#continuation-dumps)).

## The base snapshot

Creating a realm means building a few thousand objects: the global object, every
constructor and prototype, a thousand builtins compiled from the prelude. Doing that per
VM would cost milliseconds, and codemode creates a VM per script. So it is done once, at
build time. [`crates/djs-prelude/build.rs`](../../crates/djs-prelude/build.rs) creates a
VM with a native build of djs-core, runs the prelude that bgen generated
([builtins](builtins.md)), compacts and collects the heap, verifies the result, and
writes it out; the library embeds it, and the wasm module carries it coded in a custom
section ([the wasm boundary](host.md#the-base-in-the-module)).

A new VM is a copy of the base (`Vm::from_base`), which checks the header and the
build hash but skips the full walk, since the base was verified when it was
built. That copy **is** startup: once a wasm instance has its base, `vm_new` takes a few
microseconds. The whole base is the new VM's old region, so the collections while a
script runs don't trace it again.

The base ends with a **tail**: the cells every VM writes, put last on purpose. They are
the realm record, `Math.random`'s state, the global object, the global lexical scope,
`Symbol.for`'s registry, Map and Set tables, the atom table's cell with room for the
atoms the VM adds, the host call table, the interpreter state and the value stack, the
queues for promise jobs, host results and unhandled rejections, and the VM's own shape
tree, already holding the shapes of the objects builtins create; <!--n base/tail kb.1-->10.4 KB<!--/n--> in all.
Everything before the tail is the **builtin prefix**, which no VM writes to while
builtins are merely used. The builtins' objects are marked as prototypes when the base is
built, not when a cache first goes through them. Strings in the prefix don't cache their
hashes. Each VM grows its own tree of shapes, so its objects never add transitions to a
builtin's shape. No collection moves or frees the prefix ([the garbage collector](gc.md)),
so the same bytes stay at the same offsets in every VM of the build.

Every VM's memory holds the base, so it is built to be small:

- property tables, the atom table and the value stack are shrunk to their used size
  before the final collection, except that the global object's table keeps room for 16
  more entries: nearly every host adds a few globals (`console`, its tools), and a full
  table would be copied at twice its size, with a new index, in every VM;
- equal strings are stored once: a builtin's `name`, a constant in a function's pool and
  the atom table often hold the same string, and strings can't be told apart by where
  they live, so the base keeps the atom table's copy and points the others at it;
- the prelude keeps no source text, no line tables or local names, its internal helper
  closures get no `length`, `name` or `prototype`, and the installed builtins keep their
  `length` and `name` in their templates rather than as properties;
- small objects share shapes instead of carrying a property table each;
- the functions of the prelude share one constant pool, without the entries only its
  set-up code used;
- atoms that only the prelude's set-up code used are dropped and the rest renumbered to
  close the gaps, rewriting every atom id in the heap: property tables, shapes, symbols,
  templates and the atom operands of the code;
- the final collection is a copying one, so the layout depends only on the object graph,
  not on how the prelude allocated.

The base is <!--n base/bytes kb-->191 KB<!--/n--> in the full build and <!--n slim/base kb-->134 KB<!--/n--> in the
[slim one](builtins.md#two-builds-full-and-slim). Building it twice gives the same bytes,
and the base built natively is byte for byte the realm that djs-wasm builds inside
wasm32, which a test checks, so nothing in it depends on the platform.

> **Trade-off.** A prebuilt realm over initializing at startup. Startup becomes a copy,
> and the builtins' bytecode is never compiled at run time. The cost is memory and size:
> every VM carries the whole base, and every builtin, prototype and helper adds to it,
> which is why the prelude is written to be small and the base is compacted.

## Images in use

Images are the engine's in-process mechanism:

- **The base** is copied into every new VM (above).
- **A native host's own setup.** A Rust host that runs setup code of its own before
  every script (helpers, a frozen realm) seals the heap after it (`Vm::seal`: a full
  collection that makes everything live old) and keeps that image as a snapshot, then
  starts each later script from a copy instead of running the setup again ([the Rust
  interface](rust-api.md)). A wasm host that knows how big its heap will get can create
  its VM with room for it (`vm_new`'s second argument), because growing the arena moves
  it, and in wasm its old block stays behind as a hole.
- **The stress tests and the determinism checks** stop a run every few instructions,
  restore it from its image and compare images byte for byte
  ([determinism](determinism.md)).

## Continuation dumps

An image restores only into a djs whose layouts, opcodes, intrinsics and internal ids
number everything as the image does, and those change often as the engine develops. A run
that waits minutes for a tool or a person must survive a restart and an upgrade of djs
([persistence](goals.md#persistence)). So runs are persisted as **continuation dumps**: a
JavaScript program that recreates the run, taken while it waits for its tools. A dump
says what the run *means* rather than how one build laid it out, so any later djs loads
it. [Continuation dumps](dumps.md) shows what a dump contains and how a host uses one;
[how continuation dumps are written](continuations.md) explains how the writer gets there
from the heap. Inside a process, the writer itself works on an image: the host copies the
VM's image into the module that has the dumper.

## Inspecting a heap

What is in a VM's heap, and why is it as large as it is? The REPL's `.heap` answers both
([the command line](cli.md#the-heap-report)). It reads the live VM's image through the
engine's own heap code, never its own parser: the image is verified as a restore would
verify it, the walker and the layout table say which fields are references and what they
are called, and liveness follows the collector's rules. From that it reports:

- a summary: live and garbage bytes, the old region and the young one, and the VM's own
  part past the base;
- cells and bytes per cell kind and per object class;
- a diff against the base image every VM starts from;
- the builtins a script changed: the words of the builtin prefix that differ from this
  build's base, by cell, named by their builtin paths (`%Array.prototype%`, `%Math%`);
- the largest cells, and the largest *retained* sizes: what a cell keeps alive on its
  own, from the dominator tree of the object graph;
- strings (duplicated contents, the largest), shapes (the most common key lists),
  templates and bytecode per script, closures;
- what a waiting VM holds: queued jobs, pending host calls, promises, and the suspended
  async functions and generators with their locals by name.

An almost empty session's image is little more than the base's <!--n base/bytes kb-->191 KB<!--/n-->. So the
largest cells, strings, shapes and functions describe only the session's own part: the
cells that are new or changed against the base. The base's part is one line of totals.

Every cell gets a path from the roots that reads like JavaScript where the heap allows,
built from the shapes' keys, the elements' indexes, the templates' recorded names of
captured variables and locals, and the layout's field names for everything internal:
`globalThis.cache[3].body`, `handle[1]~pending`, `<atoms><entries>`.

The diff explains what a session added. Cells have no identity but their offset, and a
collection moves them, so the report matches the two images structurally: a breadth-first
walk from both roots pairs the targets of the same field. Immutable cells (strings,
bytecode) pair only with the same content, and a shape's list of transitions pairs its
children by the key they add, since one inserted transition shifts the rest. Then every
cell of the image is the reference's own, in place or moved, a changed one, or new, and
the new ones are grouped into the subtrees that keep them alive. A changed cell counts
the words that differ; one that grew into a bigger cell (a regrown table) compares over
the words both have, and its new words count where they are not zero. For a REPL session
that made 50 small objects:

```text
== diff against the base image
  against the base image (182.0 KB): 91.1% of the image's 8-byte words are the
  reference's at the same offset; cells line up for the first 175.1 KB (90.8%)
  cells                             count     KB  differing KB
  unchanged, in place                3014  174.4
  unchanged, moved                     76    4.3           0.6
  changed                              10    5.9           1.3
  new                                 244    8.3           8.3
  gone (the reference's, not here)      0    0.0

  largest new subtrees (new cells and the new cells only they keep alive):
  KB   cells                 kind  path
  5.2    152         Object/Array  globalThis.notes
  0.9     20         Object/Proxy  globalThis.tools
  0.6     17      Object/Ordinary  globalThis.console
```

> **Trade-off.** Structural matching over offsets. Comparing the two images offset by
> offset would be simpler, but it would call every cell after the first one that moved a
> change, which is exactly the case a size question is about. Matching through the object
> graph names what moved and what it cost; it can pair the wrong cells where two siblings
> are equally plausible, which shows up as a changed cell where there was a new one.

## Where to look

- [`crates/djs-core/src/vm/mod.rs`](../../crates/djs-core/src/vm/mod.rs): `image`,
  `from_bytes`, `from_bytes_trusted`, `from_base`, `seal`, `prepare_base_image` and the
  build hash and the check of an image's.
- [`crates/djs-core/src/heap/walk.rs`](../../crates/djs-core/src/heap/walk.rs): the
  verifier; [`crates/djs-core/src/heap/compact.rs`](../../crates/djs-core/src/heap/compact.rs):
  compaction for the base image; [`crates/djs-core/src/vm/image.rs`](../../crates/djs-core/src/vm/image.rs):
  what the base keeps and drops.
- [`crates/djs-prelude/build.rs`](../../crates/djs-prelude/build.rs): how the base
  snapshot is made; [`crates/djs-prelude/src/lib.rs`](../../crates/djs-prelude/src/lib.rs):
  how a VM is made from it.
- [`crates/djs-cli/src/heap/`](../../crates/djs-cli/src/heap/): the heap report behind
  the REPL's `.heap`.
