# 0117: Compact snapshot images (snapshot direction A, D019)

status: open
agent: -
branch: issue/0117-compact-snapshots
depends-on: 0088, 0089
guide: docs/guide/snapshots.md, docs/guide/host.md
touches: crates/djs-wasm/ (an export for the reference descriptor, if needed), js/src/sandbox.ts, js/pi/ (snapshot()/resumeVm), crates/djs-core/src/vm/ (image header: reference descriptor, additive), docs/host.md, scripts/snapshot-research/ (re-measure)

## Goal
Direction A from 0088 (work/issues/0088-snapshot-design-research.md), approved by the owner
(D019): snapshots are collected images compressed with zstd against the recomputable
reference image (the base image, or the post-prelude image for pi) used as a dictionary,
with a header that names the reference (engine build hash + which reference image +
its hash) and restore pinned to the engine build that wrote it. 0088 measured a waiting
session at 943 KB raw → 22–27 KB this way, with ~0.6 ms extra on restore.

## Acceptance criteria
- [ ] A snapshot container format (versioned header: magic, format version, engine build
      hash, reference kind + reference hash, compression, payload) documented in
      docs/host.md; the Sandbox's `bytes()/fromBytes()` and the pi adapter's
      `snapshot()/resumeVm()` produce and consume it; old raw images still load.
- [ ] Compression in the host (node's built-in zstd with a dictionary; check what the
      browser path can use, and fall back to uncompressed where zstd isn't available) or in
      wasm if that turns out better; measured both ways.
- [ ] Refusing to restore an image from a different engine build or reference, with a clear
      error (the host keeps old engine builds if it needs old snapshots: document the
      engine-registry idea from 0088 as a follow-up).
- [ ] Sizes and restore times re-measured with scripts/snapshot-research (wasm, not native
      dev builds), and a round-trip test in the gate's node tests.
- [ ] The guide's snapshots.md and host.md updated (D2102 rules).

## Log

## Follow-ups

## Result
