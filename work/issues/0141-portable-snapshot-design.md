# 0141: Deep dive: a heap snapshot design that is portable across djs versions

status: in-progress
agent: portable
branch: issue/0141-portable-snapshots
depends-on: 0088, 0140
guide: - (a guide chapter may follow)
touches: docs/portable-snapshots.md (new design report), work/scratch/0141/ (prototypes), scripts/snapshot-research/ (experiments); no engine changes on main unless a tiny measurement hook is needed (announce it)

## Goal
Owner, after 0140 (docs/snapshot-versioning.md concluded: images are inherently
build-specific; journal replay is the durable record; migration and logical snapshots
"not planned"): "what would a heap snapshot design require that is more portable between
versions? Let's make a deep dive. Surely there has to be a better way."

Take the opposite stance from 0140: **assume we want snapshots that survive engine
upgrades** and design the best way to get them. Find out what it really takes, what
it costs, and where the hard limits are, with prototypes for the hard parts.

## Directions to explore (not exhaustive)
- **Separate code from data.** Code identified by content (hash of the script source +
  the prelude source), recompiled by the new engine on restore; the snapshot carries only
  data and *source-level* descriptions of suspended code.
- **Source-level suspension points.** Suspended async functions/generators described by
  stable, compiler-independent coordinates (e.g. the n-th await in a function identified
  by source span/hash), locals and scopes by source-level identity (binding name +
  scope path), not by register/slot. What about temporaries on the operand stack at an
  await (`f(a(), await b())`)? Options: a spec'd source-level evaluation-state
  description; a compiler rule that spills temporaries to named synthetic locals at
  awaits; restricting portable suspension to statement-level awaits and falling back
  otherwise. Measure how often each case occurs in the 2,627 harvested codemode scripts
  (scripts/harvest, work/scratch/0130 has the extraction).
- **Builtins by stable identity.** Every builtin object referenced by a spec-level path
  (`%Promise.prototype.then%`, `%Array.prototype%`), engine-internal functions
  (promise resolve functions, async-from-sync iterators, bound functions) by spec
  meaning + their captured state, never by offset or numeric id.
- **Data as a logical object graph.** Objects, arrays, strings, Maps/Sets, promises with
  reactions, closures with their environments, symbols, errors (stack?), typed arrays,
  property order and attributes, prototypes; engine caches (shapes, ICs, atoms) dropped
  and rebuilt. What's needed beyond structured clone (closures, promises, async frames,
  pending jobs, private fields, WeakMaps)?
- **Prior art to study and cite**: Smalltalk/Squeak/Pharo images across VM versions,
  Lua Pluto/Eris persistence (closures, coroutines), Stackless Python tasklet pickling,
  Gambit/Scheme serializable continuations, Java continuation libraries (Javaflow, Kilim),
  Erlang hot code upgrade and term format, V8/SpiderMonkey snapshots (and why they're not
  portable), Hermes bytecode, durable execution systems (Temporal, Restate, Golem
  wasm, Cloudflare durable objects / workflows), CRIU, Unison content-addressed code.
  What did each do about code changes under suspended frames?
- **Hybrid with the fast path**: keep today's image as the same-build fast path; the
  portable form is written alongside (or derived from the image by the old engine when an
  upgrade happens: "export on drain"), and imported by the new engine.
- **What the engine must promise** across versions for this to work (compiler
  determinism at source level, builtin behaviour), and how the gate would test it
  (old-build snapshots of the corpus restored by every new build; golden portable
  snapshots).

## Deliverable
docs/portable-snapshots.md: a concrete design (format sketch, what's captured, the restore
algorithm, what the compiler and runtime must guarantee), prototypes of the hardest
parts (e.g. export/import of a suspended async function across two builds from different
commits, in work/scratch/0141), measured coverage on the harvested codemode corpus,
costs (size, time, implementation phases, risk), hard limits, and a comparison with
0140's journal-replay recommendation. Be honest where it doesn't work.

## Log

## Follow-ups

## Result
