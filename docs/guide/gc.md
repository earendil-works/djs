# The garbage collector

Everything a djs VM keeps is in the arena, and the arena only grows by bumping
([one arena](arena.md)). Something has to give the memory of dead objects back, and in
djs that same thing decides what an image contains and whether two runs of a program
leave identical heaps. This chapter explains what the collector traces, how it moves
cells, when it runs, and what happens when memory runs out.

> **In short.** A VM's collections are mark-compact **in place**: they mark what is
> reachable from one roots cell and slide the survivors down over the garbage in their
> allocation order, so no second arena is needed. Most of them are **minor**: the heap's
> prefix that holds the realm and whatever the host sealed, the **old region**, is taken
> as live and stays where it is, and only what the program allocated since is traced and
> moved; a separate **copying** collector lays a heap out breadth-first for canonical
> hashes and the base image. Collections run only at safepoints, when allocation has
> crossed a threshold that is itself stored in the heap, so where they happen depends on
> the program and never on how it was sliced.

## Roots, handles and what is traced

All roots are in one place: the `Roots` cell at offset 0. Its fields point at the atom
table, the realm, the value stack, the interpreter's registers, the job queue, the host
call table and the **handle table**. A handle is how Rust code or an embedder holds a
value across a collection: an index into that table, whose slots the collector traces
and rewrites when cells move. A raw value held outside the heap is an arena offset and
is stale after a collection, which is why collections happen only where no Rust code
holds one ([safepoints](suspension.md#safepoints)).

What counts as a traced field comes from the layout table alone: a field declared as
`Value` or `Ref` in the cell declarations is traced, nothing else is ([cells, strings
and objects](heap.md)). There is no per-kind tracing code to forget when a cell kind is
added, and both collectors use the same description. The frames of running functions
are plain values on the value stack, so the collector traces the stack without knowing
about frames. A few kinds get extra treatment:

- a reference to a **flattened rope** is redirected to its flat string, so the rope
  dies;
- **Map and Set** tables, whose object keys hash by offset, are rehashed in place after
  the cells have moved;
- **ephemeron tables**, the storage of the weak collections, are not traced like other
  cells ([below](#weak-references-and-finalization));
- inline cache slots that hold arena offsets are emptied.

## Sliding in place

A full collection runs three passes over the arena:

1. **Mark.** Starting at the `Roots` cell, every reachable cell gets a mark bit in its
   header, and its words are set in a **live bitmap**, one bit per 8 bytes. The marker
   keeps a stack of cells still to scan, which grows on demand; if it cannot grow, a
   cell stays marked but unscanned, and a walk over the marked cells finds it later.
   Only the *set* of marked cells reaches the result, so the order of marking cannot
   change it.
2. **Count.** A table with one entry per 512 bytes of arena records how many live bytes
   lie before that block. The new address of a live cell is that entry plus the live
   words before it in its block, which one population count of a bitmap word gives.
   Nothing is written into the cells.
3. **Update and slide.** One walk over the marked cells in address order rewrites every
   traced field with its target's new address and moves each run of adjacent live cells
   down. The new addresses come from the bitmap and the table alone, so each cell is
   visited once, and nothing below the first dead word moves, so fields pointing there
   stay as they are. Map and Set tables are rehashed last.

The bitmap, the table and the stack take about 2.3% of the arena. They are allocated
before anything is touched, so a host that cannot provide them gets an error with the
heap unchanged.

Because cells keep their order, the heap after a collection is the live cells in the
order they were allocated. Collecting along the way and collecting once at the end leave
the same bytes for the same allocations, and collecting a collected heap changes
nothing. That is what the [determinism checks](determinism.md#the-same-program-the-same-heap)
build on, together with collection points that don't depend on slicing.

> **Trade-off.** In-place over copying for a VM's own collections. A copying
> collector needs the old and the new arena at once, about twice the live data, plus
> what the host allocator fragments when the arena moves. In wasm, memory never shrinks,
> so that peak is what every run costs; sliding in place has no second arena, which
> matters most for allocation-heavy runs. The cost: two traversals instead of one, plus
> the bitmap work, so a collection takes longer, and the engine carries two collectors.

## The old region

Most of a fresh VM's heap is the base image: the realm and the compiled builtins, nearly
all of it live for the VM's whole life ([the base snapshot](snapshots.md#the-base-snapshot)).
A program that allocates a lot of short-lived data collects every few hundred kilobytes,
and a full collection would trace that whole base every time.

So the arena has an **old region**: a prefix `[0, B)`, with B stored in the `Roots` cell.
A **minor** collection treats every old cell as live. It does not mark them and does not
move them; it marks and slides only the **young** cells above B. What it needs to know
is which young cells the old ones keep alive, and the old cells are mutable: scripts add
globals, patch prototypes, fill caches and grow the realm's tables. Rather than track
every write, a minor collection reads the whole old region once:

1. **Walk.** It visits the old cells in address order, and every traced field that
   points at or above B is a root for the young cells. The field's address goes on a
   short list.
2. **Mark, count, update, slide** as before, but only over the young cells: the bitmap
   and the table start at B, and survivors slide down to B. Of the old cells, only the
   fields on the list are rewritten (if the list fills up, the update walks the old
   region again).

A few old cells need more than their fields read: an old WeakMap's pairs with young keys
join the ephemeron rounds and are cleared if the key dies, an old Map or Set with young
object keys is rehashed, and old function templates get their offset caches emptied.

The walk does not go through the general cell walker. For each cell kind, a table worked
out at compile time from the layouts lists where its `Value` and `Ref` fields are; the
walk loads each, compares it with B, and only a young target leads to a call. Reading a
field this way is several times cheaper than tracing it, so a minor collection over a
heap that holds just the base costs less than half of a full one.

**Where B comes from.** A VM made from the base image starts with B at the image's end.
An embedder can **seal** a heap with `Vm::seal` (`vm_seal` in wasm): a full collection,
after which everything live is old. A host that runs a prelude of its own (a lockdown,
helpers its scripts use) seals after it, so that prelude becomes old too. Nothing else
moves B up. Before that collection, `seal` copies the cells a session replaces first to
the end of the heap (Map and Set tables, the root shape's transitions, the global
object's property table, the atom table), so that their dead copies drop out at the end
of the old region instead of shifting it.

**Full collections** trace everything above the builtin prefix (next section). They run
every 16th collection, after a minor one that left more than half the memory cap in use,
whenever the embedder asks (`Vm::collect`), and whenever there is no old region (B = 0).
The first collection of a VM made from the base image is full too: setting up the VM
kills some base cells right away, and if they stayed for sixteen collections they would
shift every later threshold. A full collection keeps the old region's live cells and
drops its dead ones: B shrinks to the live bytes below it, but never grows.

**What it retains.** Between full collections, an old cell that died still holds its
fields, so the young cells only it reaches survive too: say, an old object's property
table after the object grew. In practice that is a few kilobytes, and the next full
collection frees it.

**Determinism.** B and the count of minor collections since the last full one are in the
`Roots` cell, like the threshold. Whether a collection is minor or full is decided from
the heap alone, so a restored VM collects the same way as the original. A full collection
slides everything into allocation order, so its result is the same whether minor
collections ran before it or not.

> **Trade-off.** A walk over the old region over a write barrier. A barrier that records
> stores into old cells (a card table) would let a minor collection read only the cells
> written since the last one. But it costs a compare on every traced store in the
> interpreter's hottest paths, and it has to be complete, raw stack writes included: a
> forgotten store frees a live cell. The walk has nothing to forget, and the per-kind
> tables keep it cheap.

> **Trade-off.** A sealed setup phase over promotion. A generational collector moves
> long-lived young cells into the old region, so later collections stop tracing them.
> In codemode, cells that survive one or two collections still tend to die soon:
> per-task data, a growing string's buffer that is replaced as it grows. Promoted, they
> would stay until the next full collection, and thresholds computed over them would
> raise peak memory. So the old region holds only what is old on purpose: the base and
> what the host sealed. The cost: a program with a large long-lived young heap traces
> it again at every minor collection.

## The builtin prefix

The **builtin prefix** sits below B and below anything an embedder seals. It is the
part of the base image that holds the builtins, and no collection moves or frees it. A
full collection walks it the way a minor one walks the old region, since its fields can
point at newer cells: the walk counts them as roots, and the prefix's cells themselves
are not marked and never slide. So every VM keeps the builtins at the offsets the base
gave them, and its images share those bytes with the base. In debug builds a check can
make sure nothing writes there: every arena write below the prefix's end panics, and
after every run and collection the prefix is compared with a copy.

> **Trade-off.** Never freeing the prefix. A script that reshapes a builtin (adds a
> property to `Array.prototype`, redefines one with `defineProperty`) gives it new
> storage, and the old storage stays in the prefix, dead, for the VM's life; a lockdown
> that redefines hundreds of builtins leaves some kilobytes that way. In return a full
> collection neither marks nor slides the base, and the prefix's bytes stay those of
> the base. With [frozen builtins](builtins.md#frozen-builtins) nothing in the prefix can
> change, so nothing is left behind.

## Weak references and finalization

WeakMap, WeakSet, WeakRef and FinalizationRegistry keep their weak entries in
**ephemeron tables**: arrays of key–value pairs that the marker does not trace like other
cells. Once everything else reachable is found, every pair whose key is reachable gets
its value traced too, which may make more keys reachable, and this repeats until nothing
changes. Pairs whose keys stayed unreachable are then cleared. That is all the weakness
these objects need:

- a **WeakRef** holds its target as the key of a one-pair table; `deref` finds the
  cleared key once the target died. A target that `deref` returned is kept until the
  current job ends, as the language requires;
- a **FinalizationRegistry** keeps its targets in such a table and its held values in an
  ordinary array. After a collection, the interpreter queues a cleanup job for every
  registry with a target that died (an embedder can choose to schedule these itself).

The tables are processed in a fixed order, so which pairs die depends only on the object
graph. But weak references make *when* collections run observable to a script, so a
host can make them strong (the tables become ordinary arrays, and nothing they hold ever
dies) or absent ([determinism](determinism.md), the `weakRefs` option in [the JavaScript
interface](js-api.md#creating-a-sandbox)).

## Slices and the strings they keep

A [slice](heap.md#strings) reads its units from its parent string, so it keeps the
parent alive. That is what makes it cheap, and also its risk: 100 snippets of 60
characters cut from a 5 MB log would keep the 5 MB. A full collection therefore treats a
slice's reference to its parent as weak while it marks:

1. Marking does not follow a slice to its parent.
2. After the mark, a walk over the marked slices sums, for each parent that nothing
   else reached, the units its slices cover.
3. A parent of at least 4,096 units that its slices cover less than a quarter of is
   still kept for this collection, since its slices read it, but each of its slices is
   flagged. Every other parent is marked and stays.
4. Right after the collection, each flagged slice gets a copy of its units and becomes a
   flattened rope in place, and a second pass of the collector drops the parents. An
   image taken right after a collection does not carry them.

The rule depends only on the heap at the collection, so it is as deterministic as the
collection points. Minor collections keep every parent.

> **Trade-off.** Up to three quarters of a parent stay alive for the slices that cover
> the rest. A higher share would copy more and keep less, but the copies are made all
> at once, right after a collection, while the parents still exist: with a half instead
> of a quarter, an image of two logs cut into pieces shrinks to less than half, and the
> peak during the copy doubles. The share is a constant in the collector.

## Copying, for canonical layouts

The copying collector reserves a new arena as large as the old one's used part, copies
the `Roots` cell to offset 0, and then scans the new arena from the start, cell by cell:

![A Cheney collection: live cells are copied from the old arena into a new one, which is scanned from offset 0 until the scan catches up with the copy point](img/cheney.svg)

For every traced field of a scanned cell, the target is copied to the end of the new
arena unless it already was, and the field is rewritten with the new offset. A copied
cell gets the `FORWARDED` bit in its old header and its new offset in its old first
payload word, which is why every cell kind has at least one. When the scan catches up
with the copy point, everything reachable has been copied, and the old arena is dropped.
A heap with a builtin prefix keeps it: the copy starts with the prefix's bytes as they
are, and the prefix's fields are rewritten like those of any copied cell.

The order is breadth-first from the roots, in field order, so the layout depends only on
the object graph, not on the order in which cells were allocated. djs uses it where that
matters:

- the **canonical hash** collects a copy of the heap this way and hashes the result, so
  two VMs with the same graph have the same hash whatever their history. A slice comes
  out as the flat string it reads, so the hash does not depend on which strings are
  slices either ([canonical hashes](determinism.md#canonical-hashes-and-journals));
- the **base image** is made this way, so it doesn't depend on how the prelude
  allocated.

## When it runs

Allocation never collects. It sets `gc_pending` once the arena crosses the **GC
threshold**, and the collection runs at the next [safepoint](suspension.md#safepoints): a
frame entry, a return, a backward jump, a loop back-edge of a native builtin. After a
collection, `next_threshold` sets the next threshold from the live size:

- let the heap grow by as much as is live (at least 256 KiB), so it can reach twice its
  live size;
- round the threshold up to whole 64 KiB wasm pages, counted from the end of the builtin
  prefix;
- but never grow by more than half of what is left below the cap, so that collections
  come more often as memory gets tight.

The builtin prefix, which is never collected, counts toward the live size as a fixed
allowance instead of its real size, and pages are counted from its end: a base a few KB
smaller or larger then moves every threshold by exactly that much, so collections still
come after the same allocations. The arena's backing store stops growing at the
threshold ([growing, and the cap](arena.md#growing-and-the-cap)), since wasm memory never
shrinks and the peak is what a run costs.

> **Trade-off.** Twice the live size, not more. Minor collections skip the old region,
> so a collection's cost is mostly the young data that survives it, and a larger growth
> factor would mean fewer collections and a few percent fewer instructions in
> collection-heavy runs. It would also raise their peak memory by up to a sixth, and in
> wasm the peak is what a run costs, so djs takes the smaller peak.

Three rules make the collection points a function of the program alone:

- the threshold is stored in the `Roots` cell, so a VM restored from an image collects
  where the original would have;
- the threshold is computed from the live size and the cap only, never from what the
  host allocator happened to reserve;
- `run` does not collect when it starts, and nothing a stop does allocates in a way that
  counts.

So a program run in one go, in slices of 97 fuel, or with an image saved and restored at
every slice collects at the same instructions and leaves the same bytes.

```js live
let kept = [];
for (let round = 0; round < 50; round++) {
  const batch = [];
  for (let i = 0; i < 200; i++) batch.push({ round, i, label: "item " + i });
  kept.push(batch[0]);            // keep one object per round, drop the rest
}
console.log(kept.length, kept[49].label, kept[49].round);
```
<!-- gen:run -->
```text
50 item 0 49
```
<!-- /gen -->

On the docs site, the *Heap* tab shows the cells this example allocated, by kind, apart
from the base realm every VM starts with, and its *Collect now* button runs a collection
in the page: most of the ten thousand objects disappear, and the fifty kept ones are
packed in with the rest of the live heap.

## Out of memory

The embedder gives each VM a memory cap, and running into it is a JavaScript error the
script can catch, at the same point in every run ([limits](limits.md#memory) shows what
a script sees). Since allocation never collects, the heap at the moment of failure may be
mostly garbage, so djs does three things:

1. **Collect and retry.** Before an out-of-memory error reaches JavaScript, the failing
   step collects and runs once more. For bytecode, the step is the instruction:
   instructions that allocate before they change anything (`put_field`, `add`, `object`,
   a few intrinsic calls) have their operands put back on the stack, the heap collects,
   and the instruction runs again. Native builtins do the same at the points where their
   generated code holds no heap value outside the frame.
2. **A preallocated error.** If the retry fails too, the realm's one
   `InternalError: out of memory` object is thrown. It is never allocated at the throw.
3. **A reserve.** The arena's limit is the cap minus `min(cap / 32, 1 MiB)`. Throwing
   the error opens the reserve and collects when the handler is reached, so a handler
   that formats the error and reports it has room to run.

A refusal by the *host* allocator below the cap (the backing store cannot grow, the
collector's side tables cannot be allocated) is not a function of the program, so it is
not catchable: the run stops with `HostOutOfMemory`, and the VM stays usable.

> **Trade-off.** Retry at instruction granularity over collecting inside allocation.
> Collecting on an allocation failure would rescue every allocation site, but every Rust
> function that holds a heap value across an allocation would then need to root it, in
> hundreds of places. Retrying the whole instruction needs a list of the instructions
> that may retry and nothing else, at the price of an instruction that runs twice when
> memory is tight.

## What is never collected

Two kinds of heap data live as long as the VM: **shapes**, because weak transitions
would make the shape tree depend on collection points ([shapes](heap.md#shapes)), and
**atoms**, the interned property keys ([atoms](heap.md#atoms-property-keys-as-integers)).
Both are bounded in practice; a script that makes millions of distinct keys keeps their
atoms until the VM is dropped.

## Where to look

- [`crates/djs-core/src/heap/compactor.rs`](../../crates/djs-core/src/heap/compactor.rs):
  the in-place collector, full and minor, the per-kind field tables and the slice rule.
- [`crates/djs-core/src/heap/gc.rs`](../../crates/djs-core/src/heap/gc.rs): the copying
  collector, the collection schedule (`collect_auto`), `seal` and `next_threshold`;
  [`crates/djs-core/src/heap/gc_tests.rs`](../../crates/djs-core/src/heap/gc_tests.rs):
  its tests, among them that collecting a collected heap changes nothing.
- [`crates/djs-core/src/heap/walk.rs`](../../crates/djs-core/src/heap/walk.rs): tracing
  from the layout table, and the verifier.
- [`crates/djs-core/src/heap/handle.rs`](../../crates/djs-core/src/heap/handle.rs): the
  handle table.
- [`crates/djs-core/src/heap/weakref.rs`](../../crates/djs-core/src/heap/weakref.rs):
  WeakRef and FinalizationRegistry on ephemeron tables.
- [`crates/djs-core/src/heap/compact.rs`](../../crates/djs-core/src/heap/compact.rs):
  the cells `seal` moves to the end of the heap.
- [`crates/djs-core/src/interp/run.rs`](../../crates/djs-core/src/interp/run.rs):
  safepoints and the collect-and-retry of a failing allocation.
