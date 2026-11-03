# 0164: Image compatibility by dependencies, not by source hash

status: open
agent: -
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

## Follow-ups

## Result
