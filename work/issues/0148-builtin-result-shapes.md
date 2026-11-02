# 0148: Order-independent shapes for objects that builtins create

status: done
agent: heap
branch: issue/0148-builtin-result-shapes
depends-on: 0147
guide: docs/guide/determinism.md, snapshots.md (the session-image guarantee)
touches: crates/djs-core/src/heap/shape.rs, crates/djs-core/src/vm/ (prepare_base_image warm-up), crates/djs-prelude/

## Goal
0147 lands with the guarantee "builtins leave no trace in the prefix; shapes of objects
created by builtin calls are session state, ordered by first creation". The owner's goal is
that the order and time of builtin use don't matter for the session image. Objects that
builtins create (iterator results {value, done}, RegExp match results, property
descriptors, ...) get their shapes in the session region in creation order (shapes never
die, D1831), so `new Map(..)` and `/a/.exec()` leave order-dependent shapes.

Steps, smallest first:
1. Pre-create at base build the shapes that builtins produce (from a survey of builtin
   result objects plus the harvested corpus), so builtin calls use prefix shapes.
2. Only if needed: transitions from prefix shapes to new session shapes kept in a per-VM
   side table (back to one shared root shape); never append descriptors in place to a
   prefix descriptor array.
Measure how often scripts extend builtin-created objects (harvested corpus) before doing 2.

## Acceptance criteria
- [x] Order-independence test extended: runs using Map/Set/RegExp/iterators/descriptors in
      different orders give identical session images and canonical hashes.
- [x] Instructions (±1%), module size, base image size measured. props +1.4% and
      +648 B gzip recorded as trades (D2067, coordinator: option A).
- [x] Guarantee restated in the guide.

## Log
- Survey (a temporary ignored test: each call in a fresh VM from the base, the session
  image's growth, new shapes by key chain, new cells by kind, new strings). Besides
  iterator and match results, builtins and the engine create: descriptors (data and
  accessor), function length/name/prototype (W and not), constructor, RegExp lastIndex,
  Error message/cause/errors, allSettled status/value|reason, withResolvers,
  Proxy.revocable, template raw. Plus non-shape residue: the job queue, the rejection
  list, the host inbox (allocated on first use), `prepareStackTrace` (interned by every
  Error), the FinalizationRegistry list.
- Step 1 alone suffices: the per-VM root shape is created in the base with those chains,
  and its tree goes to the base's tail, so extending a builtin-created object writes the
  tail, never the prefix: no side table (step 2) is needed. Corpus check (a scratch hook
  in difftest): 2 of 310 codemode + harvested cases extend a pre-created shape at all
  (parents `value`, `status`: user literals most likely). Corpus shape use: function
  shapes 310, descriptors 306, iterator results 306, allSettled 83/10, Error message 49,
  lastIndex 15.
- Queues allocated in the tail, slots `undefined` (an emptied queue then equals an unused
  one), emptied instead of nulled. `prepareStackTrace`/`stackTraceLimit` pinned in the
  base; interning them after compact_for_image regrew the atom entries (+4.5 KB raw), so
  they are interned first. Parser names are not pinned (a session's first compile
  interns them first anyway).
- Order test extended to 23 calls; passes reversed and on a subset. The subset check first
  failed on queue slots (zero bits unused vs `undefined` after use): fixed by the fill.
- Measured (wasm release vs main ae8771c7): module 458,304 → 458,952 B gzip (+648); base
  322,376 → 325,008 B raw (+2.6 KB; per VM). Instructions, 22 workloads: within ±0.9%
  except props +1.4% (10 runs × 2; identical binaries vary ±0.7% on props). Cause: the
  root shape's 13 extra children, searched by `shape_without` on every delete; a
  `{value,done}`-only variant is at main's level. A one-entry transition cache outside
  the image: props +0.8% but +334 B gzip code: reverted (patch in scratch). Coordinator:
  keep all chains, record both trades, raise the ceiling (D2067, size.tsv 459,164).
- Separate merge 685e6736 (D024, D2400): one snapshot container format, session images
  only; readers of DJSN 1, DJSS, DJPI and raw images in containers removed.
- Checked: djs-core lib, djs-prelude, djs-cli, e2e plain, js tests (59), native-vs-wasm,
  difftest codemode + harvest with --determinism --gc-stress --suspend-every 97 (armed),
  test262 Promise, RegExp, getOwnPropertyDescriptor(s), Error, NativeErrors,
  AggregateError, Proxy/revocable, FinalizationRegistry, Iterator, class, Function.
- Guide: determinism.md (the guarantee restated), snapshots.md (tail contents).
- Per call, the session image's growth after a bare run, alone: 0 B for Map/Set get/has,
  exec/match/matchAll//d, iterators, getOwnPropertyDescriptor (data and accessor),
  allSettled/all/withResolvers/revocable, Error/AggregateError, generators, String.raw,
  new RegExp. What remains is the script's own: identifiers (`x => x` +24, a catch
  binding), `(class {})` +32 (`<fields 0>`), data-keyed results
  (`getOwnPropertyDescriptors(Map.prototype)` +1032), FinalizationRegistry +80.

## Follow-ups
- props +1.4% (D2067): a cheaper delete transition without code growth, e.g. a
  key-sorted transition index on the root, or the last delete transition per shape cached
  in the image's per-VM tail.
- FinalizationRegistry's list is still allocated on first use (rare in codemode).
- If 0149's slim build drops builtin families, drop their chains from
  `BUILTIN_RESULT_SHAPES` with them.

## Result
- Using builtins, in any order, adds nothing to the session image: the shapes of objects
  builtins create are in every VM's shape tree from the base (`vm::image::
  BUILTIN_RESULT_SHAPES`), the engine's queues are pre-allocated in the tail, and the
  Error hooks' names are in the base (D2067). Test
  `touching_builtins_in_any_order_leaves_the_same_session` (23 calls, reversed, subset).
- Known gaps: data-keyed results, identifiers and class field names stay the script's
  own; FinalizationRegistry's list; queues past their base capacity. Trades: +648 B gzip
  (ceiling 459,164) and props +1.4%.
