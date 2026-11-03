# Snapshots and images

A codemode script that waits for a tool may wait for minutes, or for a human. Keeping a
VM in memory all that time is wasteful, and a process restart would lose it. The goal of
[snapshots](goals.md#snapshots) is that the whole VM can be saved at any point and
continued later, in another process. After the previous chapters, there is not much left
to explain: the state is one byte array without pointers. This chapter covers what an
image is, how one is loaded safely, the one image every VM starts from, and what images
cost.

> **In short.** An image is the arena's bytes. Taking one is a copy; restoring one is a
> check and a copy, with no fixups. An image loads into any build of djs that numbers
> things the way the image does: its roots carry a hash of every table whose numbers it
> holds, and loading compares them before it verifies the image cell by cell. Every VM starts as a copy of the
> **base snapshot**, an image of the initialized realm with all builtins compiled, made
> at build time, which is why creating a VM takes microseconds. A stored snapshot is the
> collected image compressed against an image both sides can rebuild, such as the base
> snapshot: a session waiting on a tool takes a few KB instead of half a megabyte.

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

Restoring is `Vm::from_bytes`: check the header, verify the cells, copy the bytes into a
fresh arena, and continue. The host registers its host functions again under the same
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
session's stack had 32K slots (256 KB) that every later session image carried. The base image is made with the
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
kind), the invariants of each kind, and that prototype chains and ropes are acyclic. A
corrupt image is an error at load time. One that slipped past could still only cause a
panic (a trap) or a wrong result, never undefined behavior, because every access is
bounds-checked.

> **Trade-off.** Raw images over a versioned serialization format, and hashes of the
> tables they use over the build's identity. Images cost no code to write or read and
> restore in a copy, and they capture everything, including frames in the middle of a
> builtin. The descriptor lets them survive the many engine changes that number nothing
> differently. What it guards is the numbering, not the meaning: a change that keeps
> every table but reads a slot differently needs the hand-bumped version, and forgetting
> it lets an old image restore into an engine that misreads it (the verifier catches
> broken structure, not changed meanings). Reading the sources is conservative the other
> way: a renamed constant refuses images that would have worked. A VM saved before a
> deploy that does change a dependency cannot resume on the new engine, unless the host
> keeps the old engine around to finish it. What survives any upgrade instead is a
> journal of the run's external inputs, which a new engine replays (D1730, see [the host
> interface](host.md)); djs records such journals, but has not made them its durable
> format. For the bindings of an idle session there is also a dump: the globals written
> as a JavaScript program (functions as their source text, closures by small factories
> that keep shared variables shared, data as literals, builtins by name) that any later
> build runs to recreate them, values only, never frames (D2353).

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

pi's adapter uses images in two places (D1495, D1890):

- **The prelude image.** pi runs its own prelude (helpers, a lockdown that freezes about
  700 objects) before every script. The adapter seals the heap right after it (a full
  collection that makes everything live old) and keeps the image, keyed by everything
  the prelude saw. It restores the image for the next script with the same inputs; a
  start then takes about a millisecond instead of several.
  - A start that does run the prelude creates its VM with room for the heap size the
    last sealed prelude left (D2043). Growing the arena while the prelude runs would move
    it, and in wasm its old block stays behind as a hole: the hint lowers such a start's
    peak memory from 3.0 to 2.25 MiB.
- **Waiting sessions.** `snapshot()` collects the session and takes its image while it
  waits for a tool; `resumeVm()` continues it, possibly in another process. The `Sandbox`
  does the same with `bytes()` and `fromBytes()`.

## Compact snapshots

A collected image of a session waiting on a tool is about 490 KB. Nearly all of it is
the base realm and pi's prelude, which neither the script nor its tools touched. Storing
it as it is wastes space; general-purpose compression gets it to about 135 KB, still
mostly the realm.

The realm does not have to be stored at all, because both sides can rebuild it. The base
snapshot is fixed by the engine build, and ships inside the module. pi's prelude image is
fixed by the build and the prelude's inputs: the prelude source, the tool list, the
globals, the store, the memory limit. So a snapshot is compressed with zstd using such a
**reference image** as its dictionary, and stores only a description of the reference
(D2260).

That works far better than one might expect, because of how the heap is collected. A
collection slides the live cells down in their allocation order ([the garbage
collector](gc.md)), so the cells a session inherited from its reference stay exactly where
they were: in a session waiting on a tool, 99% of the image's words sit at the same
offset as in the prelude image. zstd encodes all of that as one long match, and what is
left is what the script did:

| a snapshot of | image | compressed against the reference |
| --- | ---: | ---: |
| a pi session waiting on a tool | 478–500 KB | 2–8 KB |
| a pi session holding 60 files it read | 660–842 KB | 57–110 KB |
| a pi session holding a 1.5 MB log | 2.2 MB | 404 KB |
| a new sandbox | 32 KB (322 KB raw) | 1.0 KB |

Against the base image instead of pi's prelude image, the waiting session is about 51
KB, and with the copying collector the heap used before, it was 40 KB: one word in eight
stayed in place. Compressing a waiting session takes about 1.2 ms, and restoring costs
0.1 to 0.3 ms more than an uncompressed image.

One more thing keeps the reference's cells in place: where its growable cells sit (D2057).
A session's first steps replace a few cells of the reference outright. Defining a
sandbox's `console` grows the global object's property table, which is exactly full in
the base, and the first calls through pi grow the prelude's Map of pending calls. The grown copies go to the young end, and the next full collection drops the old
ones. If those sat in the middle of the reference, everything after them would slide
down. A new sandbox's snapshot was 16 KB that way: only a fifth of the image lined up
with the base, and the moved cells cost the compressor the references they hold to
other moved cells. So the base image puts those cells last, and so does `Vm::seal` for an
image an embedder seals itself, like pi's prelude image. A dead copy then moves nothing
before it.

The atom table is split for the same reason (D2431). The base's atoms, about 490
entries, and their hash index sit in the builtin prefix and are never written. The atoms a VM adds go to a second, small
list and index in the tail, with room for 32 before they grow, and interning looks in
the VM's own index first. So a session's first atoms change only the words they add,
and the session image no longer carries the base's entries (15 KB at the time): a fresh REPL's
session image went from 34.6 to 11.5 KB.

What a snapshot stores is a **session image** (D2066): the image without the builtin
prefix, since the module (or the reference) already has it. It holds the `Roots` cell,
the prefix words that differ from the base, as offset and value pairs in ascending order,
and the heap from the prefix's end on. A script that leaves the builtins alone has no
such words. The format is a function of the image and the base alone, so equal images
give equal session images. Restoring puts the image back together from the base, after
checking a hash of the base's prefix. Compressed against the reference, this changes
little, because the dictionary already made the prefix almost free. But an uncompressed
snapshot of a new sandbox is 32 KB instead of 322 KB. pi's lockdown changes about 1,700
words of the prefix, which pi's prelude image holds in place. So the prefix words are
taken against the reference when it has the same prefix, and against the base only
otherwise; as a diff against the base, those words cost 8 KB in every container.

A snapshot is a small container: a 68-byte header, the host's own metadata (the
`Sandbox`'s host function names, the adapter's session state) as JSON, and the session
image, compressed or not. The header names everything a restore depends on: the
container's version, the image's dependency descriptor and the hash of the base image's
builtin prefix (the session image is a diff against it), which reference (the base, or
pi's prelude image) with its hash and length, the compression, and what the prefix words
are against. A restore compares them with the module's own base image, in that order,
before it touches the image, and stops with a message that says what is wrong: another
container version, a snapshot that needs another component ("the snapshot is from a djs
with another opcode table (snapshot: X; this module: Y)") or another base image, a
reference this host does not have or that differs, a damaged frame (zstd's checksum). So
a snapshot restores into a newer module when the merges in between left its descriptor
and its base image alone: over djs's history so far, close to half of the merges did,
where under a third left the build hash alone. There is one container format, and nothing reads older ones: djs has not been
released, so its formats change in place (D024, D2400). A host that has to restore
snapshots whose dependencies changed has to keep the build that wrote them.

For pi's prelude image to be a reference, it has to come out the same in every process
that computes it. Only one input was not under control: the clock, which the adapter sets
before every slice of work. So the prelude runs with the clock at zero, in UTC; nothing
in it reads the time, and each run sets its own clock before the script starts (D2262).
Two processes then build the same image byte for byte. `resumeVm` finds the reference
among the prelude images of its process, or recomputes it from the session's prelude and
start data, refusing data that make a different image.

> **Trade-off.** Compressing in the host, not in the engine. node has zstd built in, so
> the adapter and the `Sandbox` use it, at level 3 (D2261). Higher levels save little on
> a waiting session and a fifth on one holding megabytes, at up to a hundred times the
> time, which is too long while a script waits. A pure-Rust zstd compiled into the module
> was measured too: the same sizes, as fast on small snapshots and up to twice as slow on
> large ones, and 57 KB more of gzipped module. The cost of keeping it out: a browser has
> no zstd that takes a dictionary, so there snapshots are written uncompressed unless the
> page brings its own codec.

> **Trade-off.** Whole compressed images over deltas, past the builtin prefix. A delta
> format would have to track which cells changed since the reference and encode them on
> its own. Compressing the whole image against the reference gets the same result
> without any of that, because the collector keeps the reference's cells in place. Only
> the prefix, which never moves, is stored as a word diff. The cost: every snapshot reads and
> compresses the whole image (1 to 8 ms), and the size depends on that layout property.
> A change that grows a cell near the start of the reference moves everything after it,
> and the snapshot grows from a few KB to tens of KB.

## Inspecting a heap

What is in an image, and why is a snapshot the size it is? `djs heap` answers both
(D2264). It reads a raw image (what the REPL's `.snapshot --raw` writes), a session image
or a snapshot container, decompressing a container with its reference: this build's base
when the hashes match, or a reference given as a file, which
[`scripts/heap/reference.ts`](../../scripts/heap/reference.ts) writes for a module's base
or for pi's prelude image of a session's start data.

It reads the image through the engine's own heap code, never its own parser: the image is
verified as a restore would verify it, the walker and the layout table say which fields
are references and what they are called, and liveness follows the collector's rules.
From that it reports:

- a summary: live and garbage bytes, the old region and the young one, and the size of
  the session image a snapshot would store before compression;
- cells and bytes per cell kind and per object class;
- a diff against the reference image;
- the builtins a script changed: the words of the builtin prefix that differ from this
  build's base, by cell, named by their builtin paths (`%Array.prototype%`, `%Math%`);
- the largest cells, and the largest *retained* sizes: what a cell keeps alive on its
  own, from the dominator tree of the object graph;
- strings (duplicated contents, the largest), shapes (the most common key lists),
  templates and bytecode per script, closures;
- what a waiting VM holds: queued jobs, pending host calls, promises, and the suspended
  async functions and generators with their locals by name.

An almost empty session is still more than 300 KB of image, nearly all of it the base. So
when there is a reference to compare with, the largest cells, strings, shapes and
functions describe only the session's own part: the cells that are new or changed
against the reference (D2432). The reference's part is one line of totals, and `--all`
lists the whole image. Without that, a fresh REPL looked as if it held 900 strings and
874 functions, all of them the base's.

Every cell gets a path from the roots that reads like JavaScript where the heap allows,
built from the shapes' keys, the elements' indexes, the templates' recorded names of
captured variables and locals, and the layout's field names for everything internal:
`globalThis.cache[3].body`, `handle[1]~pending`, `<atoms><entries>`.

The diff is what explains a snapshot's size. Cells have no identity but their offset,
and a collection moves them, so the tool matches the two images structurally: a
breadth-first walk from both roots pairs the targets of the same field. Immutable cells
(strings, bytecode) pair only with the same content, and a shape's list of transitions
pairs its children by the key they add, since one inserted transition shifts the rest.
Then every cell of the image is the reference's own, in place or moved, a changed one,
or new, and the new ones are grouped into the subtrees that keep them alive. A changed
cell counts the words that differ; one that grew into a bigger cell (a regrown table)
compares over the words both have, and its new words count where they are not zero. For a pi
session waiting on its second tool call:

```text
== diff against pi's prelude image
  against pi's prelude image (484.3 KB): 98.9% of the image's 8-byte words are the
  reference's at the same offset; cells line up for the first 484.2 KB (99.0%)
  cells                             count     KB  differing KB
  unchanged, in place                8415  443.9
  unchanged, moved                      0    0.0           0.0
  changed                              21   40.3           0.5
  new                                  78    4.7           4.7
  gone (the reference's, not here)      1    0.1

  largest new subtrees (new cells and the new cells only they keep alive):
  KB   cells             kind  path
  3.2     47  Object/Ordinary  handle[1]~pending<entries>[5]
```

The 4.7 KB of new cells are nearly all the script itself, kept alive through the promise
of the call it waits on, and the container is 2.1 KB.

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
- [`crates/djs-core/src/vm/session.rs`](../../crates/djs-core/src/vm/session.rs): session
  images.
- [`js/src/snapshot.ts`](../../js/src/snapshot.ts): the snapshot container, its checks and
  the zstd codec; [`js/pi/vm.ts`](../../js/pi/vm.ts): the adapter's prelude images,
  `snapshot()` and `resumeVm()`.
- [`crates/djs-cli/src/heap/`](../../crates/djs-cli/src/heap/): `djs heap`, the analyzer;
  [`crates/djs-cli/src/container.rs`](../../crates/djs-cli/src/container.rs): the
  container in Rust.

Decisions on this page:

<!-- gen:decisions D2040 D2041 D2057 D2063 D2066 D2067 D024 D2400 D2058 D2043 D1091 D1890 D1725 D1052 D026 D2265 D2266 D2267 D2268 D1320 D1330 D1750 D1056 D1730 D1321 D1431 D1340 D1430 D1433 D1831 D1990 D1495 D2260 D2261 D2262 D2264 D2308 D2445 D2446 D2447 -->
- D2040: In-place mark-compact for in-VM collections; Cheney for canonical layouts
- D2041: An old region that minor collections neither trace nor move
- D2057: The cells a VM replaces first sit at the end of base and sealed images
- D2063: The base image ends with a tail of per-VM cells; nothing writes below BASE_END
- D2066: Session images: the roots cell, a canonical prefix diff and the session region
- D2067: Builtin result shapes and the engine's queues are in the base; the ceiling rises 720 B
- D024: No compatibility with unreleased formats
- D2400: One snapshot container format, session images only
- D2058: The size ceiling rises by 600 B for stable snapshot layouts
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
- D1730: Canonical hash and run journals
- D1321: The base snapshot (djs-prelude)
- D1431: The base image is compacted before it is collected
- D1340: The prelude keeps no source text
- D1430: Privileged code keeps no line tables or local names
- D1433: Prelude closures carry no `length`, `name` or `prototype`
- D1831: Shapes with strong transitions and a per-heap shape budget
- D1990: A restored arena reserves its GC threshold plus a slack
- D1495: pi adapter: reuse the VM image taken after the prelude
- D2260: Snapshots are compact containers: collected, zstd against a reference, pinned to the build
- D2261: Snapshot compression runs in the host: node's zstd at level 3; none without it
- D2262: pi's prelude image is the reference of the adapter's snapshots and is recomputable
- D2264: `djs heap` analyzes dumps through djs-core's heap code; djs-cli reads and writes snapshot containers
- D2308: Images hold no dead value-stack slots; embedder collections shrink a grown stack
- D2445: A closure's length and name come from its template until changed
- D2446: One constant pool per compilation unit
- D2447: The base's atoms are renumbered without gaps
<!-- /gen -->
