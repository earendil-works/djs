# 0140: Snapshots across djs versions (research report)

status: done
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

- Churn: `scripts/snapshot-research/churn.py` groups main's commits by issue and checks
  which snapshot inputs each touched (139 issues): build hash 100, cell layouts 22,
  opcodes 14, intrinsic table 33, realm slots 23, builtins (prelude.js) 39, compiler 33,
  base image layout 10, pi adapter 10, container 2; the last 30 issues: 14, 1, 0, 2, 2, 3,
  2, 5, 3, 2.
- Demonstration: build N = 104f8a1e (main after 0133, `git archive` + build-wasm.sh in
  work/scratch/0140/n), build N+1 = this branch (a195956f). `scripts/snapshot-research/
  upgrade.ts` drives each build with its own host code: Sandbox and pi snapshots from N
  are refused by N+1 (clear message), restore and finish on N (registry), Sandbox.replay
  refuses another engine by design, a semantic replay and pi's replay of the journaled
  results reproduce the run exactly on N+1 (21 ms); pi's fingerprint differs only in the
  djs build hash.
- Between N and N+1 (same layout hash, same opcodes): 307 of 450 intrinsic ids moved by
  one (one insertion, `EachDense`); the analyzer reads N's session's promise resolver and
  rejecter as other builtins; 425 of 931 base bytecode cells and 57 of 874 builtins were
  replaced; 11% of the base's words in place.
- Report: docs/snapshot-versioning.md (dependencies with churn, the demonstration, options
  a–f with guarantees/cost/size/risk, a recommendation and phases). No engine changes, no
  decisions.

## Follow-ups

- Sandbox: a cross-version replay mode for journals (check calls, arguments and result;
  hashes only on the same build).
- Golden journals from the harvested codemode corpus (0118) replayed by every build in the
  gate.
- pi (when it adopts djs): store snapshots next to the durable journal; let a changed djs
  build hash through the fingerprint check (keep module version, prelude, limits strict).
- Only if needed: an engine registry with drain and a revocation list.

## Result

docs/snapshot-versioning.md answers the owner: a djs snapshot was never portable across
builds, with or without the builtins in it (an image is the engine's memory: bytecode,
builtins by offset and intrinsic id, layouts); since 0117 the builtins are named by hash
instead of stored, which made snapshots 100x smaller without changing that. On an upgrade
today every old snapshot is refused with a clear error (demonstrated for the Sandbox and
pi between two real builds). Recommendation: journal replay as the durable record across
versions (pi already replays to survive restarts; across builds it reproduced the run
exactly), the snapshot as the same-build fast path, an engine registry with drain only if
exact continuation across an upgrade is required; no image migration or logical
snapshots. Reproducible with scripts/snapshot-research/churn.py and upgrade.ts.
