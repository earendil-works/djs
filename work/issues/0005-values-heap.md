# 0005: Values, arena heap, strings, atoms, objects

status: done
agent: heap
branch: issue/0005-values-heap
depends-on: 0001
touches: crates/djs-core/src/value.rs, crates/djs-core/src/heap/, crates/djs-core/src/lib.rs (mod lines only)

## Goal
The memory core described in ARCH.md ("The Core Idea", "Values", "Heap"): a single
position-independent byte arena addressed by `u32` offsets that holds all VM state, plus
the value representation and the first cell kinds. The compiler and interpreter (0006)
and the GC (later) build on this, so the layout API matters more than feature
completeness.

## Acceptance criteria
- [x] `Value`: a 64-bit NaN-boxed `Copy` type. Holds an f64, or a tagged `undefined`,
      `null`, `bool`, the hole/empty marker, an int32 tag (it may stay unused for now), or
      a reference (string, object, symbol; bigint and host-ref tags are reserved). The
      encoding is documented in the module docs. Canonicalize NaNs on boxing.
- [x] Arena: growable, a little-endian byte buffer, `u32` offsets, 8-byte alignment,
      bounds-checked reads and writes (no `unsafe`), a configurable memory cap that
      returns an out-of-memory error (never a panic). Offset 0 is reserved as null.
- [x] **A layout description macro** (ARCH "Hand-rolled heap ergonomics" risk). Each
      cell kind and its fields (name, type, offset) are declared once. The macro
      generates typed accessors and a const layout table that can later be exported for
      the JS-side reader and for the snapshot build hash. Variable-length tails (string
      payloads, property arrays, element arrays) are supported.
- [x] Cell header: kind, GC bits, size. Every cell kind has a `trace` description (which
      fields or slots hold `Value`s or references), and a heap walker test checks that
      walking the arena cell by cell visits every allocated cell. This is the
      groundwork for the copying GC, which is a separate issue: allocation must not
      collect (ARCH "Generated state machines": GC only at safepoints).
- [x] Strings: one-byte (Latin-1), two-byte (UTF-16LE) and rope cells. Constructors
      from UTF-8 (transcoding, lone surrogates impossible there), from `&[u16]` and from
      Latin-1. Length in UTF-16 code units, `char_code_at`, equality, an ordering
      comparison (code unit order), hashing (our own fast hash), concatenation that
      makes a rope above a size threshold, and flattening (into a new flat cell, with
      the rope cell redirected or replaced). Iteration over code units without
      flattening is optional.
- [x] Atoms: an interned string table in the heap (a hash table cell), `atom(str) ->
      Atom`, and a generated list of well-known atoms (`length`, `prototype`,
      `constructor`, `toString`, `valueOf`, `__proto__`, …) with fixed ids. Integer-like
      keys ("array index" atoms) are represented specially, so element access never
      interns.
- [x] Symbols: a cell with a description. Property keys are atoms or symbols.
- [x] Objects: a cell with a class/kind, a prototype ref, a property storage ref and an
      elements ref. Properties: a small inline list (key, flags, value or getter/setter
      pair) that switches to a heap hash table above N entries. Flags: writable,
      enumerable, configurable, accessor. Dense elements for arrays with a hole marker,
      a length, and growth.
- [x] Primitive operations, all leaf (no user code): own property
      get/define/delete/has, ordered own keys (spec order: integer keys, string keys in
      insertion order, symbols), a prototype chain lookup that returns the found slot
      (data value or accessor pair; the caller decides how to call a getter), and element
      get/set fast paths.
- [x] Roots: a root set in the arena (a header with offsets of the realm root object, the
      value stack area, etc.), even if mostly empty for now. Document the plan.
- [x] Unit tests for all of the above, including a test that the arena bytes contain no
      host pointers (for example: build a heap, copy the bytes into a fresh arena, and
      check that everything still reads back the same). This is the cheap precursor of a
      snapshot.

## Notes
- Read ARCH.md fully. You are setting conventions that the GC, the interpreter, the
  snapshot code and the JS host reader all depend on. Record the important ones in
  `docs/decisions.md` (value encoding, cell header format, string representation,
  atom representation).
- Rust structs may be used for transient views (`StrView<'a>`, iterators), but persistent
  state lives in the arena only.
- Do not build the interpreter, the compiler or the GC here. Keep the API small and
  direct; 0006 will extend it as needed.
- Performance matters but correctness and a clean layout come first. Avoid per-access
  overhead that cannot be removed later (for example, make sure bounds checks can be
  hoisted for bulk copies).

## Log
- 2026-11-01: Read ARCH/TESTING/decisions. Design: NaN boxing with tag in the high word
  (`0xFFF8_0000 | tag`); 8-byte cell header (kind, gc, flags, size); Roots cell at offset 0
  (so 0 = null and the Cheney scan starts at the roots); `cells!`/`records!` macros with
  explicit offsets and compile-time validation; atoms as stable `u32` ids (int bit 31,
  symbol bit 30, table index otherwise), with symbols as atoms so every key is an `Atom`.
- Implemented `value.rs` and `heap/{arena,layout,cells,arrays,string,atom,object,walk}.rs`
  plus `heap/tests.rs`. Strings: Latin-1/UTF-16/rope with a narrowing invariant, iterative
  rope walks (100k-deep ropes tested both ways), flatten-and-redirect. Objects: linear table
  up to 8 entries, then a hash index over insertion-ordered entries (tombstones, compaction
  on growth); dense array elements with holes; full ValidateAndApplyPropertyDescriptor and
  ArraySetLength (non-configurable elements stop truncation, read-only length).
- Hardening: `Heap::from_bytes` verifies the image (structure, every traced slot, kind
  invariants, rope length monotonicity, prototype acyclicity). Hash probes are bounded, so
  even a corrupted heap that slips through cannot hang a leaf op. A random-corruption test
  checks that `verify` never panics.
- Tests: 48 unit tests (value encoding, arena, layouts, strings, atoms, symbols, objects,
  arrays, walker/tracing, image round trip, byte-identical images from two heaps, image
  validation, OOM at several caps leaving a verifiable heap). Debug and release both pass;
  wasm32 build clean.
- Decisions D1050-D1056 recorded.

## Follow-ups
- Copying GC (Cheney from the Roots cell at 0, `FORWARDED` gc bit, short-circuit flattened
  ropes, large-object space for big strings).
- Atom GC: atoms (and with them symbols) are never freed. Needed for long-lived sessions with
  many dynamic keys. Also the `Symbol.for` registry.
- Generate the JS-side heap reader from `cells::LAYOUTS` (a small export tool that dumps the
  table as JSON/TS).
- Typed `Ref` fields in the layout macro (the target kinds), so the kind checks in
  `walk::cell_invariants` can be generated instead of hand-written.
- `char_code_at` on an unflattened rope is O(depth); the interpreter should flatten before
  indexed loops (or flatten on first indexed access, as V8 does).
- Roots fields `STACK`, `JOBS`, `HOST_CALLS`, `HANDLES` are reserved but null; their cells
  come with 0006 and later issues.
- More `ObjectClass`es (Function, Error, wrappers, Arguments, ...) in 0006. Classes with
  exotic index behavior must set `OBJ_INDEXED_PROPS` (see `object.rs` docs).
- Maybe later: dense elements for non-array objects, shapes, a faster non-ASCII hash.

## Result
Landed in `crates/djs-core/src/value.rs` and `crates/djs-core/src/heap/`:

- `value::Value`: NaN-boxed `Copy` value (D1050), with `Str`/`Obj`/`Sym` handle types.
- `heap::Heap`: the arena heap. `Heap::new()` / `with_limit(bytes)` create the roots cell,
  the atom table, the well-known atoms and the well-known symbols. `bytes()` is a complete
  position-independent image; `from_bytes(vec, limit)` restores and verifies one.
- Layout: `heap::cells` declares every cell kind once (`cells!`). Use the field handles with
  `arena().get(cell, Kind::FIELD)`. `cells::LAYOUTS` is the exportable table and
  `cells::LAYOUT_HASH` the build hash.
- Strings: `alloc_str` (UTF-8), `alloc_utf16`, `alloc_latin1`, `str_len`, `char_code_at`,
  `str_eq`, `str_cmp`, `str_hash`, `concat`, `flatten`, `str_view`, `str_chunks`,
  `str_units`, `str_to_string`.
- Atoms: `atom_str(&str)`, `intern(Str)`, `atom_from_index(u32)`, `Atom::int(i)`,
  `atom_array_index`, `atom_string`, `atom_to_value`, `atoms::*` (well-known, including
  `atoms::SYM_*` symbols). Symbols: `new_symbol`, `symbol_atom`, `atom_symbol`,
  `symbol_description`.
- Objects: `new_object`, `new_array`, `alloc_object(class, proto, slots)`,
  `get_own_property`, `has_own_property`, `find_property` (proto chain; returns holder +
  `Property`), `has_property`, `define_own_property(&PropertyDescriptor)` (spec validation,
  Array exotic `length`/index behavior), `create_data_property`, `delete_own_property`,
  `own_keys` (spec order), `get_prototype_of`/`set_prototype_of`, `prevent_extensions`,
  `get_element_fast`/`set_element_fast`, `array_length`, `same_value`.
- Walking: `cells()`, `trace_cell`, `cell_refs`, `verify`.

Known gaps: no GC (allocation only bumps; garbage stays until the GC issue). Atoms are
never freed. Roots for the stack/jobs/host calls/handles are placeholders.
`define_own_property` on an array's `length` expects the caller to have done
ToNumber/ToUint32 already.
