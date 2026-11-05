# Images and continuation dumps

A codemode script that waits for a tool may wait for minutes, or for a human. Keeping a
VM in memory all that time is wasteful, and a process restart or an upgrade of djs would
lose it. The goal of [persistence](goals.md#persistence) is that such a run can be
written out while it waits and continued later, in another process and on a later djs.
djs has two ways to capture a VM. An **image** is the arena's bytes: exact, instant, and
tied to the numbering of the engine that made it, so djs uses images only inside a
process. A **continuation dump** is the run written as a JavaScript program that
recreates it: taken only while the run waits for its tools, and readable by any later
djs. This chapter covers both: what an image is and how it is checked, the one image
every VM starts from, why images stopped being the way runs are persisted, and what a
dump holds and costs.

> **In short.** An image is the arena's bytes. Taking one is a copy; restoring one is a
> check and a copy, with no fixups. An image loads into any build of djs that numbers
> things the way the image does: its roots carry a hash of every table whose numbers it
> holds. Every VM starts as a copy of the **base snapshot**, an image of the initialized
> realm with all builtins compiled, made at build time, which is why creating a VM takes
> microseconds. Images also carry pi's prelude between starts, the stress tests' stops
> and a VM between two wasm modules, all inside one process. To persist a run, djs
> writes a **continuation dump** at an idle point: a Script that rebuilds the run's data,
> its functions stopped at their `await`s and its open tool calls, which every later djs
> loads or refuses with a reason. Compressed, a waiting run's dump is a couple of KB.

## An image is the arena

`Vm::image()` (in wasm `vm_arena_ptr` and `vm_arena_len`, read after a collection) is the
image: the arena from offset 0 to its end. It can be taken whenever `run` has returned: after `Done`, while
`OutOfFuel` in the middle of a loop, while parked on a host call, while idle with ten
tools pending. Nothing is serialized, because nothing needs to be:

- every reference is an offset into the same bytes ([the arena](arena.md));
- the interpreter's registers, the value stack and the frames are cells ([the
  interpreter](interpreter.md));
- pending host calls, queued jobs, the inbox of tool results and the fuel debt are heap
  state ([suspension](suspension.md));
- native code is referred to by index (builtins) or by host-assigned id (host
  functions), and the embedder's handles are slots in a table in the arena (D1091).

Restoring is `Vm::from_bytes`: check the header, verify the cells (in the tools and the
stress tests; see below), copy the bytes into a fresh arena, and continue. The host registers its host functions again under the same
ids, and its handles are still valid, because they were slot numbers all along.

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
before they keep an image (D1890). The stress tests also compare raw images taken at
arbitrary stops, which must be identical too (D1725).

An image holds nothing a run merely left behind (D2308). The interpreter leaves popped
values in the value stack's slots above the stack pointer and overwrites them later;
`Vm::image()` zeroes them, and so does every collection before it traces the stack. A
stack left behind when the interpreter grows it gets its dead slots zeroed too. So two
runs with the same history give the same image bytes whether or not, and wherever, they
were stopped and restored. A collection the embedder asks for also shrinks a stack that
grew for a deep recursion back to what it holds: after 2,000 nested calls a pi
session's stack had 32K slots (256 KB) that every later image of it carried. The base image is made with the
copying collector instead, so its layout depends only on the object graph, not on how the
prelude allocated.

An image also holds the collector's state: the next collection threshold, where the
[old region](gc.md#the-old-region) ends and how many minor collections ran since the last
full one. A restored VM goes on collecting exactly as the original would have. The old
region is part of the image like any other bytes; it is a collector feature, not a
separate layer of the format (D2041).

## What an image depends on

An image contains bytecode, intrinsic ids, object class numbers, slot meanings and cell
layouts. A build of djs that numbers any of those differently would misread it. Most
changes to the engine number nothing differently, though: a faster interpreter loop, a
fixed builtin, a better collector. So an image is keyed on what it depends on, not on the
build that wrote it (D026). The roots cell carries a **dependency descriptor** (D2265),
one 32-bit word per component:

- `VERSION`, the **layout hash**: the cell layouts and the well-known atom names
  (D1052). The host's reader checks this one too, since the layouts are all it reads.
- `FEATURES`, the **feature set**: the cargo features that change images, one bit each:
  whether the build has the full builtin set or is the slim one, and the Unicode tables
  (D2267). Every feature of djs-core is classified as changing images or not; one that is
  in neither list fails the build.
- `COMPAT`, the **compatibility version**: a number bumped by hand for a change of
  meaning that none of the hashes can see, such as what an existing slot of an internal
  record holds, or how a suspended generator's frame is laid out.
- `OPS`, the **opcode table** hash: names, immediates, stack effects and numbering of the
  instructions, in which every function's bytecode is written.
- `INTRINSICS`, the **intrinsic table** hash: the ids the builtins call the engine by.
- `RECORDS`, the **internal id tables** hash: the build script reads the sources that
  number things images store, with comments and whitespace removed, and hashes them
  (D2266). They are the object classes, realm slots, run states, header and template
  flags, value tags and frame continuations, the fast and native builtin tables, and
  every top-level constant of the heap, interpreter, builtin and RegExp modules, which is
  where the slots of internal records are named. Editing a comment keeps the hash;
  renumbering, adding an entry or moving a slot changes it.

Loading compares the layout first, since the other words are read at the offsets it
gives, and then the others in that order. A mismatch is a `LoadError::Incompatible` that
names the first component that differs: "the image is from a djs with another opcode
table (image: 1a2b3c4d; this build: 5e6f7a8b)". The feature set shows by name, so a slim
image in a full build reads as just that. Tests that the gate runs flip one entry of each
table and expect its hash to change, so a hash cannot quietly stop seeing its table.

The engine's **build hash**, of everything that can change behavior (the layout, opcode
and intrinsic tables, djs-core's sources and generated prelude, the enabled features and
the `libm` version: D1320, D1330, D1750), still exists. It identifies the exact build,
which is what a run journal needs to replay step by step, but images are not keyed on
it. It changed in seven of ten merges to djs so far, the descriptor in half.

After the descriptor, `from_bytes` **verifies** the image (D1056): it walks every cell,
checks every traced field (a valid value pointing at the start of a cell of a fitting
kind), the invariants of each kind, and that prototype chains and ropes are acyclic. The
stress tests and the REPL's heap report load images that way, which makes every restore
a check of the heap. The wasm module does not: the only images it loads were made in the same
process (the transport to the dump module, pi's prelude images), so it checks the header
and the descriptor and leaves the verifier out of the shipped code (D2700). A bad image
could still only cause a panic (a trap) or a wrong result, never undefined behavior,
because every access is bounds-checked.

> **Trade-off.** Raw images over a serialization format, inside a process. Images cost no
> code to write or read and restore in a copy, and they capture everything, including
> frames in the middle of a builtin, which is exactly what the base, pi's prelude images
> and the stress tests need. The descriptor lets them survive the many engine changes
> that number nothing differently, but what it guards is the numbering, not the meaning:
> a change that keeps every table but reads a slot differently needs the hand-bumped
> version. For a run that has to outlive the engine build, that is not enough, and
> images are not used for it (below).

## The base snapshot

Creating a realm means building a few thousand objects: the global object, every
constructor and prototype, a thousand builtins compiled from the prelude. Doing that per
VM would cost milliseconds, and codemode creates a VM per script. So it is done once, at
build time. `crates/djs-prelude` has a build script that creates a VM with a host build
of djs-core, runs the prelude that bgen generated ([builtins](builtins.md)), compacts
and collects the heap, verifies the result, and writes it out; the library embeds it as
`djs_prelude::BASE` (D1321). The wasm module carries it in its data section.

`vm_new` copies the base into a fresh arena (`Vm::from_base`, which checks the header
and the dependency descriptor but skips the full walk, since the base was verified when it was
built). That copy **is** startup: about 40 µs in wasm (D1990), most of it the copy of
the base's bytes. The whole base is the new VM's old region, so the collections while a
script runs don't trace it again.

The base ends with a **tail**: the cells every VM writes, put last on purpose (D2063).
They are the realm record, `Math.random`'s state, the global object, the global lexical
scope, `Symbol.for`'s registry, Map and Set tables, the atom table's cell with room for
the atoms the VM adds, the host call table, the interpreter state and the value stack,
the queues for promise jobs, host results and unhandled rejections, and the VM's own
shape tree, already holding the shapes of the objects builtins create (D2067), about
8 KB. Everything before the tail is
the **builtin prefix**, which no VM writes to while builtins are merely used. The
builtins' objects are marked as prototypes when the base is built, not when a cache
first goes through them. Strings in the prefix don't cache their hashes. Each VM grows
its own tree of shapes, so its objects never add transitions to a builtin's shape. No
collection moves or frees the prefix ([the garbage collector](gc.md)), so the same bytes
stay at the same offsets in every VM of the build.

Because it ships in every wasm module and is copied into every VM, the base has been put
on a diet:

- property tables, the atom table and the value stack are shrunk to their used size
  before the final collection (D1431);
- equal strings are stored once: a builtin's `name`, a constant in a function's pool and
  the atom table often held the same string in separate cells, and strings can't be told
  apart by where they live, so the base keeps the atom table's copy and points the
  others at it (D2430);
- the prelude keeps no source text (D1340), no line tables or local names (D1430), and
  its internal helper closures get no `length`, `name` or `prototype` (D1433), and the
  installed builtins keep their `length` and `name` in their templates rather than as
  properties (D2445);
- shapes replaced private property tables for small objects (D1831);
- the functions of the prelude share one constant pool, without the entries only its
  set-up code used (D2446);
- the atoms of names the base no longer needs are dropped and the remaining ones
  renumbered to close the gaps, rewriting every atom id in the heap: property tables,
  shapes, symbols, templates and the atom operands of the code (D2447).

It is about 196 KB today. Building it twice gives the same bytes, and the base built
natively is byte for byte the realm that djs-wasm builds inside wasm32, which a test
checks, so nothing in it depends on the platform.

> **Trade-off.** A prebuilt realm over initializing at startup. Startup becomes a
> `memcpy`, and the builtins' bytecode is never compiled at run time. The cost is size:
> the base is about a quarter of the wasm module, and every builtin, prototype and
> helper adds to it, which is why the prelude is written to be small and the base is
> compacted.

## Images in practice

Images stay the engine's in-process mechanism:

- **The base** is copied into every new VM (above).
- **pi's prelude image.** pi runs its own prelude (helpers, a lockdown that freezes
  about 700 objects) before every script. The adapter seals the heap right after it (a
  full collection that makes everything live old) and keeps the image, keyed by
  everything the prelude saw (D1495, D1890). It restores the image for the next script
  with the same inputs; a start then takes about a millisecond instead of several. The
  prelude runs with the clock at zero in UTC, so the image is a function of its inputs
  alone (D2262).
  - A start that does run the prelude creates its VM with room for the heap size the
    last sealed prelude left (D2043). Growing the arena while the prelude runs would move
    it, and in wasm its old block stays behind as a hole: the hint lowers such a start's
    peak memory from 3.0 to 2.25 MiB.
- **The stress tests and the determinism checks** stop a run every few instructions,
  restore it from its image and compare images byte for byte ([determinism](determinism.md)).
- **The dump module.** A host that persists runs keeps the dumper in a second wasm
  module (below); the VM's image is what travels between the two instances.

## Why images are not the persistence format

For a while, images were also how a waiting run was saved. A collected image of a pi
session waiting on a tool is about 490 KB, nearly all of it the base realm and pi's
prelude, so a saved run was compressed with zstd against an image both sides could
rebuild (the base, or pi's prelude image). Because collections slide cells down in their
allocation order, 99% of such a session's words sat at the same offset as in that
reference, and a waiting session took 2 to 8 KB (D2260).

What sank it was durability, not size. An image restores only into a djs whose layouts,
opcodes, intrinsics and internal ids number everything as the image does, and about half
of the merges to djs changed one of them. A run saved before an upgrade could not
resume after it, unless the host kept the old engine around. A dump says what the run
*means* instead of how one build laid it out, and that survives (D2560). So containers,
session images and reference compression were removed (D2700), and the 2 to 8 KB turned
out not to be the best one could do: a compressed dump of the same waiting session is
smaller still.

## Continuation dumps

A dump is taken at an **idle point** only: `run` returned `Idle`, no frame is on the
stack and no job is queued, and the run waits for its tool calls (D2561). There, every
piece of the run in progress is a function stopped at an `await` or `yield`, which the
compiler can describe by name. A run of this script, stopped while its second read is
open:

```js
const notes = [];
for (const path of ["a.md", "b.md"]) {
  notes.push(await tools.read({ path }));
}
return notes.join("\n");
```

is this dump (`djs run --body --dump-after 2`; the open call's state is the CLI's):

```js
"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 4);
globalThis.$dump = {};
$dump.p1 = $djs.resume(3, "tools.read", [{ path: "b.md" }], {"made":["tools.read"],"answered":{"tools.read":1}});
$dump.n2 = ["# A"];
$dump.i3 = $djs.advance(Array.prototype.values.call(["a.md", "b.md"]), 2);
$dump.Fa4 = $djs.source("notes.js", "3753c3a1172c33f7", "tools,console");
$dump.a4 = $djs.k(async function (tools, notes) {
const $djs_I = $djs.I;
const $djs_i1 = $dump.i3;
notes.push(await $dump.p1);
for (const path of $djs_I($djs_i1)) {
  notes.push(await tools.read({ path }));
}
return notes.join("\n");
}, $dump.Fa4, 1, 54).call(globalThis, $djs.host("tools"), $dump.n2);
try { ($dump.a4); } finally { delete globalThis.$dump; delete globalThis.$djs; }
```

It is an ordinary Script (D2830): djs, or node, runs it given the host's functions and
one object the host provides while it loads, `$djs` (D2834, D2890):

- data is written as literals and the calls that make it; a builtin is the expression
  that finds it, never its contents;
- `$djs.seed(...)` restores `Math.random`'s sequence and says which id the run's next
  host call gets (D2897);
- `$djs.resume(id, name, args, state)` stands for the open tool call: the host continues
  call `id` of its function `name` under the same id and gives back its promise; the
  last argument is the host's own state of the call, JSON it gave when it dumped (D2770);
- `$djs.host("tools")` is the host's `tools` object, named by the name the host gave it
  rather than by where the script found it (here a parameter); the host returns the very
  object it bound under that name (D2970, D2971);
- `$dump.a4` is what the suspended function's **continuation** returned: a function of
  the same kind whose body is the rest of the original from the `await` on, written where
  it is called. Its parameters are the values the frame held there that the rest still
  reads (`tools`, `notes`; the dead `path` and `console` are left out), passed as the
  call's arguments. The rest of the current loop iteration comes first, `notes.push(await
  …)` reading `notes.push` again, which gives what the run had looked up, since builtins
  cannot change (D2836). Then the loop goes on as written over the saved iterator;
- the loop's iterator is made again and advanced with the same calls that put it there:
  in-flight builtin state is re-enacted, not described by the engine's internal layout
  (D2562). The small helpers that takes, here `$djs.advance`, the loop wrapper `$djs.I`
  and `$djs.k`, which gives the continuation its origin record, are the same in every
  dump, so the host provides them as a fixed runtime instead of each dump carrying them
  (D2903); djs makes that object for its hosts, and a host without djs imports it from
  [js/src/dump-runtime.js](../../js/src/dump-runtime.js);
- the continuation's origin record, the arguments of `$djs.k` after it, names its origin,
  the script's function (`$dump.Fa4`, which the host makes again from the script it ran,
  named by its file and hash, and never calls), the await it stands for (`1`, the
  first) and the few decisions the writer took on the run's values that shaped the
  continuation's text (`54`, as bits: whether it reads `notes.push` or the loop's `next`
  again instead of keeping what the run looked up, …). A dump of a
  run that resumed this one writes the continuation again from the original and those
  decisions, finds out that way which stretches of the continuation's text are copies of
  the original's, reads the continuation's frame as one of the original and writes the
  original again (**origin tracking**, D2833, D2913), so a dump stays the same size
  however many times a run is resumed and dumped;
- the Script's completion value is what the host asked for, the run's result promise
  (D2895), and its last statement deletes `$dump` and `$djs`.

To read a frame, the exporter uses what the compiler records at each `await` (its
**suspension map**: the bindings and values a frame holds there by name, and the syntax
around it, D2563, D2850). The dump itself names no slot of a frame, so it depends on no
build's compiler: a later djs runs it as it runs any script. Golden dumps written once are
run by every gate (D2920). Dumps are written only for VMs with frozen builtins, the mode
pi's sessions run in: the writer may then assume that no builtin changes between the
moment a value was read and the load, and write a builtin the run already looked up as
the expression that finds it again (D2836). A sandbox without them cannot dump. What the
writer cannot write, such as an object carrying a class's private field (only the class's
own construction can add one, D2878), is refused with its path.

What happens to the open calls is the host's decision: deliver a result it recorded,
issue the call again, or reject it. Timers and requests in flight are host state, and the
host keeps them beside the dump.

The first format of continuation dumps (format 2) was not plain JavaScript: a loader in
the engine ran it with privileged primitives that built frames slot by slot, and it
compiled the dump's own text to find where each slot went. Plain dumps replaced it, and
the loader, its primitives and the engine's support for them were deleted (D2830, D2920).

**What a dump costs.** Over 300 real codemode scripts and their 1,063 idle points, a
dump is 6 KB of text at the median and 17 KB compressed at p99; with zstd at level 3,
which hosts apply when they store it, the median is 1.7 KB, a third of what the image
containers took (D2703). A session holding a 1.8 MB log dumps to 1.8 MB of text and
394 KB compressed. Writing a dump takes about 0.4 ms at the median in wasm (3 ms at
p99) and resuming one, a new sandbox included, under a millisecond; a session holding
1,500 records dumps in about 11 ms, one holding a 3.7 MB log in about 30 ms.

> **Trade-off.** Dumps over images for persistence. A dump survives every change to the
> engine that keeps the meaning of the script's code, it is readable, and it is smaller
> once compressed. The costs: it exists only at idle points, so a run that is computing
> cannot be persisted (it reaches an idle point or reruns, which determinism makes safe);
> the exporter has to know how to re-enact every kind of in-flight builtin state, and
> refuses what it cannot write (a weak collection, a revoked Proxy) with its path; and a
> dump is code, so a host loads only dumps it wrote.

> **Trade-off.** The dumper in a second module (D2704). Writing dumps is about 300 KB
> of code, a quarter of the module, that only hosts which persist runs need. So the
> shipped module leaves it out, and such a host loads the dump module next to it and
> copies the VM's image into it to dump. Loading needs no dumper: a dump is a Script. Keeping that
> instance between dumps makes both packagings equally fast; the cost is a second module
> to ship and load, and a copy of the image per dump.

## Inspecting a heap

What is in a VM's heap, and why is it as large as it is? The REPL's `.heap` answers both
(D2264). It reads the live VM's image through the engine's own heap code, never its own
parser: the image is verified as a restore would verify it, the walker and the layout
table say which fields are references and what they are called, and liveness follows the
collector's rules. From that it reports:

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

An almost empty session is still about 200 KB of image, nearly all of it the base. So
the largest cells, strings, shapes and functions describe only the session's own part:
the cells that are new or changed against the base (D2432). The base's part is one line
of totals.

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
> offset would be simpler, but it would call every cell after the first one that moved
> a change, which is exactly the case a size question is about. Matching through the
> object graph names what moved and what it cost; it can pair the wrong cells where two
> siblings are equally plausible, which shows up as a changed cell where there was a
> new one.

## Where to look

- [`crates/djs-core/src/vm/mod.rs`](../../crates/djs-core/src/vm/mod.rs): `bytes`,
  `from_bytes`, `from_base`, the build hash;
  [`vm/compat.rs`](../../crates/djs-core/src/vm/compat.rs): the dependency descriptor and
  the compatibility version; [`compat_inputs.rs`](../../crates/djs-core/compat_inputs.rs):
  the source regions of the internal id tables hash and the feature classification.
- [`crates/djs-core/src/heap/walk.rs`](../../crates/djs-core/src/heap/walk.rs): the
  verifier; [`heap/compact.rs`](../../crates/djs-core/src/heap/compact.rs): compaction
  for the base image.
- [`crates/djs-prelude/build.rs`](../../crates/djs-prelude/build.rs): how the base
  snapshot is made.
- [`crates/djs-core/src/vm/dump.rs`](../../crates/djs-core/src/vm/dump.rs): the
  exporter; [`vm/dump_plain.rs`](../../crates/djs-core/src/vm/dump_plain.rs) and
  [`vm/dump_cont.rs`](../../crates/djs-core/src/vm/dump_cont.rs): the plain Script and
  the continuations; [`vm/dump_runtime.rs`](../../crates/djs-core/src/vm/dump_runtime.rs):
  the dump runtime; [`tests/dump/v2/`](../../tests/dump/v2/): golden dumps.
- [`js/src/sandbox.ts`](../../js/src/sandbox.ts): tasks (`start`, `suspend`,
  `Sandbox.resume`), `begin`, `Run.dump` and `loadRun`, and the dump module; [`js/pi/vm.ts`](../../js/pi/vm.ts): the
  adapter's prelude images and its dumps of pi sessions.
- [`crates/djs-cli/src/heap/`](../../crates/djs-cli/src/heap/): the heap report behind the
  REPL's `.heap`.

Decisions on this page:

<!-- gen:decisions D2040 D2041 D2063 D2067 D2043 D1091 D1890 D1725 D1052 D026 D2265 D2266 D2267 D2268 D1320 D1330 D1750 D1056 D1321 D1431 D1340 D1430 D1433 D1831 D1990 D1495 D2260 D2262 D2264 D2308 D2445 D2446 D2447 D2560 D2561 D2562 D2563 D2700 D2703 D2704 D2432 D2340 D2269 D2770 D2830 D2833 D2834 D2836 D2850 D2878 D2890 D2895 D2897 D2903 D2920 -->
- D2040: In-place mark-compact for in-VM collections; Cheney for canonical layouts
- D2041: An old region that minor collections neither trace nor move
- D2063: The base image ends with a tail of per-VM cells; nothing writes below BASE_END
- D2067: Builtin result shapes and the engine's queues are in the base; the ceiling rises 720 B
- D2043: pi's later cold starts size the arena for the sealed prelude heap
- D1091: Handle table
- D1890: pi adapter: snapshot() and the prelude image collect first
- D1725: The determinism checks compare raw images too
- D1052: Layout description macro
- D026: Image compatibility keyed on what an image depends on
- D2265: Images carry a dependency descriptor; loading and containers check it, not the build hash
- D2266: RECORDS_HASH: the id tables images store numbers of, read from the sources
- D2267: Cargo features are classified as image features or image-neutral
- D2268: When to bump COMPAT_VERSION
- D1320: VM images carry a build hash (Roots::BUILD)
- D1330: The build hash includes a hash of djs-core's sources
- D1750: The build hash covers features and libm; canonical f32/f16 NaN
- D1056: Heap images are verified on load
- D1321: The base snapshot (djs-prelude)
- D1431: The base image is compacted before it is collected
- D1340: The prelude keeps no source text
- D1430: Privileged code keeps no line tables or local names
- D1433: Prelude closures carry no `length`, `name` or `prototype`
- D1831: Shapes with strong transitions and a per-heap shape budget
- D1990: A restored arena reserves its GC threshold plus a slack
- D1495: pi adapter: reuse the VM image taken after the prelude
- D2260: Snapshots are compact containers: collected, zstd against a reference, pinned to the build
- D2262: pi's prelude image is the reference of the adapter's snapshots and is recomputable
- D2264: `djs heap` analyzes dumps through djs-core's heap code; djs-cli reads and writes snapshot containers
- D2308: Images hold no dead value-stack slots; embedder collections shrink a grown stack
- D2445: A closure's length and name come from its template until changed
- D2446: One constant pool per compilation unit
- D2447: The base's atoms are renumbered without gaps
- D2560: A waiting session persists as a continuation dump; heap images stay inside a process
- D2561: Dumps are taken only at idle points
- D2562: In-flight builtin state is re-enacted; a few privileged loader primitives
- D2563: The compiler's suspension maps are a pinned contract
- D2700: Heap images are no longer a persistence format; what was removed
- D2703: Hosts store dump text compressed with zstd; the format stays a plain Script
- D2704: Packaging B: the shipped module has no dumper, hosts that persist load the dump module
- D2432: `djs heap` shows the session's own part by default
- D2340: Heap views show the program's own cells apart from the base realm; REPL dumps are containers
- D2269: No `djs heap` command and no `.image`; the REPL's `.heap` keeps the heap report
- D2770: A call's state is JSON text, written verbatim as `$dump.pending`'s fourth argument
- D2830: Continuation dumps become plain JavaScript
- D2833: Re-dumps use origin tracking
- D2834: Plain dumps are mostly self-contained: the code to continue is in the dump, the host provides `$djs`
- D2836: Plain dumps are written only for VMs with frozen builtins
- D2850: Suspension maps carry the syntax around each site
- D2878: Plain dumps refuse instances that carry private names
- D2890: Plain dumps load as a Script with `$djs.resume`/`$djs.seed`; open calls keep their ids
- D2895: Several handles in a plain dump: a null-prototype object as the completion value
- D2897: `$djs.seed(lo, hi, next)`: call ids go on after a plain load as in the uninterrupted run
- D2903: The plain dump runtime: the format's helpers are members of `$djs`
- D2920: Format 2 is retired; plain dumps are the only format, and the goldens are plain dumps
<!-- /gen -->
