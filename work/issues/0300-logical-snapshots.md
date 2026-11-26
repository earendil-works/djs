# 0300: Heap snapshots revisited (2): a logical snapshot of a waiting run, keyed by bytecode, small (prototype)

status: open
agent: psnap
branch: issue/0300-logical-snapshots
depends-on: -
touches: crates/djs-core/src/vm/snapshot/ (new, behind a non-default cargo feature `snapshot`), crates/djs-core/Cargo.toml (the feature), tests for it, docs/heap-snapshots-revisited.md (your sections 3-5), work/scratch/0300/
guide: -

## Goal
The owner questions continuation dumps (vm/dump*.rs: ~26.5k lines, ~270 KB gz of the
shipped module, JavaScript that re-enacts a run) and wants to know whether a **heap
snapshot** can do the job instead: simpler, small, and surviving djs and pi upgrades in
more situations than "same build". psychoscript (vendor/psychoscript of the primary
checkout; read `src/snapshot.ts`, `src/snapshot/codec.ts`, and the Rust port
`rust/crates/psychoscript/src/snapshot.rs`) does exactly that, build-bound:
- a logical graph: one record per reachable object, fields by name from a per-class spec,
  an unknown field is a codec gap (nothing silently dropped);
- code is never saved: the snapshot carries the sources, restore recompiles them and
  refuses when a source's bytecode hash differs, "so a pc always means the same
  instruction";
- builtins are named (or saved "by position" in a fresh base); only base objects the run
  changed are saved; natives by name;
- host calls in flight by kind (idempotent: run again with the same key; durable: its data).

Hypothesis to test: djs can do the same with **bytecode hashes instead of the build** as
the compat key. A snapshot then survives every djs change that keeps (a) the bytecode the
compiler emits for the run's functions, (b) the meaning of the named fields of the cell
kinds it holds, (c) the builtin/intrinsic names it refers to. Cell layouts, intrinsic ids,
atom ids, the base image, shapes, GC and everything else may change freely. Agent psnapr
(0299) measures in parallel how often (a)-(c) changed in djs's history.

Prior research (deleted from the tree by 0297; read it, it saves you days): `git show
9d001b47:docs/portable-snapshots.md` (0141: a by-meaning graph, frames named by source
site and operand roles; its prototype `scripts/snapshot-research/portable` is in history
too) and `git show 9d001b47:docs/goldilocks-snapshots.md` (0142: tiers, "image with
symbolic relocations"). The difference now: frames travel as pc + registers (no operand
roles, no suspension maps), guarded by the bytecode hash; everything else by name.

## Acceptance criteria
- [ ] **Export** at an idle point (the same moments a plain dump is taken: no frames on the
      stack, empty job queue, only open async host calls), into a self-describing binary
      format: a header (format version, a schema digest per cell kind used, the script
      sources or their hashes, per-function bytecode hashes of the functions the snapshot
      refers to), a string table, and one record per reachable non-base cell.
      - Base objects (builtins, frozen) by **registry path** (vm/registry.rs,
        tests/builtins/registry.tsv); intrinsic ids by intrinsic name; atoms by string;
        host functions and host roots by identity; the global object's own properties the
        run added or changed, by key.
      - Cells by kind name and **named fields** from the layout table (heap/cells.rs's
        `cells!`, heap/layout.rs): a generic codec driven by the table, with per-kind code
        only where a field is not self-describing (atoms inside U32Arrays, shape trees,
        property tables, hash tables of Map/Set, typed array bytes, bytecode templates).
        Shapes are rebuilt from keys, not stored. An unknown or unsupported kind refuses
        the snapshot with a path, like the dumper's refusals.
      - Code: templates are not stored. A closure or a frame refers to (script, function
        index in compile order) plus that function's bytecode hash; a frame holds pc and
        registers as they are.
      - Open host calls: id, identity, args (and the host's state, as dumps have it).
- [ ] **Import** into a fresh VM made from the base: compile the stored sources again, check
      every referenced function's bytecode hash (refuse with a clear reason on mismatch),
      allocate and fill cells, patch references, reopen host calls under their ids, set
      the next call id and Math.random state. The run continues and behaves exactly as the
      uninterrupted one.
- [ ] **Coverage and equivalence** on the harvested codemode corpus, at every idle point the
      plain-dump harvest uses (vm/dump_harvest.rs, tests/corpus/codemode-harvest): how many
      export, how many import and continue with the same output and result as the
      uninterrupted run, what is refused and why. Also chains (export, import, run to the
      next idle point, export again).
- [ ] **Sizes and times** at those idle points: raw binary, zstd -3 (measure on the host
      side; no dependency in djs-core), vs the plain dump text and its zstd (the harvest
      and tests/scores/dump-size.tsv have the method), vs `Vm::image`. Plus the large-state
      cases pi hit with dumps (BOARD: 100 KB of live state cost ~12 ms and ~150 KB per
      step; a 1e6-element array ~1 s and 5 MB): an object of ~100 KB of strings, a 1e6-number
      array, a Map of 10k entries, a 20k-line log string array. Median and p99, native
      release.
- [ ] **Portability experiment**: on a scratch commit, perturb internals a snapshot must not
      depend on, without touching bytecode or named fields' meaning: renumber intrinsics,
      reorder or add base builtins (registry ids), add a field to a cell kind the run uses,
      change a cell kind's field order/offsets, change the base image layout. Show that
      snapshots exported before load after. Then show the refusals for what it must notice:
      a changed bytecode for a suspended function, a removed builtin path, a removed field.
- [ ] **Complexity**: lines of the snapshot module (export + import + codec) and its share of
      a wasm build with the feature (gz), vs the dumper's.
- [ ] Sections 3 (design as built), 4 (results) and 5 (what it would take to ship, and what
      it cannot do) of `docs/heap-snapshots-revisited.md` (0299 writes sections 1-2;
      coordinate through the coordinator). Merge the doc. The prototype code stays on the
      branch unless the coordinator says otherwise.

## Stretch (only once the above is done)
- **Incremental snapshots**: pi saves before every launched call, so a run that keeps 1 MB
  of state pays 1 MB per call. Measure a delta snapshot (records added/changed since the
  previous snapshot of the same run, by record identity) on the large-state cases.
- A wasm export + `Sandbox` method (`run.snapshot()`, `sandbox.loadSnapshot(bytes)`) on the
  branch, so pi's prototype could try it.

## Notes
- Native Rust tests are enough (like vm/dump_harvest.rs); no wasm or JS needed for the
  acceptance criteria.
- Bytecode hash: of the template's code and constants, with atom and intrinsic ids replaced
  by their names (otherwise it changes whenever builtins move). Decide what else belongs in
  it (exception tables, register count) and write down why.
- Frames: find out where djs keeps a suspended async function's activation (heap cells?
  registers, pc, try stack) and how promises' reactions and builtin closures
  (PromiseResolveFunction, Promise.all element records) are represented; those are the
  records that need care.
- Do not stop on scope: cover the cell kinds the corpus actually holds at idle points,
  refuse the rest, and report the gap with counts. If something fundamental blocks the
  design, write that down as a result: a negative answer with evidence is a good outcome.
- Builds niced, `-j 6`, engine runs under scripts/timeout.sh; psnapr (0299) builds historic
  commits on the same machine.

## Log

## Follow-ups

## Result
