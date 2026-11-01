# 0135: Why are REPL heap dumps ~450 KB and the docs' heaps ~6000 cells?

status: in-progress
agent: bloat
branch: issue/0135-heap-dump-size
depends-on: 0117, 0127
guide: docs/guide/snapshots.md, gc.md, the live example Heap tab text
touches: crates/djs-cli/src/repl/ (.snapshot/.heap), crates/djs-docs/assets/live.js (Heap tab), js/src/ (Sandbox bytes/heap stats if needed), research notes; base-image content changes only as proposals unless small. Coordinate with snapshot (0133, the heap dump analyzer, same area) and heap (0134, heap/)

## Goal
Owner: "why are the heap dumps we get from the REPL ~450 KB large? Also the heap dumps in
the docs show almost 6000 active cells. That's silly."

## Questions
- What exactly does the REPL's `.snapshot` write (raw image? collected? container?) and
  where do its ~450 KB go (base realm vs user data vs garbage vs slack)? Same for the docs'
  Heap tab: what are the ~6000 live cells (builtin functions, prototypes, shapes,
  atoms, templates...) by kind/class?
- Why does a fresh VM have ~6000 live cells, what would QuickJS have after
  JS_NewContext for comparison, and which of them are needed?
- Fixes, in two kinds:
  1. presentation/format: REPL dumps written as compact containers (0117) by default,
     sizes reported as "own data vs shared base", the Heap tab showing the VM's own cells
     separately from the base realm;
  2. substance: fewer cells in the base realm (e.g. lazily materialized builtins/prototypes,
     shared native function cells, atom/shape reductions), with estimates of cells and
     bytes saved and the cost to vm_new and determinism.

## Acceptance criteria
- [ ] A clear answer with numbers (use 0133's analyzer if it has landed, or the REPL's
      `.heap` and the walker).
- [ ] Land the presentation fixes (REPL default, Heap tab) if they're small.
- [ ] Substance changes as a ranked proposal for the coordinator/owner, not built here
      unless trivial.

## Log

## Follow-ups

## Result
