# 0009: Copying GC and handle table

status: done
agent: heap
branch: issue/0009-copying-gc
depends-on: 0005
touches: crates/djs-core/src/heap/

## Goal
The Cheney copying collector from ARCH.md ("Heap" → GC), driven by the roots cell and
the `LAYOUTS` trace descriptions from 0005. It runs only at safepoints (allocation
never collects; the caller asks for a collection when `gc_pending` is set). Plus a handle
table, so the embedder and Rust code can hold values across collections.

## Acceptance criteria
- [x] `gc_pending` is set by allocation once the heap passes a threshold (growth policy:
      for example, next threshold = live × factor, with a minimum). `collect()` is called
      explicitly, at a safepoint.
- [x] Cheney copy into a fresh arena: roots cell first, then a breadth-first scan using
      the layout trace descriptions, including variable-length tails (property arrays,
      elements, rope children, hash tables) and every `Value` slot. Forwarding uses the
      header GC bits.
- [x] Atom table and symbols: atoms stay alive (atom GC is a follow-up), but the strings
      behind them are moved correctly. Rehash wherever a table is keyed by offsets (it
      should not be: check and document).
- [x] Rope flattening during GC is optional. If you do it, document it.
- [x] Handle table in the arena (a roots slot): `new_handle(value) -> Handle(u32)`,
      `get`, `set`, `drop_handle`, with a free list. Handles are the only way Rust code
      keeps a reference across `collect()`.
- [x] Memory cap: the collection itself fits in the cap's accounting (to-space
      allocation), or returns `OutOfMemory` cleanly with the old heap intact.
- [x] Tests: random object graphs (properties, arrays, ropes, symbols, cycles) built
      through handles, then repeated `collect()` calls. Check them with a structural
      comparison (a deep compare function walking from the handles) and `verify()`.
      Garbage is actually freed (arena size shrinks). The image round trip (`from_bytes`)
      still works after a GC. Collecting twice is idempotent in layout, so the bytes are
      identical: a deterministic order matters for snapshots.
- [x] Rough numbers: collection time per MB live (native release), recorded in the Log.

## Notes
- Large-object space (non-moving big strings/buffers): design it in the Log, but only
  implement it if it is small. Otherwise list it as a follow-up.
- The old-generation / delta snapshot story (ARCH "Deltas") is out of scope, but do not
  make choices that preclude it.
- 0006 (compiler and interpreter) will add cell kinds (function templates, frames,
  closures). Keep the "add a cell kind" path simple and documented, so that the GC is
  automatic for anything declared in `cells.rs`.

## Log
- 2026-11-01: 0008 (compiler design) has not landed yet, so nothing about frames or
  templates to take into account. The GC is driven only by the `Value`/`Ref` field types
  in `cells.rs`, so frames and templates declared there will be collected automatically.
  `cells.rs` now documents how to add a cell kind.
- Implemented `heap/gc.rs` (Cheney: reserve to-space, copy roots to 0, scan with
  `walk::for_each_slot`, forward through `gc_bits::FORWARDED` + first payload word,
  short-circuit flattened ropes), the threshold and `gc_pending` (in `Heap`, outside the
  arena), and `heap/handle.rs` (a `HandleTable` cell at `Roots::HANDLES` with a free list).
  `verify` checks the handle table too. A compile-time assert keeps every cell ≥ 16
  bytes.
- Offset-keyed tables: none. The atom index hashes string contents, property indexes hash
  atom ids, string hashes are content hashes, handles are indexes. Nothing is rehashed.
- Memory cap: the to-space is reserved up front for the full from-space size (an upper
  bound on live data). The arena never exceeds the cap; host memory peaks at about 2× the
  arena during a collection. A reservation failure returns `OutOfMemory` before anything is
  touched (tested with a test-only fault hook).
- Tests (`heap/gc_tests.rs`): 40 random graphs (objects, prototypes, arrays with holes and
  sparse parts, ropes and flattened ropes, symbols, accessors, non-default attributes,
  deletes, dropped handles, cycles) checked with a canonical dump that walks from the
  handles. They also check `verify`, that a second collect produces identical bytes, and
  the `from_bytes` round trip after a GC. Another 20 seeds compare runs with and without
  collections every 37 ops: dumps equal, and the bytes are identical after a final
  collect. Plus focused tests for garbage freed, handles, flattened-rope short-circuit,
  atoms/symbols surviving, the `gc_pending` threshold, failed reservation, and collection
  under a tight cap with retry.
- Numbers (native release, Apple M5 Max,
  `cargo test -p djs-core --release gc_throughput -- --ignored --nocapture`):
  - records (200k objects × 5 properties, half strings): 60 MiB live in 800k cells,
    12.9 ms per collection, about 0.21 ms per MiB;
  - linked list (500k objects with a next pointer and a rope each): 80 MiB live in 1.5M
    cells, 23.4 ms per collection, about 0.29 ms per MiB (about 15 ns per cell).
- Large-object space: not implemented. Design sketch. Copying is already about memcpy
  speed for big strings, so the gain is mainly avoiding the 2× peak and the copy of
  multi-MiB buffers. Proposed: a non-moving *prefix* of the arena, shared with the ARCH
  "Deltas" old generation. At a collection, cells above a size threshold (and, for the old
  generation, the base snapshot) are placed in or kept in the prefix and never move again.
  The young to-space is built in a separate buffer whose offsets start at the prefix end,
  then appended after truncating the arena to the prefix. The roots cell moves into the
  prefix. Without a write barrier the whole prefix is scanned as roots each time; this is
  cheap for strings, which have no traced slots. The prefix is swept with
  `gc_bits::MARK` plus a free list, or only compacted by a full collection (for example
  before a full snapshot). Nothing in the current code precludes this: the to-space start
  offset and the roots location are the only things that change.

## Follow-ups
- Large-object space / non-moving prefix (see the Log sketch), together with the
  old-generation design for delta snapshots.
- Atom GC (still from 0005): atoms and symbols are kept alive by the atom table.
- The interpreter (0006) must call `collect()` only at safepoints when `gc_pending()` is
  set, and keep every live value in arena roots (value stack, frames) or handles.
- Optionally flatten ropes during GC (needs extra to-space, so the reservation bound would
  change), or a debug mode that runs `verify()` after every collection in the stress
  modes.

## Result
- `Heap::collect() -> Result<GcStats, HeapError>`: Cheney copying collection, called at
  safepoints. `Heap::gc_pending()` and `gc_threshold()` expose the trigger. On
  `OutOfMemory` the heap is unchanged.
- Handles: `new_handle(v) -> Handle`, `get_handle`, `set_handle`, `drop_handle`,
  `handle_count`; `Handle::index()` / `from_index` for the embedder. Handles survive
  collections and image round trips.
- `walk::for_each_slot(kind, cell, size, f)`: slot iteration from the layout table
  without borrowing the arena.
- Adding a cell kind: see the `cells.rs` module docs; GC, walker and verify follow
  automatically.
- Known gaps: no large-object space, no atom GC, ropes are not flattened by the GC.
  Collection needs host memory for a second copy of the arena.
