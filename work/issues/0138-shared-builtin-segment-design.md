# 0138: Design study: builtin code in a shared read-only segment (no build)

status: done
agent: heap
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
- Measured the split on the base: 2,849 template-owned cells, 177,616 B (templates,
  bytecode, capture descriptors, constant pools, handler tables). The pools reference
  367 strings (13 KB) and nothing else; privileged templates have no children.
- Prototyped the access dispatch (work/scratch/0138, not landed):
  - eager, every code/template/constant read checks `RO_BASE`: loop +2.4%, calls +1.4%,
    arrays +1.3%, props +1.0%, pi-like +0.3%;
  - lazy copy, frame entry and caches only: loop +0.5%, call1 +0.9%, the rest ≤ 0.6%.
- Wrote docs/shared-segment.md:
  - encoding, the per-VM parts (caches out of templates, a pinned string prefix), GC,
    snapshots, determinism, lazy builtins, pi;
  - estimates and phases (6–7 weeks, medium-high risk);
  - a cheaper alternative: decode the base straight into the arena, −0.31 MiB per
    instance.

## Follow-ups
- Candidate issue: decode/inject the base directly into the first VM's arena instead of
  keeping a decoded copy per instance (docs/shared-segment.md §8): est. −0.31 MiB wasm
  memory per instance, djs-wasm + js/src/snapshot.ts.

## Result
docs/shared-segment.md. Recommendation: don't build now. For pi it saves 0.18–0.43 MiB
per instance and costs +1–2.4% instructions (eager) or a lazy-copy mechanism, plus an
image-format change and an audit of every template read. Do §8's base-into-arena change
first. Revisit with several VMs per instance; if built, the lazy-copy variant in four
phases.
