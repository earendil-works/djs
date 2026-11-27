# 0301: Logical snapshots on main: the module, one slot table, import validation, gate legs

status: open
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
- [ ] The module from `issue/0300-logical-snapshots` on main (rebased; the branch's research
      scripts stay out), built into the shipped module (`snapshot` in its features; the size
      ceiling is raised by what it costs, recorded, and comes back down in 0304).
- [ ] **One slot table**: object classes' internal slots defined once in heap/object.rs
      (the `classes!` table from snapshot/schema.rs moved there), used by allocation, verify,
      the snapshot and the generated JS layout; the prose slot comments become that table.
- [ ] **Import validation**: an imported heap is checked before anything runs: frames against
      their code (pc on an instruction, registers within the frame, handler state), records
      against their kinds (field types, tail lengths, references in range), the result run
      through the heap verifier where it exists. Malformed bytes refuse, never trap or
      corrupt (fuzz the importer briefly: truncations, bit flips).
- [ ] **Gate**: a snapshot leg in the difftest/harvest (export, import and continue at every
      idle point of the corpus, chains included), the negative control (a broken importer
      must be caught), and a **pin of the normalized bytecode of every corpus function that
      can hold a frame** (a committed list; a change shows in review and is regenerated
      deliberately, like the registry). Not a compatibility promise: a visible decision.
- [ ] Decisions recorded (format, key, what refuses, the trust model: snapshot bytes are
      trusted host input, validated for robustness).

## Notes
- Keep dumps working until 0304; both may coexist on main briefly.
- 0299's follow-ups (the bytecode pin, a machine-readable slot table) are this issue.

## Log

## Follow-ups

## Result
