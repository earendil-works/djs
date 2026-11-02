# 0144: Frozen, lazily created builtins outside the session heap (design + prototype)

status: in-progress
agent: heap
branch: issue/0144-frozen-lazy-builtins
depends-on: 0138, 0141, 0142, 0143
guide: - (later)
touches: docs/frozen-builtins.md (design), work/scratch/0144/ and a scratch branch (prototype); no engine changes on main

## Owner's direction
"We want to bring down the overall cost of heap snapshots. Putting all builtins eagerly
into the heap is useful for determinism, but the order in which these objects are created,
and when, should not necessarily matter." And: "what if djs gains an entire mode where all
builtins start out frozen? That would greatly simplify things."

## The design to work out
A VM mode (pi's build) where:
1. **All builtins are frozen from birth** (Node `--frozen-intrinsics` / SES semantics, with
   the override-mistake fix: assignment to an inherited non-writable builtin property
   creates an own property on the receiver instead of throwing, as pi's lockdown emulates
   with ~680 accessor closures). The global object stays extensible for script globals;
   its builtin bindings are frozen.
2. **Builtins are created lazily** (on first touch / first call), and because they are
   frozen, a builtin is a pure function of (build, registry id): nothing per-VM to
   preserve.
3. **Creation order and time don't matter**: builtins live **outside the session's
   arena** (a separate builtin space per VM or per instance, or module data), so the
   session heap's allocation sequence, layout, collections, image bytes and
   canonical_hash are independent of which builtins were touched and when. Determinism is
   defined over the session heap only.
4. **Snapshots contain only the session heap**; references to builtins are by stable,
   append-only registry id (bloat's 0143 step 1). Restore re-creates builtins on demand.
   Same-build images stay exact; across builds the references survive as long as the
   registry keeps the id (the session's own bytecode/frames still need 0142's tier 2 for
   cross-build restores).

## Questions
- Reference representation: how a heap Value/Ref names a builtin-space cell (0138's
  high-offset split? a tag?), the access-path cost (0138 measured eager vs lazy-copy
  variants), ICs/shapes keyed on builtin holders, GC (builtin space never collected or
  collected separately?), pointers from builtin space to session heap (must not exist:
  frozen + no per-VM state makes that hold?).
- What state builtins have that isn't frozen-object state: Symbol.for registry, RegExp
  legacy statics, Math.random seed, Promise job queue, the atom table (builtin names as
  fixed atoms, 0143 step 3), shapes of builtin objects; where each lives.
- The override fix: exact semantics, spec deviations, test262 impact (the default build
  stays spec-conformant; the mode is opt-in).
- Determinism: confirm that with builtins outside the arena, two runs touching builtins in
  different orders give identical session images and canonical hashes (prototype test).
- pi: the prelude drops its lockdown (owner decision later; not changed now): measure what
  a pi session costs under the mode (bloat: today 471 KB / 8,370 cells).
- **Prototype** on one family (Array + Array.prototype + its iterator) end to end in a
  scratch branch: frozen, lazy, outside the arena, snapshot by id; measure fresh-heap
  size, first-touch cost, lookup/call cost (wasm instructions on the workloads),
  snapshot sizes, and order-independence.
- A revised plan replacing 0143's steps 2–4 (and 0138), with estimates and risks.

## Deliverable
docs/frozen-builtins.md with the design, prototype numbers, and the plan.

## Log

## Follow-ups

## Result
