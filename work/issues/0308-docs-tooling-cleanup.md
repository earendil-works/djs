# 0308: Docs and tooling cleanup after the dump removal

status: in-progress
agent: pclean
branch: issue/0308-docs-tooling-cleanup
depends-on: 0307
touches: docs/ (non-guide), docs/guide, work/research, scripts/, crates/djs-cli, crates/djs-docs, scripts/gate.py, TESTING.md, ARCH.md
guide: docs/guide/cli.md, docs/guide/js-api.md, docs/guide/host.md, docs/guide/snapshots.md, docs/guide/determinism.md

## Goal
- Delete the dump research archive (docs/suspension-design-review.md, work/research/0188,
  0199, 0207, 0215, 0216); trim docs/heap-snapshots-revisited.md only where it misleads.
- Fold docs/cli.md and docs/host.md into the guide (cli.md, js-api.md, host.md): one place
  per topic; move any design reasoning only they hold into the guide's trade-off boxes or
  ARCH.md; update the code comments that point at them.
- Prune docs/decisions-index.md of superseded and removed-component entries; add D3640+.
- The guide pages that still describe moving a VM between fuel slices through an image, or
  mid-run images as a use, describe djs as it is; `djs run --suspend-every`, the REPL's
  `.suspend-every` and the docs runner slice in the same VM (difftest's `--suspend-every`
  through images stays: it is the heap-consistency stress, and byte-identical images stay a
  development check, owner).
- Gate: drop the stress variants from the unchecked release difftest leg (the checked leg
  runs them); record the CPU saved.
- Delete unreferenced research/perf/size scripts (scripts/wasm-areas-unnamed.py,
  wasm-leb-padding.py, unmerged scripts/size/patches, perf tools only the README names).

## Acceptance criteria
- [ ] The above; djs-docs check clean; TESTING.md and ARCH.md follow.

## Log

## Follow-ups

## Result
