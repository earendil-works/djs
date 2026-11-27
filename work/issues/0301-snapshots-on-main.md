# 0301: Logical snapshots on main: the module, one slot table, import validation, gate legs

status: done
agent: psnap
branch: issue/0301-snapshots-on-main
depends-on: 0300
touches: crates/djs-core/src/vm/snapshot/, crates/djs-core/src/heap/object.rs (slot table), crates/djs-core/Cargo.toml, crates/djs-wasm (exports), js/src/abi.ts, crates/djs-difftest, scripts/gate.py, tests/snapshot/ (new), TESTING.md, docs/decisions.md
guide: -

## Goal
The owner adopted logical snapshots (docs/heap-snapshots-revisited.md, its summary and the
review in work/issues/0300's log) as djs's persistence format; continuation dumps go in 0304.
**Backwards compatibility with earlier releases is not required** (owner, 2026-11-24): no
reader for dumps or earlier snapshot formats, no migration, no golden files frozen across
releases; D024 applies to the snapshot format too. This issue lands the 0300 prototype on
main in a form that can be relied on within djs's development.

## Acceptance criteria
- [x] The module from `issue/0300-logical-snapshots` on main (rebased; the branch's research
      scripts stay out), built into the shipped module (`snapshot` in its features; the size
      ceiling is raised by what it costs, recorded, and comes back down in 0304).
- [x] **One slot table**: object classes' internal slots defined once in heap/object.rs
      (the `classes!` table from snapshot/schema.rs moved there), used by allocation, verify,
      the snapshot and the generated JS layout; the prose slot comments become that table.
- [x] **Import validation**: an imported heap is checked before anything runs: frames against
      their code (pc on an instruction, registers within the frame, handler state), records
      against their kinds (field types, tail lengths, references in range), the result run
      through the heap verifier where it exists. Malformed bytes refuse, never trap or
      corrupt (fuzz the importer briefly: truncations, bit flips).
- [x] **Gate**: a snapshot leg in the difftest/harvest (export, import and continue at every
      idle point of the corpus, chains included), the negative control (a broken importer
      must be caught), and a **pin of the normalized bytecode of every corpus function that
      can hold a frame** (a committed list; a change shows in review and is regenerated
      deliberately, like the registry). Not a compatibility promise: a visible decision.
- [x] Decisions recorded (format, key, what refuses, the trust model: snapshot bytes are
      trusted host input, validated for robustness).

## Notes
- Keep dumps working until 0304; both may coexist on main briefly.
- 0299's follow-ups (the bytecode pin, a machine-readable slot table) are this issue.

## Log
- The 0300 module applied as it was (without scripts/research), then: `snapshot` in
  djs-core's defaults, djs-wasm's shipped features (build-wasm.sh, gate's build:size and
  wasm clippy, the js tests' module).
- One slot table (D3701): `object_classes!` in heap/object.rs declares each class with its
  id and named slots (`CLASSES`, `SlotKind`); enum, `from_u32`, `slots()`, `slot_table()`,
  `slot(name)` (const). schema.rs's own table went; the snapshot reads `CLASSES` (WeakRef and
  FinalizationRegistry still refused). Slot constants of promise.rs, suspend.rs,
  collection.rs, weakref.rs and regexp.rs come from it; layout.generated.ts has
  `objectSlots`. Verify's slot-count check uses `slots()` as before.
- Import validation (D3702), snapshot/check.rs: per-class slot checks, frames against
  code (pc on a reachable instruction via `stack_depths`, hp/locals/stack within the frame,
  saved operands = the resume point's depth - 0..2, header ints, no native frame), closure
  captures vs template, header flags only where meaningful, host call and handle tables,
  f64s never tagged, duplicate keys, the snapshot not setting skipped/count fields, Map
  buckets a power of two with room, then `Heap::verify`. Import median 189 -> 261 us.
- Fuzz (tests.rs): truncations and bit flips; the found panics were fixed (a function
  index, a Map index too small, a Symbol's atom set from the snapshot, lazy-name flags on
  objects without a template, a patch's flags, flags set before properties). 100,000
  mutations: 1 panic left, in a Promise.all element record's slot count (builtin records
  are positional: 0302). The gate runs 400.
- Gate: leg "snapshots" (harvest + programs, alone and chained, asserting no refusal or
  difference; 5.5 s in the gate's dev build), `negative_control_catches_a_broken_import`
  and `frame_bytecode_pin` (tests/snapshot/frame-bytecode.txt, 711 functions of the 331
  corpus scripts) in `test`. TESTING.md "Logical snapshots".
- Size (D3705): 698,440 -> 769,623 B gzip; ceiling 770,647.

## Follow-ups

## Result
Logical snapshots are on main and in the shipped module (`Vm::snapshot[_with]`,
`Vm::from_snapshot[_with]`, `BaseInfo`, `describe`; djs-wasm `vm_snapshot`,
`vm_from_snapshot`), with one slot table in heap/object.rs, import validation, the gate leg,
the negative control and the frame bytecode pin. Decisions D3700-D3705. Known gap: builtin
records' slot counts are unchecked until 0302.
