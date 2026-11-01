# Snapshots and images

A codemode script that waits for a tool may wait for minutes, or for a human. Keeping a
VM in memory all that time is wasteful, and a process restart would lose it. The goal of
[snapshots](goals.md#snapshots) is that the whole VM can be saved at any point and
continued later, in another process. After the previous chapters, there is not much left
to explain: the state is one byte array without pointers. This chapter covers what an
image is, how one is loaded safely, the one image every VM starts from, and what images
cost.

> **In short.** An image is the arena's bytes. Taking one is a copy; restoring one is a
> check and a copy, with no fixups. An image loads only into the exact build that wrote
> it, and is verified cell by cell before it is used. Every VM starts as a copy of the
> **base snapshot**, an image of the initialized realm with all builtins compiled, made
> at build time, which is why creating a VM takes microseconds.

## An image is the arena

`Vm::bytes()` (`vm_arena_ptr` and `vm_arena_len` in wasm) is the image: the arena from
offset 0 to its end. It can be taken whenever `run` has returned: after `Done`, while
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

Taking an image right after a collection makes it small and canonical: no garbage, and a
layout that depends only on the object graph ([the garbage collector](gc.md)). The
embedders do that when they keep an image (D1890); the stress tests also compare raw
images taken at arbitrary stops, which must be identical too (D1725).

## Loading only what this build wrote

An image contains bytecode, intrinsic ids, object class numbers, slot meanings and cell
layouts. Another build of djs may read any of those differently, so an image is not a
file format: it loads only into the build that wrote it. The `Roots` cell carries two
hashes:

- `VERSION`, the **layout hash**: the cell layouts and the well-known atom names
  (D1052). The host's reader checks this one, since the layouts are all it reads.
- `BUILD`, the **build hash**: the layout hash, the opcode table and the intrinsic table
  (D1320), a hash of djs-core's sources and the generated prelude (D1330), and the
  enabled features and the `libm` version (D1750). Any change that can change behavior
  changes it.

A mismatch is a `LoadError::BuildMismatch`, not an attempt. After the hashes,
`from_bytes` **verifies** the image (D1056): it walks every cell, checks every traced
field (a valid value pointing at the start of a cell of a fitting kind), the invariants
of each kind, and that prototype chains and ropes are acyclic. A corrupt image is an
error at load time. One that slipped past could still only cause a panic (a trap) or a
wrong result, never undefined behavior, because every access is bounds-checked.

> **Trade-off.** Raw images over a versioned serialization format. Images cost no code
> to write or read and restore in a copy, and they capture everything, including frames
> in the middle of a builtin. The cost: they are tied to one build. A VM saved before a
> deploy cannot resume on the new engine. The durable record across versions is a
> different mechanism: a journal of the run's external inputs, replayed on the new
> engine and checked against canonical hashes (D1730, see [the host
> interface](host.md)).

## The base snapshot

Creating a realm means building a few thousand objects: the global object, every
constructor and prototype, a thousand builtins compiled from the prelude. Doing that per
VM would cost milliseconds, and codemode creates a VM per script. So it is done once, at
build time. `crates/djs-prelude` has a build script that creates a VM with a host build
of djs-core, runs the prelude that bgen generated ([builtins](builtins.md)), compacts
and collects the heap, verifies the result, and writes it out; the library embeds it as
`djs_prelude::BASE` (D1321). The wasm module carries it in its data section.

`vm_new` copies the base into a fresh arena (`Vm::from_base`, which checks the header
and the build hash but skips the full walk, since the base was verified when it was
built). That copy **is** startup: about 40 µs in wasm (D1990), most of it the copy of
the base's bytes.

Because it ships in every wasm module and is copied into every VM, the base has been put
on a diet:

- property tables, the atom table and the value stack are shrunk to their used size
  before the final collection (D1431);
- the prelude keeps no source text (D1340), no line tables or local names (D1430), and
  its internal helper closures get no `length`, `name` or `prototype` (D1433);
- shapes replaced private property tables for small objects (D1831).

It is about 398 KB today. Building it twice gives the same bytes, and the base built
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
  700 objects) before every script. The adapter keeps the image taken right after it,
  collected, keyed by everything the prelude saw, and restores it for the next script
  with the same inputs. A start then takes about a millisecond instead of several.
- **Waiting sessions.** `snapshot()` collects the session and takes its image while it
  waits for a tool; `resumeVm()` continues it, possibly in another process.

How large are such images, and how should they be stored? A study measured six realistic
pi scripts at all 144 of their idle points. A collected image of a light session is
about 500 KB, nearly all of it the base realm and pi's prelude, which are a
deterministic function of the engine build and the prelude's inputs. Compressed with
that reference image as a dictionary, the same snapshot is 22 to 30 KB. The
recommendation: keep raw images for same-build use, store them compressed against the
reference, and rely on journal replay for durability across engine versions. The
original plan of page-level deltas against a non-moving old generation was measured too
and lost to the dictionary by a factor of four; it is superseded. The owner's decision
on the rest is pending, so none of it is built yet.

> **Trade-off.** Whole images over deltas. A delta against the base or a previous
> snapshot would be smaller, but a moving collector changes the offsets of everything it
> copies, so byte-level deltas of a copying heap are fragile, and a non-moving old
> generation would need a second collector mode. Compressing a whole image against a
> recomputable reference gets most of the size back with no engine change.

## Where to look

- [`crates/djs-core/src/vm/mod.rs`](../../crates/djs-core/src/vm/mod.rs): `bytes`,
  `from_bytes`, `from_base`, the build hash.
- [`crates/djs-core/src/heap/walk.rs`](../../crates/djs-core/src/heap/walk.rs): the
  verifier; [`heap/compact.rs`](../../crates/djs-core/src/heap/compact.rs): compaction
  for the base image.
- [`crates/djs-prelude/build.rs`](../../crates/djs-prelude/build.rs): how the base
  snapshot is made.

Decisions on this page:

<!-- gen:decisions D1091 D1890 D1725 D1052 D1320 D1330 D1750 D1056 D1730 D1321 D1431 D1340 D1430 D1433 D1831 D1990 D1495 -->
- D1091: Handle table
- D1890: pi adapter: snapshot() and the prelude image collect first
- D1725: The determinism checks compare raw images too
- D1052: Layout description macro
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
<!-- /gen -->
