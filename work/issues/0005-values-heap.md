# 0005: Values, arena heap, strings, atoms, objects

status: in-progress
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
- [ ] `Value`: a 64-bit NaN-boxed `Copy` type. Holds an f64, or a tagged `undefined`,
      `null`, `bool`, the hole/empty marker, an int32 tag (it may stay unused for now), or
      a reference (string, object, symbol; bigint and host-ref tags are reserved). The
      encoding is documented in the module docs. Canonicalize NaNs on boxing.
- [ ] Arena: growable, a little-endian byte buffer, `u32` offsets, 8-byte alignment,
      bounds-checked reads and writes (no `unsafe`), a configurable memory cap that
      returns an out-of-memory error (never a panic). Offset 0 is reserved as null.
- [ ] **A layout description macro** (ARCH "Hand-rolled heap ergonomics" risk). Each
      cell kind and its fields (name, type, offset) are declared once. The macro
      generates typed accessors and a const layout table that can later be exported for
      the JS-side reader and for the snapshot build hash. Variable-length tails (string
      payloads, property arrays, element arrays) are supported.
- [ ] Cell header: kind, GC bits, size. Every cell kind has a `trace` description (which
      fields or slots hold `Value`s or references), and a heap walker test checks that
      walking the arena cell by cell visits every allocated cell. This is the
      groundwork for the copying GC, which is a separate issue: allocation must not
      collect (ARCH "Generated state machines": GC only at safepoints).
- [ ] Strings: one-byte (Latin-1), two-byte (UTF-16LE) and rope cells. Constructors
      from UTF-8 (transcoding, lone surrogates impossible there), from `&[u16]` and from
      Latin-1. Length in UTF-16 code units, `char_code_at`, equality, an ordering
      comparison (code unit order), hashing (our own fast hash), concatenation that
      makes a rope above a size threshold, and flattening (into a new flat cell, with
      the rope cell redirected or replaced). Iteration over code units without
      flattening is optional.
- [ ] Atoms: an interned string table in the heap (a hash table cell), `atom(str) ->
      Atom`, and a generated list of well-known atoms (`length`, `prototype`,
      `constructor`, `toString`, `valueOf`, `__proto__`, …) with fixed ids. Integer-like
      keys ("array index" atoms) are represented specially, so element access never
      interns.
- [ ] Symbols: a cell with a description. Property keys are atoms or symbols.
- [ ] Objects: a cell with a class/kind, a prototype ref, a property storage ref and an
      elements ref. Properties: a small inline list (key, flags, value or getter/setter
      pair) that switches to a heap hash table above N entries. Flags: writable,
      enumerable, configurable, accessor. Dense elements for arrays with a hole marker,
      a length, and growth.
- [ ] Primitive operations, all leaf (no user code): own property
      get/define/delete/has, ordered own keys (spec order: integer keys, string keys in
      insertion order, symbols), a prototype chain lookup that returns the found slot
      (data value or accessor pair; the caller decides how to call a getter), and element
      get/set fast paths.
- [ ] Roots: a root set in the arena (a header with offsets of the realm root object, the
      value stack area, etc.), even if mostly empty for now. Document the plan.
- [ ] Unit tests for all of the above, including a test that the arena bytes contain no
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

## Follow-ups

## Result
