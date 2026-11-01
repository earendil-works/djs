# 0138: Design study: builtin code in a shared read-only segment (no build)

status: open
agent: -
branch: issue/0138-shared-segment-design
depends-on: 0135, 0136
guide: -
touches: docs/shared-segment.md (new design doc), work/scratch/0138/ (prototypes/measurements only)

## Goal
bloat's 0135 proposal 3: 3,723 of the base realm's 5,768 cells (217 KB) are the 874
builtin functions (object + template + bytecode + tables) copied into every VM. Moving
templates, bytecode and constant tables into a shared read-only segment (in the module's
data, or one copy per wasm instance shared by its VMs) would cut ~178 KB and ~2,850 cells
per VM and roughly halve the vm_new copy. Before the owner decides: what would it take?

## Questions
- Ref encoding: how a Ref names a cell in the read-only segment (address range, tag bit),
  what the bounds-checked access path costs (every heap read? only template/bytecode
  reads?), and what that does to wasm instructions on the workloads (prototype the
  access change if cheap, measure).
- What must stay per-VM: function objects (identity, properties), closures' captured
  cells, IC slots inside bytecode (are they in the bytecode cells today? if so they must
  move out), shapes referenced from templates, atoms referenced by bytecode (shared atom
  ids?).
- GC: read-only cells are never marked/moved; pointers from RO to per-VM must not exist.
- Snapshots/determinism: images reference the segment by build hash (already pinned by
  0117); canonical_hash; the base image split into RO + per-VM.
- Interaction with lazy builtins (proposal 4) and pi's prelude (can the sealed prelude's
  templates join a second RO segment?).
- Estimates: per-VM bytes and cells, vm_new time, module size, code size, interpreter
  cost, implementation size and risk, in phases.

## Deliverable
docs/shared-segment.md with a recommendation (do / don't / phased), for the owner.

## Log

## Follow-ups

## Result
