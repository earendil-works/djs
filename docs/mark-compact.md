# In-place mark-compact collection (0101)

Status: proposal for review (0101). ARCH change. Decision ids D2040+.

## 1. Why

0099 measured what is left of wasm peak memory. Cheney needs the from-space and the
to-space at once, about twice the live data. dlmalloc never returns memory, and each move
of a grown arena leaves a hole that the next, larger arena does not fit. A cold pi start
peaks at 2.81 MiB (`return 1`) to 4.00 MiB (tool-heavy). Every heavy workload's peak
includes one extra semispace.

A sliding collector that compacts in place removes the to-space. The peak during a
collection becomes the arena plus side tables of about len/43 bytes. The estimate is
about 2.0 MiB for a cold `return 1`, and one semispace less at the peak of every heavy
collection.

## 2. Algorithm

A Compressor-style sliding collector: mark, compute forwarding from bitmaps, update
references, slide. Cells keep their address order, and the result is dense from offset 0.

**Side tables**, allocated in Rust memory before anything is touched:
- a **live-word bitmap**: one bit per 8-byte arena word, set for every word of a marked
  cell (len/64 bytes);
- a **block table**: one `u32` per 512-byte block (one `u64` of the bitmap), holding the
  new address of the block's first live word (len/128 bytes);
- a **bounded mark stack**: a fixed 16 K entries (64 KiB), with overflow handling (below).

For a 2 MiB arena that is about 48 KiB plus the stack. If the host refuses these
allocations, `collect` returns `HostOutOfMemory` with the heap untouched (D1690, as the
to-space reservation does today). No allocation happens after that point.

**Pass 1, mark.**
- Start from the roots cell (offset 0, always live) and trace every `Value`/`Ref` slot
  with the existing `walk::for_each_slot`.
- Marking a cell sets `gc_bits::MARK` in its header. That bit was reserved for this and
  is 0 between collections. It also sets the cell's words in the live bitmap, then pushes
  the cell.
- Marking order doesn't affect the result: only the *set* of marked cells reaches the
  layout. So the stack discipline, the overflow handling and the ephemeron fixpoint below
  can't make the result nondeterministic.
- **Stack overflow:** a cell is still marked but not pushed, and a flag is set. When the
  stack drains with the flag set, a linear walk rescans the marked cells (children only;
  marking is idempotent) and repeats until no overflow is left. This is the classic
  bounded-stack scheme. Deep lists cost extra walks, never unbounded memory.
- **Flattened ropes:** a reference to a flattened rope marks its `LEFT` (the flat string)
  instead, as Cheney's `forward` does. The rope itself stays unmarked and dies.
- **Ephemerons** (D1382, D1513: WeakMap/WeakSet entries, WeakRef targets,
  FinalizationRegistry cells): an `EphemeronArray` cell is marked (the table lives if
  reachable), but its pairs are not traced. Once the stack drains, a walk over the marked
  ephemeron tables marks the value of every pair whose key is marked, then drains again.
  This repeats until a round marks nothing new. Then the pairs whose key is unmarked are
  cleared in place (`HOLE` key, `undefined` value), which is what Cheney does.
  - A counter of marked ephemeron tables skips all of this when there are none.
  - The walks use the live bitmap, so they skip dead space a 64-word block at a time.

**Pass 2, block table.** A prefix sum of `popcount` over the bitmap words gives each
block's new start. The forwarding address of a live cell at `a` is
`table[a / 512] + 8 · popcount(bitmap_word & ((1 << (a % 512 / 8)) - 1))`: a load, a
mask and a popcount. Nothing is stored in the cells, so the headers stay intact for
passes 3 and 4.

**Pass 3, update.** Walk the marked cells in address order. The bitmap gives cell
starts; dead runs are skipped a 64-bit word at a time.
- Every traced slot is rewritten with its target's forwarding address. A flattened-rope
  target is replaced by `LEFT`'s forwarding address, as in Cheney: the tag bits of a
  `Value` are kept and only the offset changes.
- `FuncTemplate`s get `clear_offset_caches` (D1621, 0083's shape slots). That is the same
  function and the same rule as Cheney: every offset slot is emptied, so warm and cold
  caches behave alike and a second collection changes nothing.
- The roots cell is updated like any other; it stays at 0.

**Pass 4, slide.** Walk the marked cells in address order again.
- `copy_within` each to its forwarding address (always at or below its old address, so
  moves never overlap the wrong way) and clear `MARK`.
- Zero `[new_len, old_len)`: the arena hands out zeroed cells, and bytes past `len` must
  stay zero.
- Set `len = new_len`.
- Finally, a walk over the now dense heap rehashes every Map/Set/WeakMap/WeakSet
  (`collection_rehash`, D1381: object keys hash by offset), as Cheney does for every
  collection it copied.

Afterwards the threshold, `gc_pending` and the reserve (D1600) work as today. The arena
keeps its capacity: no move, no hole. The D1870 rule still applies: it shrinks if the
capacity is more than twice the new threshold.

**Live bytes, and so GC points, are collector-independent.** Both collectors keep the
same set of cells (reachable, minus flattened ropes) at the same sizes, so `after` is the
same and `next_threshold` gives the same next threshold. The VM's schedule of
collections (D1601, D1710) does not depend on which collector runs.

## 3. Which collector runs when

| use | collector | why |
| --- | --- | --- |
| in-VM collections: safepoints, gc-stress, D1700 retry, D1600 handler, `Vm::collect`, `vm_collect` | **mark-compact** (`Heap::collect`) | peak memory |
| `Vm::canonical_hash` (D1730), on its copy | **Cheney** (`Heap::collect_canonical`) | journals and replay compare graphs, not histories; BFS layout is a function of the graph alone |
| image canonicalization: the base image (`djs-prelude` build.rs, after `compact_for_image`), and any embedder image that wants a canonical form | **Cheney** | the image becomes independent of the prelude's allocation history; BFS also keeps related cells together |

Cheney stays in the build: `vm_canonical_hash` needs it in wasm. Mark-compact never falls
back to Cheney: Cheney needs more memory, not less.

## 4. Determinism

- **Collected images are deterministic but history-ordered.** After a mark-compact
  collection the heap is in address order, which is allocation order and reflects the
  run's history. The same history always gives the same bytes: the algorithm has no
  hidden inputs, and the mark set does not depend on mark order.
- **The gate's checks keep holding.**
  - e2e plain vs suspension, difftest `--determinism` at 97/1009 fuel, and native vs
    wasm all compare runs with *identical histories*. D1710 makes the raw arenas
    byte-identical under any slicing and restore, so the images after a final
    mark-compact are identical too.
  - GC stress remains output-only (D1720), as today.
  - "Collect twice, identical bytes" still holds: a second mark-compact finds no
    garbage, moves nothing, and clears the same (already empty) offset slots.
- **One semantic shift in D1720.** Today an image difference after a final collection
  means "the runs built different graphs". With mark-compact it means "different graphs
  or different garbage placement". Raw images already catch the second kind (D1725). I
  propose that the "after a final collection" hashes of e2e/difftest/abidrive use
  `canonical_hash` (Cheney on a copy), so that this check keeps its graph meaning
  exactly. The raw-image comparisons stay as they are.
- **`canonical_hash` is unchanged:** Cheney on a copy (D1730). Journals stay
  graph-canonical, and replays with different collection points still match.
- **Images from earlier builds still load.** The cell format is unchanged, only placement
  differs, so the layout hash does not change for this. The build hash changes only
  through the code.
- **Suspension records (D1710).** A host `Vm::collect` while suspended used to move the
  record (BFS) and orphan it. Now the record usually stays last and `release_top` can
  free it. This only differs in a case D1710 already lists as schedule-dependent.

## 5. D1600 / D1690 / D1700

- **Failure before the heap is touched.** The only allocations are the side tables
  (about 2.3% of the arena plus 64 KiB), made up front. If the host refuses,
  `HostOutOfMemory` comes back with the heap untouched (D1690). A collection can no
  longer fail for want of a to-space, so a near-cap VM gets HostOutOfMemory less often.
- **The cap.** Today the to-space does not count against it (D1090). Side tables don't
  either.
- **D1700's collect-and-retry is unchanged.** It collects at an instruction boundary
  with the operands rooted. Live bytes, and so the retry's outcome, are the same with
  either collector.
- **D1600's reserve:** `maybe_close_reserve` runs after the collection, as now.

## 6. Speed

Cheney does one pass over live data (copy plus scan). Mark-compact does four:
1. mark: like Cheney's scan, with random header writes;
2. the bitmap prefix sum: len/512 popcounts, negligible;
3. update: a second trace of every live slot, plus the forwarding arithmetic;
4. slide: one `memmove` of the live bytes, plus a final rehash walk.

The dead space costs only bitmap words. Expected: 1.5–2.5× Cheney's collection time.
Collection is a small share of most runs. I'll measure it on 0081/0087's workloads
(strbuild, higher-order, arrays, json, jsonh, forof, pi-like, props, strings, loop) and
on the pi bench, in wasm and natively:
- time: wasm instructions and wall clock;
- memory: peak wasm memory.

Proposed acceptance:
- peak memory down on every workload (expected: one semispace at heavy collections;
  the pi cold start near 2 MiB);
- total time within +3% on the engine track. If a workload with a high collection share
  (strbuild, higher-order) loses more, I'll report it with the trade, not hide it.

## 7. Code size (D016)

A second collector of about 350–450 lines: the bitmap marker with overflow rescan, the
ephemeron fixpoint, the block table, the update and slide walks, the rehash walk. It
reuses `for_each_slot`, `clear_offset_caches`, the rope check and `collection_rehash`.
Estimate: +5–8 KB of wasm code (opt-level s), measured on the gate's wasm build. Cheney
stays for `canonical_hash` and images. Its in-VM-only parts (the arena swap, the
threshold, the capacity) can go into a shared tail, so the net cost is somewhat less.

## 8. Tests

- GC tests run with both collectors (heap/gc_tests.rs's random graphs):
  - the same live graph (compared by a canonical dump);
  - collect twice gives identical bytes;
  - ephemeron chains and cycles;
  - WeakMap entries die with their keys;
  - flattened ropes short-circuited;
  - Map/Set rehash;
  - IC slots emptied;
  - mark-stack overflow (a small stack forced in tests, a long linked list).
- The 0071 slice tests (tests/slices.rs), e2e image equality, difftest `--determinism`
  and native-vs-wasm unchanged and green.
- `canonical_hash` equal before and after `Vm::collect` (it already is a test), and
  equal across collection points with mark-compact in-VM.
- Memory and time numbers as in section 6.

## 9. Open questions for the owner

1. Use `canonical_hash` for the gate's "after a final collection" comparisons (section
   4)? That is my recommendation.
2. Base image via Cheney (BFS, graph-canonical) or via mark-compact (no difference in
   size)? I propose Cheney.
3. Should `Heap::collect` keep its name for mark-compact, with Cheney as
   `collect_canonical`? The embedder API (`Vm::collect`, `vm_collect`) would mean "in
   place".
