# 0140: Snapshots across djs versions (research report)

status: in-progress
agent: snapshot
branch: issue/0140-snapshots-across-versions
depends-on: 0088, 0117, 0133, 0136, 0139
guide: - (a guide section may follow)
touches: docs/snapshot-versioning.md (new report), scripts/snapshot-research/ (experiments), work/scratch/0140/; no engine changes

## Goal
Owner: "how can snapshots be made stable if the builtins are no longer in the snapshots?
What happens if a new version of djs releases?"

Today a snapshot container (D2260) is the collected image zstd-compressed against a
reference image (the module's base image, or pi's prelude image) that is not stored in
the snapshot, and restore is refused unless the engine build hash matches. The image
itself is a raw heap: offsets, cell layouts, bytecode, shapes, IC slots, builtin function
cells — all specific to one build.

## Questions
1. Precisely what a snapshot depends on today: the reference image (dictionary), the
   build hash, the heap layout (cell formats, Roots), the bytecode format and opcode
   numbering, builtin cells (by offset, from the base image), atoms, shapes, prelude.
   Which of these change in a typical release? (Use git history: how often did each
   change over the last N merges? e.g. layout hash, opcode table, base image bytes.)
2. What happens today on upgrade, end to end, for pi and the Sandbox: a session
   suspended on version N, host upgraded to N+1. Demonstrate it (two builds from
   different commits, a scripted restore attempt).
3. Options, each with what it guarantees, cost, size, risk, and what pi would need:
   a. Engine registry: keep old builds (wasm + adapter by build hash), restore with the
      old engine, finish the session there (drain), revocation list for bad builds.
   b. Self-contained snapshots: store the reference (or the diff to a published,
      content-addressed reference) so a snapshot doesn't depend on the host having it;
      still pinned to the engine.
   c. Image migration: a versioned image format plus per-version migration (cell layout
      changes, opcode renumbering, builtin identity by stable id instead of offset);
      how far this can go (bytecode of suspended frames is the hard part).
   d. Journal replay (0088's B): record inputs (tool results, time, randomness) and
      replay the script on the new engine; determinism makes it reproduce; what breaks
      it (behaviour changes between versions, nondeterministic hosts), size and time.
   e. Logical snapshots (0088's C): serialize JS-level state (objects, closures,
      suspended async frames by source position) independent of the engine; feasibility.
   f. Hybrids: e.g. registry for in-flight sessions + journal for long-term durability;
      stable builtin ids plus a release policy that keeps image layout compatible
      within a minor series.
4. A recommendation for pi's actual needs (how long sessions stay suspended, how often
   djs would be upgraded, acceptable failure modes), with a phased plan.

## Deliverable
docs/snapshot-versioning.md: a clear report for the owner, with the demonstration,
numbers, and a recommendation. Reuse 0088's research (work/issues/0088-*.md and its docs)
and update it rather than repeating it.

## Log

## Follow-ups

## Result
