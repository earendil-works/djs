# 0141: Deep dive: a heap snapshot design that is portable across djs versions

status: done
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

- Read 0140 (docs/snapshot-versioning.md), 0088 and its harness, docs/host.md
  (containers, journals, pi), ARCH (heap, compiler, interpreter, snapshots),
  docs/compiler.md 6–7.8, the guide's snapshots.md, interp/run/{suspend,promise}.rs,
  `djs heap`'s frame reading (crates/djs-cli/src/heap/graph.rs).
- Key observation: an upgrade changes only the engine, never the script or pi's prelude,
  so a suspension point of unchanged source has a compiler-independent meaning (the
  spec's evaluation state). That is what every prior-art system lacks (they face new
  *user* code under old frames) and what makes a logical export/import feasible.
- Coverage, static: scripts/snapshot-research/await-sites.mjs (acorn AST, counts only)
  over the 2,535 distinct scripts of work/scratch/0130/extract.jsonl (primary checkout;
  nothing printed but counts): 4,881 awaits; 48.6% hold only bindings, 4.4% a for-of
  iterator, 47.0% expression operands (37% `call.callee` alone, mostly `text(await …)`),
  0 finally/switch. Ten roles + the for-of record cover all.
- Prototype: scripts/snapshot-research/portable (standalone crate on djs-core's public
  API, ~2,300 lines). Export of a waiting VM as JSON (code by script + span, awaits by
  source position, locals by name, operands by role from a bytecode abstract
  interpretation, builtins by path, builtin closures by intrinsic name, records by named
  schema, host functions by id, pending host calls re-issued by sequence number, the
  global object's added properties as a patch); import rebuilds frames in the importing
  build's layout. Fixed along the way: privileged prelude methods (%ArrayIteratorProto%/
  next) need intrinsic-rooted paths; builtin paths must be checked against a fresh realm
  (the corpus prelude puts `text`/`tools` on the global); a closure without line-table
  entries is not privileged (script name decides).
- Cross-build: the same crate copied into `git archive 1b6c2098` (main before 0080, 406
  commits back: no shapes, 22 cell kinds, 404 KB base, other intrinsic ids, a 5-field
  Promise.all element record) builds unchanged with `--features old`.
  - Own cases (statement, for-of with `out.push(await …)`, Promise.all, try/finally +
    `sum += await`, `.finally`/`.catch`): every idle point OLD → NEW gives the
    uninterrupted output.
  - Harvested corpus (300 cases, 278 with idle points, up to 25 points each): 1,057/1,057
    round-trip on each build; OLD → NEW and NEW → OLD: 1,057 imported, 1,008 continue as
    the exporter's run, 49 as the importer's own run (the builds differ: stack-trace
    columns, D2031), 0 otherwise. 46% of the 5,209 activations resume at another pc;
    every builtin closure id differs; frame sizes equal; operand roles identical at every
    point.
  - Sizes: JSON median 27 KB (6.3 KB zstd -3; includes the corpus prelude and the cases'
    embedded results) vs 368 KB collected images. Times (native dev): export 3.6 ms, of
    which 3 ms are two realm walks for builtin paths; import 0.8 ms.
- Prior art studied from what I know (no web access): Smalltalk images, Pluto/Eris,
  Stackless, Gambit/Termite, Racket stateless servlets, Javaflow/Kilim, Erlang hot code
  loading and ETF, V8/SpiderMonkey snapshots and serializers, Hermes, Temporal, Restate,
  Golem, Cloudflare DO/Workflows, CRIU, Unison. Claims kept general where unsure.
- Wrote docs/portable-snapshots.md: design (format, naming table, export/import
  algorithms, compiler and runtime contract, pi's reference world, hybrid with images
  and "export on drain"), prototype results, coverage, prior art, costs (4–6 weeks in
  three phases), hard limits, gate tests, comparison with replay, recommendation.
- No engine changes. No measurement hook needed: everything goes through djs-core's
  public API. Guide: no page affected (snapshots.md still describes what is built).
- Decision D2350 (the prototype's place and how it builds against older trees).

## Follow-ups

- Suspension-map pin test (recommended now, small): the compiler reports, per `await`
  of the corpus and e2e async tests, the roles of the operands below the awaited value
  and the bindings in scope; a committed snapshot makes lowering changes at awaits
  visible decisions (docs/portable-snapshots.md 9, 11).
- If portable snapshots are adopted: phase 1 in djs-core (export/import, compiler
  suspension maps with spec roles and binding scope positions, binary format, host API,
  corpus round trip in the gate), then phase 2 for pi (Proxy, lockdown patches or the
  prelude as a reference stage, schemas for all builtin closure kinds, golden
  snapshots), as in docs/portable-snapshots.md 7.
- A table of builtin names inside the engine (realm objects by spec path), replacing the
  prototype's realm walks (3 ms per export); also useful to `djs heap`'s paths.
- Possibly a guide section on portability once a direction is decided.

## Result

docs/portable-snapshots.md: a design for snapshots that survive engine upgrades. At idle
points the old engine exports the VM as a logical graph in source and spec terms; the new
engine compiles the same source and rebuilds its own frames. It works for djs because
the program never changes across an upgrade, only the engine.

Prototype in scripts/snapshot-research/portable, with await-sites.mjs for the
static coverage:
- It exports on main before 0080 and imports on today's main, 406 commits apart, and
  the other way around.
- All 1,057 tried idle points of the 300 harvested codemode cases transfer in both
  directions: 1,008 continue exactly, 49 continue with the importing build's own
  behaviour where the builds differ.

Static coverage of 2,535 real scripts: every await is describable with ten operand roles
plus for-of iterator records. Statement-level awaits alone would cover half.

Costs: 4–6 weeks of engine work, then a contract on lowering at awaits, on builtin
record schemas and on format readers.

Hard limits:
- quiescent points only;
- the new engine's behaviour applies after the restore;
- no image identity or canonical hash across builds;
- stack traces travel as text.

Recommendation: replay stays the durable record now (0140). Add a suspension-map pin
test today. Build phase 1 when sessions outgrow replay or hosts keep VMs across
releases.

Known gaps of the prototype:
- roles are derived from bytecode producers, not compiler-emitted, so they would not
  notice a callee/`this` swap;
- no generators, symbols, Proxy, pi lockdown patches or reference stages;
- JSON instead of a binary format;
- native dev-build timings only.
