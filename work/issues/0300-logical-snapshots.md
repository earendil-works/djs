# 0300: Heap snapshots revisited (2): a logical snapshot of a waiting run, keyed by bytecode, small (prototype)

status: done
agent: psnap
branch: issue/0300-logical-snapshots (the prototype); issue/0300-logical-snapshots-doc (merged: the doc and this file)
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
- [x] **Export** at an idle point (the same moments a plain dump is taken: no frames on the
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
- [x] **Import** into a fresh VM made from the base: compile the stored sources again, check
      every referenced function's bytecode hash (refuse with a clear reason on mismatch),
      allocate and fill cells, patch references, reopen host calls under their ids, set
      the next call id and Math.random state. The run continues and behaves exactly as the
      uninterrupted one.
- [x] **Coverage and equivalence** on the harvested codemode corpus, at every idle point the
      plain-dump harvest uses (vm/dump_harvest.rs, tests/corpus/codemode-harvest): how many
      export, how many import and continue with the same output and result as the
      uninterrupted run, what is refused and why. Also chains (export, import, run to the
      next idle point, export again).
- [x] **Sizes and times** at those idle points: raw binary, zstd -3 (measure on the host
      side; no dependency in djs-core), vs the plain dump text and its zstd (the harvest
      and tests/scores/dump-size.tsv have the method), vs `Vm::image`. Plus the large-state
      cases pi hit with dumps (BOARD: 100 KB of live state cost ~12 ms and ~150 KB per
      step; a 1e6-element array ~1 s and 5 MB): an object of ~100 KB of strings, a 1e6-number
      array, a Map of 10k entries, a 20k-line log string array. Median and p99, native
      release.
- [x] **Portability experiment**: on a scratch commit, perturb internals a snapshot must not
      depend on, without touching bytecode or named fields' meaning: renumber intrinsics,
      reorder or add base builtins (registry ids), add a field to a cell kind the run uses,
      change a cell kind's field order/offsets, change the base image layout. Show that
      snapshots exported before load after. Then show the refusals for what it must notice:
      a changed bytecode for a suspended function, a removed builtin path, a removed field.
- [x] **Complexity**: lines of the snapshot module (export + import + codec) and its share of
      a wasm build with the feature (gz), vs the dumper's.
- [x] Sections 3 (design as built), 4 (results) and 5 (what it would take to ship, and what
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
- Read 0141/0142 (git show 9d001b47:docs/portable-snapshots.md, goldilocks-snapshots.md),
  psychoscript's snapshot.ts/codec.ts, the dumper's harvest (vm/dump_harvest.rs).
- Built `crates/djs-core/src/vm/snapshot/` behind feature `snapshot` (= intrinsic-names;
  excluded from the build hash like `dump`): wire.rs (varints), schema.rs (object classes
  with named slots, generic kinds, flag names), hash.rs (bytecode hash), export.rs,
  import.rs, tests.rs. Outside the module, minimal: `#[cfg(feature = "snapshot")]` accessors
  in heap/object.rs (raw property slots, prop_insert, header flags) and heap/atom.rs
  (a symbol's atom), `CompileOptions::from_bits` cfg, build.rs feature exclusion.
- Functions are named by (script, source span) + hash, not by compile-order index: the span
  is what Function.prototype.toString shows, and a span shared by two templates is told
  apart by the hash.
- Per-VM objects of a fresh VM (global object, global lexical record, Symbol.for registry)
  are patches; the realm's per-VM values (random state, clock, epoch...) are named fields
  of the `Vm` record; the prelude function table is per VM but constant (the importer's).
- First full harvest (dev build): 300 cases, 288 with idle points, 1164 idle points; all
  1164 export, import and continue with the uninterrupted transcript; chains (a snapshot
  at every idle point) 288/288. Negative control (import drops elements): many diffs, so
  the comparison is sensitive.

- Second corpus: the dump test programs (tests/dump/*.src.js, golden/*.src.js: generators,
  async generators, finally, destructuring, classes with private names): 197 idle points,
  197 continue the same, chains 45/45; nothing refused (plain dumps refuse class-fields@1).
- Format tuning: flag sets written once; references as "next new" (0) or absolute id+1, so
  BFS graphs compress (map-10k zstd 88 KB -> 26 KB, 100 KB object 9.5 -> 4.3 KB).
  `SnapshotOptions { sources: false }` stores source hashes, the importer gets the sources.
- Sizes on the harvest (1164 idle points, both including the corpus prelude's state):
  snapshot with sources median 16.9 KB raw / 6.7 KB zstd-3; without sources 5.4 / 2.7 KB;
  plain dump 12.8 KB raw / 3.4 KB zstd-3; image 239 KB. About 2 KB of a sourceless
  snapshot is its schema (type table).
- Times (native release, load ~5-7): export median 60 us (p99 0.42 ms), import 211 us
  (p99 0.97 ms, includes compiling both scripts), dump 504 us (p99 2.2 ms).
- Large state (release): 100 KB object: snap 114 KB / zstd 4.3 KB, export 0.30 ms, import
  0.13 ms (dump 110 KB / 2.6 KB, 1.5 ms); 1e6 numbers: 4.9 MB / 2.4 MB, 3.1 / 6.5 ms (dump
  8.3 MB / 1.8 MB, 153 ms); Map 10k: 372 KB / 26 KB, 3.6 / 2.3 ms (dump 377 KB / 17 KB,
  13 ms); 20k log lines: 747 KB / 48 KB, 2.9 / 0.42 ms (dump 765 KB / 46 KB, 12.6 ms).
- Portability (scratch worktree work/scratch/0300/perturb, branch scratch/0300-perturb,
  f6f4b1b5): 3 intrinsics inserted at id 0, 2 well-known atoms inserted (every atom id
  moves), a new builtin at the front of the base prelude (base_end 180552 -> 181504, every
  builtin offset moves), BoxCell gained a field before VALUE, HostCall's ID/FN swapped and a
  field added, Interp's HOST_CALL/NEXT_CALL_ID swapped; build hash 88e0ee33 -> 60ba6f4f.
  A's 1361 snapshots (harvest + programs, snaps-48ba92b7) all load into B and continue as
  B's run; B's 1361 load into A once a field the importer lacks is accepted at its zero
  state (first: all refused, "no field PERTURB").
- Refusals: for-in/of lowering changed (two more instructions): 1283 of 1361 refused
  ("the bytecode of the function at 1..10073 of script 1 differs"), 78 load: the codemode
  body is one function, so almost any lowering change reaches it. Array.prototype.push
  renamed: 13 refused ("no builtin Array.prototype.push"), 91 not reached by B's own run, 3
  continue unlike B's own run (B's own past already differed). HostCall::FLAGS renamed:
  1361 refused ("HostCall: no field FLAGS").

- Two-level check (psnapr's 2.7): a function with a stored frame (a Generator's callee) by
  its bytecode hash, every other function by its closures' interface (params, length,
  flags, name, captured bindings by name in order). Constant immediates hashed by value
  (the pool is the script's, shared by its functions), template objects carried by site
  and checked against the importer's `get_template_object`.
- Tried and dropped: a hash of each builtin closure kind's code (its records are
  positional). It refused 2 of 1361 after the internals perturbation (the prelude
  function table's slots moved) and 659 after the sync-function lowering change (the
  builtins are compiled by the same compiler). Records stay a named-schema gap.
- Final experiment (7b2acff4, snapshots work/scratch/0300/snaps-F, perturbation
  branches rebased onto it): internals perturbation 1361/1361 both ways; for-in/of
  lowering 94 load, 1267 refused; Array.prototype.push renamed 13 refused (1254 same, 3
  continue unlike B's own past, 91 not reached); BoxCell::VALUE renamed 1205 refused, 156
  load (no boxes); every sync function's bytecode changed: 1361/1361 load.
- wasm (shipped features, gzip -9): base 427,888; +dump 698,441 (+270.6 KB); +snapshot
  487,758 (+59.9 KB, incl. the intrinsic names and the registry walk); both 761,141.
- Code: snapshot module 2,913 lines (2,608 code lines) + ~60 in heap/ accessors; the
  dumper's vm/dump*.rs (non-test) + call_site.rs 21,836 lines (18,745 code) + compiler
  smap*.rs 2,719.
- Timing split (release): import median 189 us = fresh VM 3 us + compiling the two
  scripts 131 us + the graph ~55 us; export 50 us; dump 492 us.

- Incremental (stretch): `zstd --patch-from` the previous idle point's snapshot: harvest
  pairs median 259 B (dumps 191 B); large cases 56-786 B (1e6 numbers: 786 B instead of
  2.4 MB). No format support needed: the export is deterministic and breadth-first.
- Doc: docs/heap-snapshots-revisited.md sections 3-5 written (on main 0299's sections 1-2).
  Guide pages: none affected (nothing of the engine lands).

## Follow-ups
- If the owner adopts snapshots: the slot table as a checked contract (generated from or
  tested against heap/object.rs); named record schemas for builtin closure kinds (the
  positional `NewRecord`/`RecordGet` records are unguarded; a code hash refused too much);
  a gate pin of the corpus's frame bytecode (0299's 2.7); golden snapshots per release;
  `Sandbox.snapshot()`/`loadSnapshot()` and pi's adapter (sources by hash, deltas, the
  schema kept per build); pi's real prelude (lockdown, tools Proxy) through the harvest;
  size work on the module (60 KB gz, with the intrinsic names and the registry walk).

## Result
A working prototype on branch `issue/0300-logical-snapshots` (not merged: the coordinator
said doc only): `crates/djs-core/src/vm/snapshot/` behind feature `snapshot`
(`Vm::snapshot`, `Vm::snapshot_with`, `Vm::from_snapshot[_with]`, `BaseInfo`, `describe`),
djs-wasm `vm_snapshot`/`vm_from_snapshot` behind its `snapshot` feature, the harvest,
programs, large-state, import-dir and probe tests in vm/snapshot/tests.rs, and
scripts/research/0300 (sizes.py, deltas.py, wasm-sizes.sh). The perturbation builds are
branches `scratch/0300-perturb` and `-bytecode`, `-syncfn`, `-builtin`, `-field` on top.

Results (docs/heap-snapshots-revisited.md sections 3-5): every harvest idle point (1,164)
and every dump-program idle point (197) exports, imports and continues exactly, chained
too; nothing refused. Export 50 us median (dump 492 us), import 189 us (70% compiling).
Sizes: 2.7 KB zstd median without sources (dump 3.4 KB), 6.7 KB with them; large state
3.5-50x faster than dumps; deltas a few hundred bytes. Snapshots survive renumbered
intrinsics and atoms, a moved base, changed cell layouts and any change to functions
that do not wait, in both directions; they refuse, with the reason, a changed waiting
function (for-of lowering: 1,267 of 1,361), a removed builtin path and a removed
non-zero field. Code: 3,090 lines and +59.9 KB gz vs the dumper's 21,836 + 2,719 lines and
+270.6 KB gz.

Known gaps: positional builtin records unguarded; WeakRef/FinalizationRegistry/TextCodec
refused; pi's real prelude untested; no Sandbox method.
