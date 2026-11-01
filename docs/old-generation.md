# A sticky base region: not re-tracing the base image at every collection (0101 follow-up)

Status: Phase 1 built in 0120 (D2041–D2044), with the changes listed in section 10.
Phase 2 (cards) is not built. It builds on the 0101 mark-compact prototype
(docs/mark-compact.md); a variant for Cheney is in section 4.

## 1. Why

Every collection traces the whole base image again. That is about 380–460 KB of prelude
code, builtins and the realm, nearly all of it live forever.

| | Cheney | mark-compact |
| --- | ---: | ---: |
| collector share of native samples in `props` | 13.8% | 17.7% |

Most of that is the base. A collection of a heap holding just the base (456 KB, native
release, vm/tests.rs `base_region_scan_cost`) takes:

| what | time |
| --- | ---: |
| Cheney | 0.14–0.15 ms |
| mark-compact | 0.15 ms |
| a **linear scan** of the same region for references out of it | **0.050 ms** (3× less) |

A script that allocates a lot of short-lived data collects every 256 KB or more (MIN_GROWTH)
and pays the base each time.

## 2. What the base holds, and what changes after the image is loaded

From the base image's attribution (`cargo run -p djs-prelude --example image-size`):

| cells | share | after load |
| --- | ---: | --- |
| bytecode, handler tables (Bytes) | ~26% | immutable |
| strings (Str8/Str16), Symbols, Script, BigInt | ~21% | immutable. A flat string is never written, and there are no append buffers in images. |
| FuncTemplates | ~13% | mostly immutable, but **`IC`** is set lazily to a new U32Array, and the collector empties offset slots (D1621) |
| function Objects, prototypes, namespaces, their PropValues/PropTables, AccessorPairs | ~19% | **mutable.** pi's lockdown freezes 485 builtins (each becomes a dictionary, a new table); scripts add globals and patch prototypes |
| capture descriptors, names (U32Array), RefArrays of child templates | ~4% | immutable |
| constant pools (ValueArray) | ~3% | mostly immutable. Tagged templates cache their template object in them on first use: an old→young reference |
| Shapes, descriptor and transition arrays | <1% | **mutable.** New children replace a shape's `TRANSITIONS` array; descriptor arrays get appended |
| BoxCells (the prelude's captured state) | ~1% | **mutable** |
| Roots, Interp, the value stack, HandleTable, AtomTable (+ entries, index), Realm, global object and lexical record, job queue | small | **mutable on every run** |

So the base is neither immutable nor a few mutable hubs. Writes into it are rare per cell
but spread out: 0088 measured that pi's lockdown changes 1,131 base cells on 62 of its
101 4 KiB pages. The design must find every old→young reference without listing them.

## 3. Design: a sticky, non-moving base region

- **The boundary.** `Roots::OLD_END` (new, in the image) marks a prefix `[0, B)` of the
  arena as *old*.
  - `vm_new` sets B to the base image's length.
  - `Vm::seal()` sets B to `len` right after a full collection. pi would call it after
    its prelude and its D1890 collection, so warm sessions start with the post-prelude
    image as old.
- **Minor collections** (the default):
  - Old cells are live by definition. They are not marked, not moved and not freed.
  - Young cells are traced from the roots cell and from **every reference in the old
    region to a young cell**.
  - Phase 1 finds those by a linear walk over the old cells, reading each traced slot and
    keeping the ones with a target `≥ B`. No write barrier, no mutator cost.
  - Each found target is marked (mark-compact) or forwarded (Cheney), as if the slot
    were a root. A second walk rewrites those slots after the young cells have moved.
- **Special cells in the old region** are handled during the walk:
  - **Ephemeron tables:** pairs with a young key take part in the fixpoint like young
    tables. They are not roots.
  - **Map/Set with young keys:** rehashed (D1381), since those keys move.
  - **FuncTemplates:** their offset IC slots are emptied as today (D1621). Offsets of old
    cells never change, but slots can hold offsets of young shapes and prototypes.
  - **A flattened rope:** if it is old, it stays old; its `LEFT` keeps the flat string
    alive.
- **Compaction starts at B.** Young cells slide down to B. Forwarding, the bitmap and the
  block table cover only `[B, len)`, which also makes the side tables smaller.
- **Full collections** reclaim dead old cells: the dictionary-converted builtins' old
  PropValues, replaced transition and entry arrays, and lockdown leftovers. They run:
  - at `Vm::seal()`;
  - before an image is cached or snapshotted (pi already collects there, D1890);
  - on a deterministic schedule, e.g. every 32nd collection or when young live bytes
    exceed B. The counter is in `Roots`.

  Until then dead old cells, and young cells reachable only from them, are retained. The
  retained amount is bounded by what the script dirtied in the old region. For a plain
  base it is a few KB; after a lockdown that wasn't sealed, up to ~100 KB.

**Phase 2 (only if measured worthwhile): cards.** A write barrier in `Arena::write`/`set`
for `Value`/`Ref` stores marks a card (one byte per 512 B of the old region, ~900 B for
the base) when `off < B`. The minor walk then visits only dirty cards; a card whose cells
point nowhere young after a collection becomes clean again.
- **Cost:** a compare on every traced store. Bounds and caveats:
  - The interpreter's value stack, Roots, Interp and the HandleTable are scanned at every
    collection anyway, so they can be exempt.
  - run_hot's raw stack writes (hot.rs) touch only the stack, but this has to be audited.
- **Benefit:** it pays when the old region stays mostly clean.
  - If B is the plain base and pi locks it down, 62% of the pages are dirty, so cards save
    little.
  - If B is the sealed post-prelude image, the lockdown is inside it, and a script's
    writes into old cells are few.

## 4. With Cheney

The same split works for the copying collector:
1. copy `[0, B)` verbatim to the same offsets (a memcpy);
2. scan it linearly for young references (forwarding them);
3. run Cheney over the young cells.

It still needs a to-space for the whole arena, so peak memory is unchanged. The GC time
gain is the same.

## 5. Determinism

- **B and the full-collection schedule are image state.** B is in `Roots`, and the
  schedule counter is too. Whether a collection is minor or full is a function of the
  image (D1601). The same history gives the same raw images under any slicing and
  restore (D1710), natively and in wasm.
- **The canonical hash is unchanged (D1730).** Cheney on a copy traces the whole graph
  from the roots and drops dead old cells, so it ignores B. Journals stay canonical even
  across different B (sealed or not).
- **Raw images do change.** Between full collections they keep dead old cells. The gate
  compares raw images of runs with the same history (still equal). The "after a final
  collection" checks should use the canonical hash (0101's question 1), or a full
  collection.
- **Sliding still keeps allocation order.** Compacted images still don't depend on when
  minor collections happened, as long as full collections come at deterministic points.

## 6. Snapshots

- **This is a GC design, not a snapshot format.** The old-generation *delta* plan in
  ARCH "Deltas" stays superseded (0088). Images remain the raw arena, old region
  included.
- **0088's 62-of-101 pages argue against deltas, not against this.**
  - Restoring an image keeps its B, so a restored session behaves exactly like the
    original.
  - The image may hold dead old cells, so pi should take its images after a full
    collection (it collects there already), making image sizes as today.
- **`seal()` after pi's prelude makes the lockdown part of the old region.** The 1,131
  changed cells become plain old cells: no dead copies, nothing dirty.

## 7. Expected gain

| workload | expected gain |
| --- | --- |
| base-dominated, like `props` | GC time −55–65% (the old region costs a scan instead of a trace, 3× less); runtime −9–11% |
| pi-like, json, jsonh, strings (several MB of young live data per collection) | −3–6% |
| arrays, higher-order (big young live sets) | small |
| with cards (Phase 2) and a sealed image | the old scan drops to the dirty cards, so a collection costs about the young live data alone |

The props estimate assumes its GC share is 13.8–17.7% and mostly the base.

**Code:** Phase 1 is about 150–250 lines in the collector (the boundary, the old walk, the
schedule, `seal`), estimated +1–2 KB of wasm. Phase 2 adds the barrier to every traced
store path, the card table and its clearing.

## 8. Risks

- **A missed old→young reference frees a live cell.** Phase 1 has no barrier to forget:
  the walk reads every slot. Phase 2 would need a complete barrier, including raw writes.
- **Retention between full collections.** The schedule bounds it; seal and snapshot
  points reset it.
- **Always walking the old region.** In Phase 1 every collection still costs about 0.05
  ms per 450 KB.

## 9. Proposal

Build Phase 1 after 0101 (it needs the in-place collector's fixed prefix), then measure it
on 0081/0087's workloads and the pi bench. Decide on Phase 2 with those numbers and with pi
sealing its post-prelude image.

## 10. As built (0120)

Phase 1 is in heap/compactor.rs and heap/gc.rs. It differs from the design above in these
points:
- **No promotion at full collections (D2042).** A full collection shrinks B to the live
  bytes below it. Only `seal` and canonical collections set B = len.
  - Making every full collection's survivors old was measured. It raised peak memory by up
    to 76% (higher-order 28.4 → 45.3 MiB, jsonh 6.8 → 12.0, pi-like 4.2 → 6.3): promoted
    data that died stayed until the next full collection.
- **A remembered list.** The mark walk records the old slots that hold young references (a
  fixed 16 KiB list, reserved with the side tables). The update pass rewrites only those,
  so the old region is walked once, not twice. A full list falls back to the second
  walk. Old templates' offset caches are emptied during the mark walk.
  - On a base-only heap the minor collection took 0.155 ms with two walks and 0.080 ms
    with the list; a full collection takes 0.177 ms and the bare scan 0.043 ms.
- **Old collections** are rehashed only if they have object or symbol keys ≥ B. They are
  flagged with the free `gc_bits::REHASH` (formerly `OLD`).
- **Schedule:**
  - Minor collections while B > 0, full every 16th.
  - A minor collection that leaves more than half the limit in use makes the next one full.
  - With B = 0 (`Vm::new` before a seal) every collection is full.
  - `Vm::collect` (and wasm `vm_collect`) is always full; `Vm::seal` / `vm_seal` is full,
    then B = len.
- **pi** seals after its prelude, in place of the plain collection of D1890. pi's
  snapshots still call `collect` (full).
- **Cold starts (D2043):** `vm_new_reserved(limit, live)` makes a VM from the base with the
  arena capacity a heap of `live` bytes gets. pi passes the size its previous cold start
  sealed; cold peak memory drops 3.00 → 2.25 MiB.
- **Measured:** collection time −8% to −51% (props −51%, pi-like −38%, strings −42%);
  wasm instructions pi-like −5.7%, props −5.3%, the others about ±0. Peak memory is
  unchanged except for allocator-placement effects (strbuild +6.4 MiB, strings
  −1.2 MiB). The numbers are in D2041.
- **Phase 2 is not built (D2044).** Cards could save at most the old walk, ~2–2.5% of
  props and pi-like, for a barrier on every traced store.
