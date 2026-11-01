# 0088: Snapshot design research: long-lived, cross-version, small (report only)

status: open
agent: -
branch: issue/0088-snapshot-research
depends-on: -
touches: work/issues/0088-snapshot-design-research.md (the report), docs/ (an optional design note draft, not adopted), work/scratch/0088/ (prototypes and measurements). No changes to engine code on main.

## Goal
Today a snapshot is a raw heap image (ARCH "Snapshots", D1320/D1321/D1330/D1750): compact
after a GC, deterministic, fast to restore (a copy), but **locked to the exact build** (the
build hash covers the heap layout, the opcode table, intrinsic ids, djs-core's sources,
features and libm), and sized like the live heap (the base image alone is ~400 KB). The
owner wants snapshots that **survive for a very long time, potentially across engine
versions, and are small**. Research what novel designs could look like and compare them
with today's. **We do not switch automatically**; this is about understanding the design
space and what each option would cost.

## Questions to think through (not exhaustive)
- What exactly is version-specific in today's images (cell layouts, bytecode and pcs in
  suspended frames, intrinsic ids, atom ids, prelude objects, shapes (0083), IC state,
  native frames (0063) and their state numbers, realm intrinsics, internal records) and
  what is inherently durable (user data: objects, strings, arrays, Maps, typed arrays …)?
- **Logical snapshots**: serialize the user-visible object graph in a versioned,
  self-describing format (references to builtins and intrinsics by name or path, not by
  id; strings deduplicated; compressed), and rebuild the realm from the current version's
  base on restore. How are closures represented: source text + a stable function
  identity (for example a source span or path) + captured variables? How are suspended
  computations represented: async functions parked at an `await`, generators, pending
  promise reactions, the job queue, the host inbox (D1670)? Can a continuation be
  re-expressed version-independently (function identity + a stable suspension-point id
  derived from the source, locals by name), and when must a restore refuse?
- **Restricting snapshot points**: for example only at quiescent points (idle, waiting on
  host calls, no native/builtin frames on the stack), which is codemode's natural
  durability point. How much does that simplify the problem?
- **Replay-based durability** (0073's journal): the source plus the journal of external
  inputs, re-executed on restore. That's tiny and as version-robust as the semantics are
  stable, but restore time grows with the history. Hybrids: periodic logical
  checkpoints + journal tails; "replay against a newer engine" and how to detect
  divergence (canonical hashes, D1730, are version-specific today).
- **Size**: delta snapshots against the base (ARCH "Deltas"), content-addressed chunks,
  string interning across snapshots, compression (what ratio on real codemode heaps?),
  dropping recomputable state (ICs, shapes, compiled code that can be recompiled from
  source).
- **Compatibility policy**: what guarantees would we make (N versions back? a migration
  layer?) and how would they be tested (golden snapshots in the repo, restored by every
  future version in the gate)?
- Prior art: V8 snapshots/ValueSerializer, Smalltalk images, CRIU, Erlang hot code
  loading, Temporal/durable-execution workflow engines (replay with determinism and
  versioning markers), Cloudflare Durable Objects, React Native Hermes bytecode, etc.

## What to deliver
- [ ] An inventory of version-specific vs durable state, checked against the code.
- [ ] 2–4 concrete candidate designs, each with: the format sketch, what it can and
      can't represent (and when a restore must fail), restore cost, size on realistic
      codemode heaps (measure: take images of real scripts, estimate or prototype the
      alternative encoding in scratch), version-evolution story, effect on determinism
      (D1090, D1730), implementation cost, and what it would need from the compiler
      (stable ids for functions and suspension points).
- [ ] A comparison table against today's raw images, and a recommendation (including "keep
      raw images for in-process/short-term, add X for long-term").
- [ ] Optionally a design note draft in docs/ (clearly marked "not adopted").

## Notes
- Read ARCH.md (Snapshots, Deltas, Host Interface), docs/host.md (snapshots, journals,
  pi codemode), docs/compiler.md 7 (frames, suspension), docs/objects.md, and the
  decisions D1320–D1330, D1600–D1601, D1670, D1680, D1730, D1740, D1750.
- Own worktree. The machine is shared: small prototypes only, and follow AGENTS.md
  "Testing while iterating".

## Log

## Findings

## Candidate designs

## Recommendation

## Result
