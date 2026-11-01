# 0009: Copying GC and handle table

status: in-progress
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
- [ ] `gc_pending` is set by allocation once the heap passes a threshold (growth policy:
      for example, next threshold = live × factor, with a minimum). `collect()` is called
      explicitly, at a safepoint.
- [ ] Cheney copy into a fresh arena: roots cell first, then a breadth-first scan using
      the layout trace descriptions, including variable-length tails (property arrays,
      elements, rope children, hash tables) and every `Value` slot. Forwarding uses the
      header GC bits.
- [ ] Atom table and symbols: atoms stay alive (atom GC is a follow-up), but the strings
      behind them are moved correctly. Rehash wherever a table is keyed by offsets (it
      should not be: check and document).
- [ ] Rope flattening during GC is optional. If you do it, document it.
- [ ] Handle table in the arena (a roots slot): `new_handle(value) -> Handle(u32)`,
      `get`, `set`, `drop_handle`, with a free list. Handles are the only way Rust code
      keeps a reference across `collect()`.
- [ ] Memory cap: the collection itself fits in the cap's accounting (to-space
      allocation), or returns `OutOfMemory` cleanly with the old heap intact.
- [ ] Tests: random object graphs (properties, arrays, ropes, symbols, cycles) built
      through handles, then repeated `collect()` calls. Check them with a structural
      comparison (a deep compare function walking from the handles) and `verify()`.
      Garbage is actually freed (arena size shrinks). The image round trip (`from_bytes`)
      still works after a GC. Collecting twice is idempotent in layout, so the bytes are
      identical: a deterministic order matters for snapshots.
- [ ] Rough numbers: collection time per MB live (native release), recorded in the Log.

## Notes
- Large-object space (non-moving big strings/buffers): design it in the Log, but only
  implement it if it is small. Otherwise list it as a follow-up.
- The old-generation / delta snapshot story (ARCH "Deltas") is out of scope, but do not
  make choices that preclude it.
- 0006 (compiler and interpreter) will add cell kinds (function templates, frames,
  closures). Keep the "add a cell kind" path simple and documented, so that the GC is
  automatic for anything declared in `cells.rs`.

## Log

## Follow-ups

## Result
