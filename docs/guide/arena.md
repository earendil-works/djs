# One arena: all state in a byte array

Most engines spread a running program over the process: objects on a garbage-collected
heap, frames on the native stack, bytecode and job queues in the engine's own data
structures, all held together by pointers. djs keeps all of it in one place, a
contiguous array of bytes called the **arena**. This page shows what that array holds,
how memory in it is handed out, and why the rest of the design depends on it.

> **In short.** Everything a VM knows lives in one growable `Vec<u8>` addressed by `u32`
> offsets: objects, strings, bytecode, the value stack, frames, the job queue, pending
> host calls. Memory is handed out by bumping an offset, and allocation never collects.
> Because there are no pointers in the bytes, copying the arena is a snapshot, the host
> can read it, and a bad offset traps instead of corrupting memory.

## Everything, in one place

The list of what lives in the arena is the list of what a suspended VM needs to resume:

- objects, strings, symbols, BigInts, shapes and property tables;
- compiled code: function templates, bytecode, constant pools, line tables;
- the **value stack** with every call frame, and the interpreter's registers, flushed
  into a cell whenever `run` returns (D1151);
- the promise job queue, the table of pending host calls and the inbox of their results
  (D1670);
- atoms (interned property keys), the realm with its intrinsics, and the handle table
  through which the embedder holds values (D1091).

Rust data structures (`Vec`, `Box`) hold only transient state: the compiler's working
state while it compiles, and scratch space inside one native operation. Neither survives
past the point where the VM could suspend. Native code is never referenced by pointer
either: a native builtin is a `u16` index into a static table and a host function is a
`u32` id that the host assigned.

The arena starts with the **`Roots`** cell at offset 0. Everything live is reachable
from it:

<!-- gen:cell Roots -->
![The Roots cell](gen/cell-Roots.svg)

<!-- /gen -->

A few fields are not references but small pieces of VM state that must survive a
snapshot: the GC threshold (D1601), the inline cache epoch (D1621), the shape budget
(D1831). `COMPAT`, `OPS`, `INTRINSICS`, `RECORDS` and `FEATURES` are, with `VERSION`,
the image's dependency descriptor: hashes of the tables whose numbers the image holds,
so an image never loads into an engine that would read its bytecode or its records
differently ([snapshots](snapshots.md#what-an-image-depends-on), D2265).

## Cells, back to back

The arena is a sequence of **cells**. Each starts at an 8-aligned offset with an 8-byte
header, and the cells sit back to back with no gaps:

![The arena: cells back to back from offset 0, the zeroed frontier, the GC threshold and the memory cap](img/arena.svg)

The header's `size` is the size of the whole cell, so `offset + size` is the next cell,
and anyone holding the bytes can walk the heap from 0 without knowing any cell kind. The
garbage collector, the image verifier (D1056) and the host all do. On the docs site, the
*Heap* tab of a live example walks the arena of the wasm VM in your browser this way, in
a short loop of JavaScript, and counts the cells by kind. It separates what the example
allocated from the base realm before it: the roughly 5,800 cells of builtins, their
code and names that every VM starts with as a copy of the base snapshot (D2340):

```js live
const rows = [];
for (let i = 0; i < 1000; i++) rows.push({ id: i, name: "row " + i });
let text = "";
for (const r of rows) text += r.name + "\n";
console.log(rows.length, "rows,", text.length, "characters");
```
<!-- gen:run -->
```text
1000 rows, 7890 characters
```
<!-- /gen -->

Those base cells come in two parts. The first, the **builtin prefix**, holds the builtin
objects, their code and their names. Nothing a script does through the builtins writes
there, no collection moves or frees it, and every VM of a build starts with the same bytes
in it (D2063, D2064). So a snapshot can leave it out
([Snapshots](snapshots.md#compact-snapshots)). After it comes a short **tail** of the
cells every VM writes: the realm record, the global object, the atoms the VM adds (the
base's own atoms stay in the prefix), the value stack.

Offset 0 doubles as "null", because nothing ever refers to the roots cell. The all-zero
bit pattern is also a valid "nothing" for every traced field: as a `Value` it is the
number `+0`, and as a `Ref` it is null. Freshly zeroed memory is therefore always safe
for the collector to scan, which is what lets allocation hand out zeroed bytes and fill
them in later.

What the cell kinds are and what their fields mean comes from one declaration, the
`cells!` macro in `heap/cells.rs` (D1052). The next chapter, and the [cell layouts
reference](ref-cells.md), go through them.

## Allocation is a bump

Allocating a cell rounds the size up to 8, checks it against the limit, moves the end of
the used region (`len`) and writes the header:

```rust
pub fn alloc_cell(&mut self, kind: CellKind, size: u32) -> Result<u32, HeapError> {
    let size = align8(size.max(kind.layout().min_size));
    let off = self.arena.alloc(size)?;
    if self.arena.len() >= self.gc_threshold {
        self.gc_pending = true;
    }
    self.arena.set(off, Header::KIND, kind as u8);
    self.arena.set(off, Header::SIZE, size);
    Ok(off)
}
```

The arena keeps a **zeroed frontier** up to 16 KiB beyond `len`, so most allocations are
a compare and an add and never touch the `Vec` (D1261). Data that arrives with the
allocation (a string's bytes, a cell the collector copies) is appended at the frontier
instead of zeroed first and copied afterwards, so it is written once.

The important line is the one that is missing: **allocation never collects.** Crossing
the GC threshold only sets `gc_pending`. The collection runs at the next **safepoint**:
an instruction boundary, a frame entry, a backward jump, a state transition of a native
builtin. Between safepoints, Rust code can keep raw offsets in local variables without
rooting them, because nothing moves them. This is what keeps the native builtins and the
interpreter's slow paths simple; [the garbage collector](gc.md) has the rest of the
story.

> **Trade-off.** Collect only at safepoints, not on allocation failure. A Rust function
> that allocates twice would otherwise have to root the result of the first allocation
> before making the second, at every one of hundreds of call sites in the heap code. The
> cost: the arena can overshoot the threshold between two safepoints, and an operation
> that hits the memory cap cannot simply collect and continue. It fails, and the
> interpreter collects and runs the whole instruction once more (D1700, D1900).

## Growing, and the cap

The arena grows like a `Vec`, by reallocating, with two twists:

- **It stops at the GC threshold.** wasm linear memory never shrinks, so the peak is
  what a run costs. Below the threshold, growth doubles but never reserves past the
  threshold; past it, a collection is pending and growth is by a quarter. Thresholds are
  whole 64 KiB wasm pages (D1870).
- **It starts small.** A VM made from the base or restored from an image reserves the
  image plus one 64 KiB page, not room up to its first collection threshold: most
  sandboxes never need more, and the rest grow in place (D2541). Only a host that says
  how big the heap will get (pi's cold start, which runs its prelude next) gets that
  room up front.
- **In wasm it grows in place.** The module's allocator keeps big blocks, the arena
  among them, as runs of whole pages. When the pages after the arena are free or memory
  ends there, a growth adds pages instead of copying the arena into a new block, which
  would hold the old and the new copy at once (D2053). The allocator works to keep the
  arena at the end of memory: the general-purpose allocator's first pages come before
  it, mid-sized blocks the general allocator cannot hold get pages of their own that go
  back when freed instead of staying behind the arena for good, and a buffer that grows
  right behind the arena leaves room for the arena's next growth (D2541).
- **It has a cap.** The embedder sets a memory limit. The arena may grow to the limit
  minus a reserve of `min(cap / 32, 1 MiB)`. Reaching that point is the catchable
  `InternalError: out of memory`, a preallocated object, as in QuickJS. Throwing it
  opens the reserve, so the script's handler has room to run, and collects when the
  handler is reached (D1600).

When the *host* refuses memory below the cap (the `Vec` cannot grow), that is not the
program's doing, so it is not a JavaScript error: the run stops with
`Status::HostOutOfMemory` and the VM stays valid and idle (D1690). Keeping the two apart
keeps the out-of-memory point a function of the program and the cap, which
[deterministic runs](determinism.md) rely on.

Because the arena can be reallocated, nothing outside it may keep a pointer into it
across a VM call. The host's reader re-derives its view of wasm memory after every call,
which wasm hosts do anyway, since linear memory itself can grow.

## Offsets, not pointers

Every reference in the arena is a `u32` offset from its start: in a [value](values.md)
the payload, in a `Ref` field the whole word. This one rule buys four things:

1. **Snapshots are a copy.** After a compacting collection, the arena bytes *are* the
   VM. There is no serializer to write or keep in sync with the object model, and a
   restore is a copy, a check (D1056) and nothing else: no fixups, because there is
   nothing to fix up.
2. **The host can read values in place.** A tool result that the script turned into an
   array of objects is read by walking cells in wasm memory, with a reader generated
   from the same layout table (D1210).
3. **Bugs trap instead of corrupting memory.** Every access is bounds-checked, with no
   `unsafe`. A bad offset from a VM bug or a corrupt image is a panic (a wasm trap) or a
   corrupt JS heap, never undefined behavior in the host.
4. **The heap can move.** The collector compacts the arena by sliding live cells down,
   and the arena can be reallocated, without any reference outside it going stale: the embedder
   holds values through handles (D1091), slot numbers in a table inside the arena, never
   through offsets.

> **Trade-off.** One arena of offsets over Rust-managed objects. The model for djs's
> execution side, piccolo, keeps JavaScript objects as Rust allocations behind `Gc`
> pointers (`gc-arena`), with the borrow checker guarding them. That gives type safety
> for free, but the host cannot read such a heap and a snapshot needs a full graph
> serializer: the goals of [cheap data passing](goals.md#cheap-data-passing) and
> [snapshots](goals.md#snapshots) again. djs gives up typed references: a cell offset is
> a number, and code that reads the wrong field of the wrong cell is a logic bug the
> verifier and the stress tests have to catch, not the compiler.

> **Trade-off.** Bounds checks on every access. In wasm, every load is already checked
> against linear memory, so the arena's checks are a second check (D016). Each costs one
> compare: `offset + size <= length`, done in 64-bit arithmetic so it cannot wrap, which
> lets the compiler drop the slice's own checks behind it (D2520). Release builds keep
> the checks but drop the offsets from the panic message, which saved code size at the
> about 1,600 inlined access sites (D1573). An optional build (the `unsafe-heap` cargo
> feature, off by default) skips them: about 7 KB of gzipped module and up to 15% of the
> instructions. It is safe only for images the engine made itself, because the verifier
> guarantees structure but not every semantic invariant that the checked build would
> otherwise catch with a trap. All of its `unsafe` sits in one small module, and the merge
> checks run it next to the default build.

> **Trade-off.** `u32` offsets limit an arena to 4 GiB. That is the whole wasm32 address
> space anyway, and a codemode VM is capped far below it. If larger arenas ever matter,
> offsets could be scaled by the 8-byte alignment.

## Where to look

- [`crates/djs-core/src/heap/arena.rs`](../../crates/djs-core/src/heap/arena.rs): the
  byte buffer, the zeroed frontier, the cap.
- [`crates/djs-core/src/heap/layout/access.rs`](../../crates/djs-core/src/heap/layout/access.rs):
  the one place every field read and write goes through, checked or (with
  `unsafe-heap`) not.
- [`crates/djs-core/src/heap/mod.rs`](../../crates/djs-core/src/heap/mod.rs): `Heap`,
  allocation, the conventions every heap function follows.
- [`crates/djs-core/src/heap/cells.rs`](../../crates/djs-core/src/heap/cells.rs): the
  header and every cell kind.
- [`crates/djs-core/src/heap/walk.rs`](../../crates/djs-core/src/heap/walk.rs): the cell
  walker and the verifier.

Decisions on this page:

<!-- gen:decisions D2063 D2064 D2053 D1051 D1052 D1261 D1151 D1091 D1870 D1600 D1690 D1700 D1056 D1573 D2520 D2265 D2541 -->
- D2063: The base image ends with a tail of per-VM cells; nothing writes below BASE_END
- D2064: Collections never move or free the builtin prefix
- D2053: A page allocator under dlmalloc in djs-wasm: huge blocks grow in place
- D1051: Arena and cell header
- D1052: Layout description macro
- D1261: The arena keeps a zeroed frontier; data is written once
- D1151: Interpreter state in the arena
- D1091: Handle table
- D1870: Arena growth stops at the GC threshold; thresholds in 64 KiB pages; no to-space reuse
- D1600: Out-of-memory recovery: a collection at the handler and a reserve below the cap
- D1690: Running out of host memory is not a JS out-of-memory error
- D1700: A failing allocation collects and retries its instruction once
- D1056: Heap images are verified on load
- D1573: Arena out-of-bounds panics carry no offsets in release builds
- D2520: Arena access: one u64 compare by default, an optional unchecked `unsafe-heap` build
- D2265: Images carry a dependency descriptor; loading and containers check it, not the build hash
- D2541: Instance memory: the arena kept at memory's end, lean start capacity, the coded base outside memory
<!-- /gen -->
