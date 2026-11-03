# 0164: Image compatibility by dependencies, not by source hash

status: in-progress
agent: snapshot
branch: issue/0164-image-compat
depends-on: 0117, 0147, 0148
guide: docs/guide/snapshots.md (when an image restores), host.md
touches: crates/djs-core/src/vm/ (an image's dependency descriptor), crates/djs-wasm/ (exports for it), js/src/snapshot.ts (container header check), crates/djs-cli/src/container.rs, crates/djs-prelude/ (computing the hashes at build), scripts/ (a check that the compat version is bumped when needed), docs/host.md; heap (0158) is in djs-wasm memory/base paths: coordinate via the coordinator

## Goal
Owner decision D026 (pico lesson 6): a snapshot restores when what it depends on is
unchanged, not only on the identical engine build. The container header (D2260, D2400)
names a **dependency descriptor** instead of (or next to) the build hash:
- the cell/image layout hash (layout.generated.ts already has one),
- opcode table and intrinsic table hashes (their numbering is what bytecode in the
  session region and builtin closures reference),
- the builtin prefix's hash (D2066: base prefix hash, already in session images),
- the container format version,
- a hand-bumped `COMPAT_VERSION` for semantic changes the hashes can't see (e.g. a
  change in how a builtin's internal record is interpreted).
Refusal messages say which dependency differs. A gate check fails when a change alters
a dependency's inputs without the descriptor changing (e.g. a test that flips one opcode
and expects a different opcode hash), and a short guideline (TESTING.md / AGENTS.md) on
when to bump COMPAT_VERSION. Measure how often each dependency changed in the last N
merges (0140's churn.py) to show how much more often snapshots now survive.

## Log

- Descriptor (D2265): `vm::ImageDeps` in vm/compat.rs, six u32 in the roots cell:
  layout (`VERSION`), `FEATURES` (96), `COMPAT` (48, was `BUILD`), `OPS` (84),
  `INTRINSICS` (88), `RECORDS` (92); Roots grows 88 → 104 bytes (heap/cells.rs, outside
  `touches`: the fields had to live in the roots cell). `Vm::new` writes them;
  `check_image` compares layout first, then the rest; `LoadError::Incompatible { dep,
  found, expected }` replaces `BuildMismatch`. `hash_intrinsics` split out of
  `INTRINSICS_HASH` so a test can flip an entry.
- `RECORDS_HASH` (D2266): build.rs hashes the regions of crates/djs-core/compat_inputs.rs
  (shared with vm::compat_tests via include!). First cut counted intrinsic-id consts
  (`const X: u16 = id("Name")`, 280 of them in interp/run/intrinsics.rs): they changed in
  28 issues and duplicate INTRINSICS_HASH, so they are left out. Added `Continuation` and
  typed array `Kind` after reading what frames and typed arrays store.
- Features (D2267): after the coordinator's notes on `unsafe-heap` (0161, image-neutral)
  and `full` (0160, the slim build), features are their own component, a bitmask with
  names, instead of being mixed into RECORDS_HASH: the refusal says "another feature set
  (image: full, regexp-unicode-props, unicode-normalization; this build: ...)". Unclassified
  features fail the build.
- Session images format 2 (offset 8: `ImageDeps::hash`), `session::prefix_hash`.
- Container format 3 (header 68): descriptor at 8..32, base prefix FNV64 at 32, then the
  reference fields. js/src/snapshot.ts `moduleDeps(base)` reads the module's descriptor
  from its base image's roots (layout.generated.ts offsets), `checkSnapshot(header, app,
  base)`; sandbox.ts / pi vm.ts pass the base instead of `djs_build_hash`;
  `instanceBuild` removed. djs-cli container.rs checks the same (`check`), heap report shows
  the descriptor, REPL/heap tests call sites updated. No djs-wasm change: the JS host reads
  the descriptor from the base image it already has.
- Gate checks (vm/compat_tests.rs): each hash changes when one entry of its table does
  (ops, intrinsics, layouts, atoms), region extraction finds every region and sees numbers
  but not comments, every component refused by name, features named, session header
  holds the descriptor hash. js/test/snapshot.test.ts: each header component refused by
  name; FEATURE_NAMES mirrors compat_inputs.rs.
- Churn (scripts/snapshot-research/churn.py --deps, base-churn.sh builds djs-prelude's base
  at every merge since it exists, ~130 builds): up to main 7074117e, a raw image survives
  80 of 158 issues (51%) vs 48 (30%) under the build hash; a container (descriptor + base
  prefix) 61 of 129 (47%) vs 40 (31%) where every base was built; last 30 issues: 63% /
  60% vs 47%. Base prefix changed in 54 of 129 issues. docs/snapshot-versioning.md "Since
  D026".
- Size: +835 B gzip (+1.7 KB raw; code +1.3 KB, data +0.4 KB) on the shipped module at
  the base commit (455,502 → 456,337, over the 455,600 ceiling there); main has since
  dropped to ~450.4 K, so after the rebase it is under. `#[inline(never)]` on check_image
  changed nothing (not inlined anyway).
- Docs: docs/host.md (container table, refusal order, "What an image depends on"),
  guide snapshots.md (section rewritten: "What an image depends on"), arena.md, heap.md,
  values.md, compiler.md, determinism.md, host.md; TESTING.md "Snapshot compatibility"
  (when to bump COMPAT_VERSION); AGENTS.md code-conventions pointer; ARCH.md Snapshots
  bullet (outside touches; it described the build hash as the key).

## Follow-ups

- pi session containers compressed against pi's prelude image also need that image to be
  byte-identical on the new build (checked by hash, not in the descriptor); any compiler,
  collector or pi-prelude-builtin change breaks it. Measure it like the base, and consider
  writing pi snapshots that must survive upgrades against the base.
- 0163 (seeded string hashing): if it changes how strings or atoms are hashed in stored
  tables, it must bump `vm::COMPAT_VERSION` (TESTING.md "Snapshot compatibility").
- A cheap lint for COMPAT bumps: warn in merge-to-main when a change touches the
  sensitive code paths (record readers in builtins/*.ts, suspend/promise/collection
  code) without a COMPAT_VERSION change. Not built; judgment stays with the reviewer.
- The +835 B gzip could probably be trimmed (descriptor as a contiguous array, plainer
  messages in Rust); not attempted.

## Result
