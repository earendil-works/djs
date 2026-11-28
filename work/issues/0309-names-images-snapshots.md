# 0309: One meaning per word: images and snapshots

status: open
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
- [ ] One meaning per word in code and docs: `git grep -i snapshot` hits only run snapshots
      (and decisions.md's history); images are images.
- [ ] Decision recorded.
