# 0133: A heap dump analyzer

status: open
agent: -
branch: issue/0133-heap-dump-analyzer
depends-on: 0117
guide: docs/guide/snapshots.md or gc.md (a short "inspecting a heap" section, D2102 rules)
touches: crates/djs-cli/ (a subcommand) or a new small tool crate, js/ (only a helper to unwrap snapshot containers if done in node), scripts/ (wrapper), TESTING.md / docs/host.md (usage)

## Goal
Owner request: a small tool to see what is in a heap dump, i.e. a raw heap image, a
legacy DJSS/DJPI snapshot, or a 0117 snapshot container (zstd against a reference image).
Today only the REPL's `.heap` prints cells per kind and objects per class for the live VM.

## Acceptance criteria
- [ ] Input: raw image, legacy snapshot formats, and the 0117 container (decompress with
      the reference image: the base image from the module, or pi's prelude image, which
      the tool can rebuild or be given as a file). A tool crate may use the zstd crate.
- [ ] Output (text by default, `--json` for scripts):
  - summary: total size, live vs garbage (after a collection), counts and bytes per cell
    kind and per object class, old region (OLD_END) vs young;
  - **diff against the reference image**: which cells are new or changed relative to the
    base/prelude image, grouped by kind/class with bytes. This is what explains a
    snapshot's size;
  - top-N largest cells, and largest retained sizes (dominator tree or a simple
    approximation) with a path from the roots (e.g. `globalThis.cache[3].body`);
  - strings: count, bytes, top duplicates, the largest strings (truncated);
  - shapes: count, dictionary vs shared, the most common property-name lists;
  - functions/bytecode: templates and bytecode bytes, closures;
  - pending jobs, promises, suspended frames (what a waiting session holds).
- [ ] Reuses djs-core's own cell layout and walker (heap/walk.rs etc.), not a second
      parser, so it can't drift; a test runs it over a few known images (empty Sandbox,
      a pi waiting session from scripts/snapshot-research cases) and checks key numbers.
- [ ] Usage documented; one example output in the issue's Result.

## Log

## Follow-ups

## Result
