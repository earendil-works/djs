# 0309: One meaning per word: images and snapshots

status: done
agent: pname
branch: issue/0309-names-images-snapshots
depends-on: -
touches: docs/guide (persistence.md -> snapshots.md, snapshots.md -> images.md, SUMMARY.md, every link and mention), ARCH.md, TESTING.md, README.md, js/README.md, crates/djs-embed (Snapshot -> Image and its builder, docs, examples, tests), crates/djs-core/crates/djs-prelude/djs-cli/djs-bench/djs-docs comments and names that say "base snapshot", docs/decisions.md
guide: docs/guide/SUMMARY.md, persistence.md, snapshots.md, rust-api.md, heap.md, host.md, js-api.md, cli.md and every page linking them

## Goal
"Snapshot" now means two things in djs: the persisted form of a waiting run (vm/snapshot,
`Run.snapshot`, `loadSnapshot`, guide persistence.md "Snapshots: keeping a waiting run") and
in-process images of a VM's memory (guide snapshots.md "Images and the base snapshot", "the
base snapshot", djs-embed's `Snapshot` of a host's setup). The owner: one meaning per word.
- Guide: persistence.md becomes snapshots.md (the run snapshot), snapshots.md becomes
  images.md (images, the base image); SUMMARY and every link updated; djs-docs check clean.
- "base snapshot" -> "base image" everywhere (docs, comments, identifiers where they say so).
- djs-embed: `Snapshot` -> `Image` (builder, methods such as `Instance::snapshot`, docs,
  examples, tests). No compatibility alias (D024/D3640). Gondolin pins djs-embed at a git rev
  (~/Development/gondolin/native/crates/gondolin-hooks-djs, read-only for you): list in the
  Result the renames it needs when it moves its pin.
- Identifiers and names elsewhere that use "snapshot" for an image (djs-prelude, djs-core's
  base build, the CLI/REPL, benches, the docs runner) follow; the run snapshot keeps its names.

## Acceptance criteria
- [x] One meaning per word in code and docs: `git grep -i snapshot` hits only run snapshots
      (and decisions.md's history); images are images.
- [x] Decision recorded (D3756).

## Log
- Guide: `git mv` snapshots.md -> images.md ("Images and the base image"), then
  persistence.md -> snapshots.md (title unchanged); every link, the anchor
  `#the-base-snapshot` -> `#the-base-image`, SUMMARY, and the live examples' info link
  (djs-docs render.rs, `snapshots.html`). The run-snapshot page's paragraph on the word's
  other meanings now says what an image is instead. rust-api.md speaks of images
  throughout; language.md, heap.md, benchmarks.md, overview.svg likewise. Prose only,
  no rewrite (another investigation may change how the guide shows a snapshot).
- "base snapshot" -> "base image" everywhere outside work/ and the decision log (code
  comments, Cargo descriptions, ARCH, TESTING, js/src/base.ts, the guide, djs-docs'
  status label, the bench metric `startup/base image size` with its rows in
  tests/scores/bench.tsv so the series continues).
- djs-embed: `Snapshot` -> `Image` (src/snapshot.rs -> src/image.rs, tests/it/snapshot.rs ->
  tests/it/image.rs), `Instance::snapshot()` -> `Instance::image()`,
  `Snapshot::image_len()` -> `Image::size()` (`len` would want an `is_empty` for clippy;
  `size` matches `Instance::heap_size`); docs, examples and tests follow (helpers
  `snapshot()` -> `image()`, `snap` -> `img`). No alias.
- djs-core comments that said snapshot for an image (build hash, restore, the collector,
  handles, the OOM reserve flag) say image; for-in's "key snapshot" is a copy; the
  compiler's expected disassembly files are listings (src/compiler/listings,
  `DJS_UPDATE_LISTINGS`, test `compiler::tests::listings`); ref-opcodes regenerated.
- Left as they are: work/ (issue history), docs/decisions.md, and the dated design and
  research notes in docs/ (frozen-builtins.md, objects.md, old-generation.md,
  size-vs-quickjs.md, architecture-bloat-audit.md, wasm-jit.md, web-globals.md,
  lessons-from-pico.md, ...), which use the word as it meant when they were written.
  scripts/size/areas.py's area "snapshots, suspension, wire format" covers both modules
  and keeps its name (tests/scores/size.tsv series).
- Guide pages updated: SUMMARY, images.md, snapshots.md, rust-api.md, arena.md,
  builtins.md, cli.md, gc.md, goals.md, host.md, introduction.md, language.md, heap.md,
  benchmarks.md, status.md (regen), ref-opcodes.md (regen), README.md, img/*.svg; djs-docs
  check clean.
- Checked: djs-embed tests, djs-core compiler:: and the renamed vm tests, workspace clippy
  -D warnings, fmt, djs-docs check.

## Result
One meaning per word: a **snapshot** is the persisted form of a waiting run; an **image**
is a VM's memory inside one process and build, the **base image** the one every VM starts
from (D3756). Guide: snapshots.md is "Snapshots: keeping a waiting run" (was
persistence.md), images.md is "Images and the base image" (was snapshots.md).
`git grep -i snapshot` outside work/, the decision log and the dated notes in docs/ hits
only run snapshots (and `heap-snapshots-revisited.md`, about them).

What Gondolin (native/crates/gondolin-hooks-djs, pinned at 0e58f62c) renames when it
moves its pin:
- `djs_embed::Snapshot` -> `djs_embed::Image` (src/engine.rs imports and the
  `Program.snapshot` field's type; `Snapshot::builder()` -> `Image::builder()`),
- `snapshot.image_len()` -> `image.size()` (src/engine.rs `image_len`, and through it
  src/lib.rs),
- `Instance::snapshot()` -> `Instance::image()` if it uses it (it does not at 0e58f62c),
- unchanged: `Builder`, `instantiate`, `host_fns`, `modules`, `Source`, `SourceKind`,
  everything on `Instance` but `snapshot()`.
Its own comments ("compiled ... into a snapshot") would read better as "image".

## Follow-ups
- None required. The dated notes in docs/ could be swept if the owner wants the word gone
  there too.
