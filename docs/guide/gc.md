# The garbage collector

Every VM state djs keeps is in the arena, and the arena only grows by bumping. Something
has to give the memory of dead objects back, and in djs that same thing decides what a
snapshot contains and whether two runs of a program leave identical images. This
chapter explains the collector, when it runs, what happens when memory runs out, and
what it costs.

> **In short.** djs uses a copying (Cheney) collector: it copies everything reachable
> from the roots into a fresh arena, in breadth-first order, and drops the old one. The
> result is compact, and its layout is a function of the object graph alone. Collections
> happen only at safepoints, when allocation has crossed a threshold that is itself part
> of the heap, so collection points depend on the program and never on how it was
> sliced. Running into the memory cap is a catchable error, with a collect-and-retry
> before it.

## Copying, breadth-first

A collection reserves a new arena as large as the old one's used part (which bounds
what can be live), copies the `Roots` cell to offset 0, and then scans the new arena
from the start, cell by cell (D1090):

![A Cheney collection: live cells are copied from the old arena into a new one, which is scanned from offset 0 until the scan catches up with the copy point](img/cheney.svg)

For every traced field of a scanned cell, the target is copied to the end of the new
arena unless it already was, and the field is rewritten with the new offset. A copied
cell gets the `FORWARDED` bit in its old header and its new offset in its old first
payload word, which is why every cell kind has at least one. When the scan catches up
with the copy point, everything reachable has been copied, and the old arena is
dropped.

What counts as a traced field comes from the layout table alone: a field declared as
`Value` or `Ref` in `cells!` is traced, nothing else is (D1052). There is no per-kind
tracing code to forget when a cell kind is added. A few kinds get extra treatment on
the way:

- a reference to a **flattened rope** is redirected to its flat string, so the rope
  dies;
- **Map and Set** tables, whose object keys hash by offset, are rehashed in place after
  the copy (D1381);
- **ephemeron tables** (WeakMap, WeakSet, WeakRef, FinalizationRegistry) are not traced
  by the scan. When the scan catches up, every pair whose key was copied gets its value
  copied, the scan resumes, and this repeats until nothing changes; pairs with dead keys
  are then cleared (D1382, D1513);
- inline cache slots that hold arena offsets are emptied (D1621).

The order is breadth-first from the roots, in field order, so the new layout depends
only on the object graph. Collecting a collected heap reproduces it byte for byte. That
is the property snapshots and the determinism checks build on.

> **Trade-off.** Copying over mark-sweep. A copying collector compacts for free: the
> image after a collection has no holes, which keeps snapshots small, and its layout is
> canonical, which makes images comparable. It is also short: one loop over the
> to-space, driven by the layout table. The costs are memory and copying. During a
> collection the old and the new arena both exist, so the peak is about twice the live
> data, plus what the host allocator fragments (D1990). And every live cell is copied
> every time, including a 1 MiB tool result that never changes: there is no
> non-moving space for large strings and buffers yet. Issue 0101 is designing an
> in-place mark-compact collector to remove the doubled peak.

## When it runs

Allocation never collects (see [the arena](arena.md)). It sets `gc_pending` once the
arena crosses the **GC threshold**, and the collection runs at the next safepoint: a
call, a return, a backward jump, a state transition of a native builtin
([suspension](suspension.md)). After a collection the next threshold is set from the
live size:

```rust
pub fn next_threshold(live: u32, limit: u32) -> u32 {
    let headroom = limit.saturating_sub(live);
    let growth = live.max(MIN_GROWTH);
    if growth > headroom / 2 {
        return live + headroom / 2;
    }
    // Whole 64 KiB wasm pages when that stays within the headroom rule (D1870).
    let t = (live as u64 + growth as u64).next_multiple_of(PAGE);
    if t <= live as u64 + headroom as u64 / 2 {
        t as u32
    } else {
        live + growth
    }
}
```

That is: let the heap grow to twice its live size (at least 256 KiB more), rounded up to
whole wasm pages, but never by more than half of what is left below the cap, so that
collections come more often as memory gets tight. The arena's backing store stops
growing at the threshold, since wasm memory never shrinks and the peak is what a run
costs (D1870).

Three rules make the collection points a function of the program alone:

- the threshold is stored in the `Roots` cell, so a VM restored from an image collects
  where the original would have (D1601);
- the threshold is computed from the live size and the cap only, never from what the
  host allocator happened to reserve;
- `run` does not collect when it starts, and nothing a stop does allocates in a way
  that counts (D1710).

So a program run in one go, in slices of 97 fuel, or with a snapshot and restore at
every slice collects at the same instructions and leaves the same bytes. The gate checks
exactly that (D1720).

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

On the docs site, the *Heap* tab shows this example's arena by cell kind after the run,
and its *Collect now* button runs a collection in the page: most of the ten thousand
objects disappear, and the fifty kept ones are packed in with the rest of the live
heap.

## Out of memory

The embedder gives each VM a memory cap. Running into it must be a JavaScript error the
script can catch, as in QuickJS, and it must happen at the same point in every run.
Since allocation never collects, the heap at the moment of failure may be mostly
garbage, so djs does three things (D1600, D1700):

1. **Collect and retry.** Before an out-of-memory error reaches JavaScript, the
   failing step collects and runs once more. For bytecode, the step is the instruction:
   instructions that allocate before they change anything (`put_field`, `add`,
   `object`, a few intrinsic calls) have their operands put back on the stack, the heap
   collects, and the instruction runs again (D1700). Native builtins do the same at the
   points where their generated code holds no heap value outside the frame (D1900), and
   `@fast` entries decline, so the builtin's own path retries (D1910).
2. **A preallocated error.** If the retry fails too, the realm's one
   `InternalError: out of memory` object is thrown. It is never allocated at the throw.
3. **A reserve.** The arena's limit is the cap minus `min(cap / 32, 1 MiB)`. Throwing
   the error opens the reserve and collects when the handler is reached, so a handler
   that formats the error and reports it has room to run.

A refusal by the *host* allocator below the cap (the backing store cannot grow, the new
arena cannot be reserved) is different: it is not a function of the program, so it is
not catchable. The run stops with `HostOutOfMemory` and the VM stays usable (D1690).

> **Trade-off.** Retry at instruction granularity over collecting inside allocation.
> Collecting on an allocation failure would rescue every allocation site, but every
> Rust function that holds a heap value across an allocation would then need to root
> it, in hundreds of places. Retrying the whole instruction needs a list of the
> instructions that may retry and nothing else, at the price of an instruction that
> runs twice when memory is tight.

> **Trade-off.** Strong shapes and atoms that never die. Two kinds of heap data are
> never collected: shapes, because weak transitions would make the shape tree depend on
> collection points (D1831), and atoms, because atom collection is not built yet
> (D1054). Both are bounded in practice; a script that makes millions of distinct keys
> keeps their atoms until the VM is dropped.

## Where to look

- [`crates/djs-core/src/heap/gc.rs`](../../crates/djs-core/src/heap/gc.rs): the
  collector and `next_threshold`;
  [`gc_tests.rs`](../../crates/djs-core/src/heap/gc_tests.rs): its tests, including
  "collecting twice gives identical bytes".
- [`crates/djs-core/src/heap/walk.rs`](../../crates/djs-core/src/heap/walk.rs): tracing
  from the layout table, and the verifier.
- [`crates/djs-core/src/interp/run.rs`](../../crates/djs-core/src/interp/run.rs):
  safepoints and the retry of D1700.
- ARCH.md, "Heap" (GC and the memory cap).

Decisions on this page:

<!-- gen:decisions D1090 D1052 D1381 D1382 D1513 D1621 D1990 D1870 D1601 D1710 D1720 D1600 D1700 D1900 D1910 D1690 D1831 D1054 -->
- D1090: Copying collector
- D1052: Layout description macro
- D1381: Map and Set are ordered hash tables rehashed by the collector
- D1382: WeakMap and WeakSet are ephemerons in the Cheney collector
- D1513: WeakRef and FinalizationRegistry on ephemeron tables; cleanup jobs after a collection
- D1621: Prototype property caches: a receiver-prototype offset, an epoch, and GC clearing
- D1990: A restored arena reserves its GC threshold plus a slack
- D1870: Arena growth stops at the GC threshold; thresholds in 64 KiB pages; no to-space reuse
- D1601: The GC threshold is part of the heap image
- D1710: GC points and raw images do not depend on the fuel slice
- D1720: Determinism checks in the gate: heap images across schedules and builds
- D1600: Out-of-memory recovery: a collection at the handler and a reserve below the cap
- D1700: A failing allocation collects and retries its instruction once
- D1900: Native builtins collect and retry at the heap cap
- D1910: Dense-array loops, fromCharCode and the URI functions as `@fast`; entries decline at the heap cap
- D1690: Running out of host memory is not a JS out-of-memory error
- D1831: Shapes with strong transitions and a per-heap shape budget
- D1054: Atom representation; symbols are atoms
<!-- /gen -->
