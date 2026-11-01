# 0088: Snapshot design research: long-lived, cross-version, small (report only)

status: done
agent: snapshot
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
- [x] An inventory of version-specific vs durable state, checked against the code.
- [x] 2–4 concrete candidate designs, each with: the format sketch, what it can and
      can't represent (and when a restore must fail), restore cost, size on realistic
      codemode heaps (measure: take images of real scripts, estimate or prototype the
      alternative encoding in scratch), version-evolution story, effect on determinism
      (D1090, D1730), implementation cost, and what it would need from the compiler
      (stable ids for functions and suspension points).
- [x] A comparison table against today's raw images, and a recommendation (including "keep
      raw images for in-process/short-term, add X for long-term").
- [ ] Optionally a design note draft in docs/ (clearly marked "not adopted"). Not written:
      the format sketch of the logical design is under "Candidate designs" C.

## Notes
- Read ARCH.md (Snapshots, Deltas, Host Interface), docs/host.md (snapshots, journals,
  pi codemode), docs/compiler.md 7 (frames, suspension), docs/objects.md, and the
  decisions D1320–D1330, D1600–D1601, D1670, D1680, D1730, D1740, D1750.
- Own worktree. The machine is shared: small prototypes only, and follow AGENTS.md
  "Testing while iterating".

## Log

- Read ARCH (Core Idea, Heap, Snapshots, Deltas, Host Interface), docs/host.md, docs/compiler.md 6–7,
  docs/objects.md, D1320–D1330, D1431, D1495, D1600/D1601, D1670, D1680, D1720, D1730, D1740,
  D1750, pi's `vm.ts`/`execution.ts` (read-only) and js/pi/.
- Built a measurement harness, **snaplab** (work/scratch/0088/snaplab, a scratch crate outside the
  workspace over djs-core + djs-prelude; not committed, scratch is gitignored). It is a Rust copy
  of js/pi/vm.ts's session logic in deterministic mode: the base image, pi's host globals and
  prelude (tests/pi/prelude-source.ts), a codemode script, fake tools answered in call order with
  `settle` + `stalled` exactly as the adapter does. At **every idle point** (the VM waits on tool
  results: pi's natural durability point) it takes the image and measures:
  - raw image (as `VmSession.snapshot()` takes it today, uncollected) and collected;
  - zstd -3/-19, gzip -9, brotli -q 11, xz -9 of the collected image;
  - byte deltas: `zstd -19 --patch-from` against the image right after pi's prelude ("post-prelude",
    what the adapter caches, D1495) and against the base, and against the previous idle image;
  - a simulated **ARCH "Deltas"** old generation: a parallel BFS matches the image's cells to a
    reference image's, then new cells + 4 KiB pages dirtied by changed reference cells;
  - a prototype **logical encoder** (logical.rs, encoder only): the object graph as records, with
    cells of a reference world named by path, templates as (script, span), strings deduplicated
    by content, bytecode/ICs/hash indexes/atoms/regexp programs left out;
  - the **journal** (code + delivered results, the D1730/pi replay record);
  - restore times: `Vm::from_bytes` (verified) vs replaying the journal from scratch / from the
    prelude image;
  - suspended activations (frames, resume op, operand stacks) and builtin-internal objects.
- Six scripts (work/scratch/0088/scripts): `ask` (human in the loop), `tool-heavy` (pi.bench's
  TOOL_HEAVY), `sequential` (await in a for-of), `big-log` (a 1.5 MB tool result kept in a
  variable), `records` (1500 issue records, a class, Maps, `Promise.all` of async helpers),
  `agent-loop` (retry wrapper with try/catch/finally, an async generator, `for await`, awaits
  inside expressions). Tool results are real source files and real prose from this repo, so
  compression ratios are realistic (a first run with synthetic repetitive text gave 60x journal
  compression; real text gives 4–5x).
- Numbers: native, djs-core at opt-level 1 (dev), shared machine: use restore times as ratios.
  Reproduce: `cd work/scratch/0088/snaplab && cargo build && scripts/timeout.sh 900
  target/debug/snaplab ../out ../scripts/*.js` (report.txt, points.csv).

## Findings

### 1. Inventory: what is version-specific in an image, what is durable

Checked against heap/cells.rs, heap/object.rs (ObjectClass), interp/run/{suspend,promise}.rs,
interp/realm.rs, bytecode/template.rs and the images above. "Churn" counts commits on main that
touched the file (697 commits in total; **260 of them change `BUILD_HASH`**, D1330, i.e. every
merge that touches djs-core/src or the builtins invalidates every image).

| state | where | why version-specific | durable meaning | at a quiescent point? |
| --- | --- | --- | --- | --- |
| cell headers, kinds, field offsets, flag bits | every cell (cells.rs, churn 25) | LAYOUT_HASH | none: representation | yes |
| atom ids | PropEntry KEY, bytecode immediates, NAMES, symbol atoms | interning order, well-known list | the key (string or symbol identity) | yes |
| bytecode, consts, handler/line tables, capture descriptors, IC numbering | FuncTemplate parts (ops.rs churn 15) | compiler + op table | user code: Script SOURCE is kept (D009) + the function's source span (SRC_START/END); the prelude's source is stripped (D1340): it is the engine | yes |
| suspended frames | Generator slots 1–3: the frame ValueArray (callee, this, args, header ints prev_bp/ret_pc/argc/flags/new_target, locals, operands), resume pc, hp | local slot assignment, header encoding, pcs, **operand stack shape** (for-of `it next` and D1800's `iter_next`, finally value + return-address pcs D1084, pending call operands), hidden locals (`<state>`, `<fields N>`, private-name bindings) | function + suspension point + bindings by name/scope + semantic operand slots | **yes: every pi session has ≥ 1** (the body awaits a tool) |
| realm slots | Realm::INTRINSICS by `Intrinsic` order, INSTALLED (realm.rs churn 25) | enum order | named builtins (`%Array.prototype%`) | yes |
| builtin closures | BuiltinClosure slot 0 = INTRINSICS index; closure kinds 20–24 (D1650) | table index | spec abstract closures by name | yes: pi's pending map holds a `PromiseResolveFunction`/`PromiseRejectFunction` pair per pending call |
| builtin records | Record slots (D1372) laid out by builtins/*.ts (promise.ts churn 6) | builtin implementation | spec fields (`[[Values]]`, `[[RemainingElements]]`, async-generator requests) | yes: `Promise.all` leaves a 5-slot Record + `PromiseAllResolveElement` per element |
| class internal slots | Promise, Map/Set (+ hash index), iterators, RegExp (compiled program), typed arrays, Generator, Proxy, Date, Error | ObjectClass numbers, slot conventions | spec internal slots | yes |
| hash indexes | PropTable INDEX, Map/Set index, atom index | hash function, load factor | none: recomputable | yes |
| GC and cache state | GC_THRESHOLD (D1601), reserve flag (D1600), PROTO_EPOCH, IC slots (D1620/D1621), append-rope buffers (D1610) | collector/cache policy | none (OOM points are per build anyway) | yes |
| interpreter registers, run state, parked call, pending throw, fuel debt (D1740) | Interp cell | encoding | at quiescence: Idle, nothing parked, debt 0 | trivial |
| native frames (0063 state numbers, bgen slots), suspended intrinsics (regexp progress records, D1332) | value stack | generated code | none expressible | **never** (only mid-run) |
| job queue, host inbox (D1670) | Roots::JOBS, Interp::INBOX | entry layout | queued work | **empty** at Idle |
| pending async host calls | HostCall cells | fn ids are host-assigned | durable (the Sandbox header names functions) | yes for Sandbox async functions; pi: none (its bridge is sync) |
| handles | HandleTable | slot numbers | the host's named roots (pi's adapter keeps api/settle/stalled numbers in its session JSON) | yes |
| user data | objects, arrays, strings, Maps/Sets, typed arrays, Dates, Errors (`stack` is an eager string), symbols, BigInts | representation only | **fully durable** | yes |
| clock, `Math.random` state, time zone, weak-ref mode | Interp, RandomState, Roots FLAGS | xorshift64* is an algorithm choice | durable if the algorithm is (pi pins QuickJS-identical numbers) | yes |
| the realm itself after pi's lockdown | ~1130 builtin cells changed (frozen, accessors) | – | a deterministic function of (engine, prelude source, prelude inputs) | yes |

What is at a quiescent point in practice (every idle point of the six scripts): 1–5 suspended
activations (async functions, one async generator), 3–15 pending promises with reactions, the
resolve/reject closures of pi's pending calls, `Promise.all` records, ArrayIterators. No builtin
frames ever: the builtin language has no `await`, so builtins keep their state in records and
reactions, not frames. But **11 of 15 suspension points in `agent-loop` have operands below the
awaited value** (for-of / for-await state, `out.push(await …)`, finally slots), as do the
for-of loops in `sequential`, `big-log` and `records`. "Statement-level awaits only" is not a
restriction real codemode would meet.

### 2. Measurements

Selected idle points (sizes in KB; the first idle point of every script is within 5 KB of
`tool-heavy, start`; points.csv has all 144):

| point | pi `snapshot()` today (raw) | collected | zstd -19 | zstd vs post-prelude ¹ | old-gen delta vs post-prelude ² | logical vs post-prelude ³ raw / zstd | logical vs base ⁴ zstd | journal raw / zstd | restore: image / replay ⁵ (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ask, waiting on the human | 943 | 504 | 108 | **21.6** | 80 | 1.3 / **0.9** | 19.6 | 0.5 / 0.3 | 0.5 / 1.3–5.9 |
| tool-heavy, start | 945 | 505 | 109 | 25.5 | 81 | 1.7 / 1.0 | 19.7 | 1.0 / 0.5 | 0.5 / 1.4–5.9 |
| tool-heavy, after 61 results (60 files kept) | 1022 | 891 | 201 | 119 | 482 | 341 / 89 | 106 | 331 / 85 | 0.6 / 14–19 |
| sequential, after 41 results (data dropped) | 791 | 521 | 112 | 29.7 | 101 | 8.5 / **3.8** | 22.0 | 217 / 58 | 0.8 / 19–27 |
| big-log, 1.5 MB log kept | 4706 | 2205 | 418 | 334 | 1781 | 1660 / 306 | 323 | 1518 / 296 | 0.8 / 38–43 |
| records, after 20 results (1500 records kept) | 3768 | 1889 | 329 | 246 | 1465 | 652 / 167 | 185 | 789 / 151 | 2.7 / 14–20 |
| agent-loop, after 10 results | 705 | 515 | 111 | 27.1 | 91 | 3.7 / 2.2 | 20.6 | 11 / 2.4 | 0.5 / 2.6–7.1 |

¹ `zstd -19 --patch-from post-prelude.img` of the collected image. node 25's built-in zstd does
the same with `dictionary:`: 27 KB at level 19 (120 ms), **36 KB at level 3 (1.5 ms)**,
decompression 0.6 ms (ask).
² ARCH "Deltas" simulated: new cells + 4 KiB pages of the reference that changed cells dirty.
³ The prototype encoder against the post-prelude world (named by paths, recomputed on restore).
⁴ The same against the engine's base only: pi's prelude state is then in every snapshot.
⁵ `Vm::from_bytes` (verified) / replaying the journal from the prelude image – from scratch.

The reference images: base 402 KB (zstd 90 KB; 198 KB of it is the prelude's compiled code);
post-prelude 939 KB raw, 500 KB collected (zstd 108 KB; 44 KB as a zstd patch against the
base). pi's prelude state as a logical snapshot against the base: 49 KB raw, 19 KB zstd (726 new
units: 260 closures, 348 boxes, lockdown accessors; 552 patches: the 485 frozen builtin
functions). Recomputing the post-prelude image (base + host globals + prelude) takes 5.5–9 ms;
restoring a cached copy 0.5 ms verified, 0.01 ms trusted.

Other measurements:
- **pi's snapshots are uncollected.** `VmSession.snapshot()` and the prelude-image cache take
  `engine.image()` without a collection: 943 KB where 504 KB are live (docs/host.md still says
  450 KB per prelude image).
- **Byte deltas between consecutive snapshots of a moving heap are erratic**: 0.6 KB (big-log:
  the change is late in BFS order) to 25 KB (ask: one small change near the roots shifts every
  later offset) to 59 KB (records). Cheney order makes images canonical, not stable.
- **The old generation does not pay off for pi.** Against the base, pi's lockdown changes 1131
  cells on 62 of the 101 pages. Against the post-prelude image the floor is 80 KB: the atom
  table's entries and index (46 KB) are reallocated whenever a script adds an atom, and IC
  fills and handles dirty 19 pages. Content-defined compression (¹) beats it by 4x with no GC
  work.
- **The heap inflates data** ~2x over its JSON form (records: 788 KB of JSON results become
  1.4 MB of new cells). Dictionary-compressed images stay 1.1–1.5x over compressed logical
  snapshots (zstd alone: 1.4–2.3x).
- **Logical vs journal:** equal when the script keeps everything it was given (tool-heavy,
  big-log, records: ±15%); the logical snapshot is 15x smaller when the script reduces its inputs
  (sequential: 3.8 vs 58 KB). A journal can only be smaller when the state is derived from small
  inputs.
- **Continuations are tiny**: the frames, promises, reactions and builtin closures of a logical
  snapshot are 100–500 bytes. The source text of the script (0.5–1.4 KB) is bigger.
- Builtin internals at idle points, all scripts: `PromiseResolveFunction`,
  `PromiseRejectFunction`, `PromiseAllResolveElement`, `FunctionPrototypeBehavior` (base), 1-,
  3- and 5-slot Records (Promise.all, async-generator request), ArrayIterator, Generator kinds
  1 and 2. A spec-named schema for ~6 abstract closures/records covers codemode.
- Binding names are recorded for user templates (`NAMES`), but are not unique (`Bucket` twice:
  the class's outer and inner binding) and include compiler-hidden bindings (`<state>`,
  `<fields 0>`): a version-independent environment needs (name, declaring scope) and stable
  names for hidden bindings.

### 3. Key insights

1. **The bulk of every snapshot is recomputable.** For a session waiting on a human, 500 of
   504 KB are the base realm plus pi's prelude, and both are a deterministic function of
   (engine build, prelude source, prelude inputs). A snapshot only needs a *descriptor* of that
   reference world plus what differs from it: as bytes (a zstd dictionary: 22–30 KB) or as
   records (a logical snapshot: 1–4 KB). Neither the base nor the post-prelude image needs to be
   stored with a snapshot.
2. **Two different problems hide in "long-lived".** (a) Small: solved by (1) for any design.
   (b) Cross-version: only the *continuations* (frames, operand stacks, builtin closures and
   records) are hard; user data and the realm are easy to name version-independently. The
   continuations are 100–500 bytes of every snapshot and all of its compatibility risk.
3. **pi already has a version-tolerant durability mechanism:** deterministic replay (`start()`,
   `deliver(seq, now)`, D1730 journals). Its durable state *between* executions (`store`) is
   already logical JSON. Only in-flight executions need continuations, and they live seconds,
   or days when waiting on a human or a subagent. That is the window an engine upgrade must be
   survived in.
4. **Quiescent points remove most of the engine-internal state** (native frames, builtin frames,
   suspended intrinsics, parked calls, jobs, inbox, fuel debt) but not the continuations of
   user code, and those have non-trivial operand stacks in realistic scripts.
5. **Byte-level deltas of a copying-GC heap are fragile, record-level ones are not.** Identity
   by offset changes on every collection; identity by path or node number does not.

## Candidate designs

### A. Compact images + engine pinning ("image + its VM")

The image format stays. What changes is how it is stored and which engine restores it.
- **Format:** `{magic, version, engine build hash, wasm content hash, reference descriptor
  (base | prelude key: prelude hash + tools/globals/store hash + heap limit + deterministic
  flag), host function names, adapter state}` + the *collected* image, zstd-compressed with the
  reference image as a raw-content dictionary.
- **Restore:** look up the engine by build hash in an engine registry (the host keeps every
  djs-wasm build it has written snapshots with; ~1.2 MB each, 345 KB brotli, plus the adapter JS
  that matches its ABI), recompute or fetch the reference image (pi's prelude cache already does
  this), decompress (0.6 ms), `Vm::from_bytes` (0.5–3 ms).
- **Represents:** everything, at any instruction boundary (G5), not only quiescent points.
- **Must fail:** the engine build is not in the registry, or it is revoked (a sandbox bug fixed
  later: then fall back to B).
- **Size:** 22–30 KB for light sessions (vs 943 KB today), 119–334 KB for data-heavy ones. Two
  cheap steps give most of it: collect before `image()` (943 → 504 KB), then the dictionary.
- **Version story:** none needed; a session finishes on the engine it started on (Temporal's
  worker versioning, Hermes bytecode + its VM, Smalltalk image + VM). New sessions use the new
  engine.
- **Determinism:** unchanged; canonical hashes (D1730) stay valid.
- **Cost:** host only, days: collect before snapshots, dictionary compression (node built-in),
  an engine registry keyed by build hash, adapter/engine bundles loaded side by side (the ABI
  version check exists). Operations: keep and revoke engine builds. The engine needs nothing
  (the base image is readable through `vm_new` + the arena).
- **Compiler:** nothing.

### B. Journal replay (event sourcing), images as a cache

The durable artifact is the journal; images are a same-build cache in front of it.
- **Format:** D1730's journal (run: engine, code, params; sync and async results in delivery
  order with their clocks; checks; end) plus pi's inputs (prelude hash, tools, globals, store,
  seed, startedAt) and, for cross-version checks, a **semantic trace**: per idle point the calls
  issued (function name + args digest) and an output digest. Canonical heap hashes stay in it
  for same-build checks only.
- **Restore:** a same-build image (A) if there is one, then replay the tail; otherwise replay
  from the start on the current engine, verifying the trace at every idle point.
- **Represents:** whatever the engine can re-execute: everything.
- **Must fail:** the replay diverges (a call with other arguments, other output). Causes across
  versions: changed engine error messages or stack positions that reach a tool's arguments or
  the output, fuel accounting (interrupt budgets), OOM points, libm results (D1750), sort with
  inconsistent comparators, bug fixes. On divergence: restore with the pinned engine (A) or
  report "cannot resume across an engine change".
- **Size:** code + every result ever delivered: 0.3–2.4 KB compressed for light sessions,
  58–296 KB for heavy ones; larger than a logical snapshot when scripts reduce their inputs.
- **Restore cost:** re-execution, linear in history: 1.3–43 ms here natively; seconds for long
  compute-heavy sessions (the djs interpreter is ~4x slower than QuickJS on scripts). Bounded by
  same-build checkpoints, but not across versions.
- **Version story:** as robust as the semantics are stable, which djs pins to qjs (4 of ~222
  corpus cases differ). Divergence is detected, never silent.
- **Determinism:** needs deterministic mode (D1680); cross-version checks use the semantic
  trace, because canonical hashes are per build.
- **Cost:** about a week: trace digests in the journal and pi's idle reports, persistence
  policy, **golden journals** in the gate (recorded once, replayed by every later build; a
  failing one is a deliberate decision, like a ratchet), the fallback to A.
- **Compiler:** nothing.

### C. Logical snapshots (a versioned object-graph format)

The VM state as records in an engine-independent vocabulary, restored into a fresh realm of
any engine version that speaks the format. The prototype encoder measures its size.
- **Format (sketch):** header (format version, semantic level, reference world: "djs realm" +
  each prelude stage as (source hash, inputs)); a string table (WTF-8, deduplicated); a path
  trie for objects of the reference world (labels: property keys, `^` prototype, `~binding`
  for captured variables, `%Name` intrinsics, `$name` named handles); scripts (name, source,
  options); functions as (script, source span); then records in BFS order: objects (class,
  flags, prototype, shape = key list with attributes shared across objects, values, elements,
  internal slots by spec name), boxes, symbols, closures (function + environment as
  (binding id → box)), promises (state, result, handled, reactions), abstract closures and
  records by spec name with a schema version, activations (function, suspension ordinal,
  bindings by id, operand slots by stack map, result promise / request queue), Maps/Sets
  (entries), buffers; patches (path → new record) for changed reference objects; roots
  (named handles, pending host calls by function name, clock, PRNG state, time zone, weak-ref
  mode, unreported rejections). Left out: bytecode, ICs, shapes, hash indexes, atoms,
  compiled regexps, GC state: all recomputed.
- **Restore:** rebuild the reference world on the current engine (5.5–9 ms, or cached),
  resolve paths, compile the scripts, allocate and fill the nodes, rebuild each activation in
  the *current* compiler's frame layout from its stack map, apply patches, verify. Expected
  cost: about a JSON parse of the data (host.md: 10k objects ≈ 5 ms in wasm) plus the
  reference world.
- **Represents:** quiescent points only (Idle or between executions). Not representable:
  frames on the stack, native frames, suspended intrinsics. WeakRef/FinalizationRegistry only
  in the strong or absent modes (D1680).
- **Must fail:** a path that does not resolve; a function span without a function; a
  suspension point whose stack map differs from the recorded one; an unknown abstract-closure
  name or schema version without a migration; a prelude stage whose source is not available.
- **Size:** the smallest: **0.9–2.2 KB** compressed for light sessions, 89–306 KB for heavy
  ones (= compressed user data). Against the bare base (no recomputed prelude) +19 KB per
  snapshot.
- **Version story:** a format version with readers for the last N; migrations per abstract
  closure schema; a "lowering version" per suspension-point construct (await in for-of, for
  await, try/finally, call arguments, …); golden snapshots in the repo restored by every build
  in the gate. The pressure is real: ops.rs changed 15 times, for_in_of.rs 10, and D1800
  changed the for-of operand stack recently. Every such change would need a migration or a
  stack-map-compatible lowering.
- **Determinism:** script-visible behavior continues identically on the same build, but the
  image does not (allocation order, atom ids, GC threshold), so D1730's canonical hash after a
  restore differs. The logical encoding itself is the version-independent hash (hash the
  records). OOM points and fuel are per build.
- **Cost:** the largest by far. Encoder ~1.5k lines (prototype: 500), decoder ~2–3k, compiler
  metadata, frame materialization in the interpreter, builtin schemas, path resolution, gate
  tests: **6–10 weeks**, then a permanent compatibility tax on the compiler and builtins.
- **Compiler:** stable function identity (script hash + span: exists), binding identity (name +
  declaring scope; stable names for hidden bindings such as `<fields N>` and private names),
  suspension ordinals per function in source order (with sub-ordinals for the awaits the
  compiler adds for `for await` and async generator returns), and a semantic **stack map** per
  suspension point (slot kinds: for-of iterator + next, finally value + return point, pending
  call callee/this/args, binary operand, literal under construction, …). Today the operand stack
  is only described by `stack_depths`.

Two variants change the cost profile:
- **C0, full quiescence only** (no pending activations: between executions, a REPL, a
  persistent "kernel" session if pi ever keeps variables across turns). Data and closures only:
  no stack maps, no frame rebuilding, no builtin records except promises. ~2–3 weeks. For pi
  as it is, this point has nothing to save but `store`, which is JSON already.
- **C', continuations as data by a pinned source transform.** Rewrite async functions and
  generators into explicit state machines before compiling (regenerator, Kotlin coroutines,
  Racket's serializable continuations, Asyncify), with labels from source positions and
  operands spilled to named temporaries. Then every quiescent state is data + closures (C0),
  and the transform is versioned and kept forever, independent of the engine's compiler. Cost:
  an AST transform (outside the core: djs has no AST, G3), slower async code, stack traces
  through a source map, a fidelity risk. Worth considering before committing the compiler to
  stack-map compatibility.

### D. (measured, not proposed) ARCH "Deltas": a non-moving old generation + page diffs

Simulated in §2 (²): ≥ 80 KB against the post-prelude image, 345 KB against the base, because
pi's lockdown dirties 62% of the base's pages, the atom table is reallocated as it grows, and
IC fills touch template cells. It also needs a second collector mode (mark bits, D1090's
determinism across two spaces). Dictionary compression gets 22–30 KB with no engine change.

## Comparison

| | raw image today | A: compact + pinned | B: journal replay | C: logical | C0 / C' |
| --- | --- | --- | --- | --- | --- |
| light session (ask, agent-loop) | 943 KB | 22–27 KB | 0.3–2.4 KB | 0.9–2.2 KB | as C |
| data-heavy (records, big-log) | 3.8–4.7 MB | 246–334 KB | 151–296 KB | 167–306 KB | as C |
| restore | 0.5–3 ms | +0.6 ms decompress (+ reference: cached, or 5–9 ms) | 1–43 ms here, grows with history | ~ JSON parse of the data + reference | as C |
| snapshot points | any instruction | any instruction | idle points (replay reaches them) | quiescent | C0: between executions only |
| across engine versions | no | by keeping the old engine | yes, unless the trace diverges (detected) | yes, within the format's window; refuses on unmapped continuations | yes; C' independent of the compiler |
| determinism (D1090/D1730) | exact | exact | exact on the same build; trace checks across | behavior yes, image bytes no | as C |
| engine work | – | none | none | large (compiler, interpreter, builtins) | C0 medium, C' transform outside the core |
| host work | – | days | ~1 week | weeks | weeks |
| ongoing tax | none | storing/revoking engine builds | golden journals; behavior changes need a decision | every lowering/builtin change needs stack-map/schema compatibility | C0 small; C' keeps transforms |
| security after a fix | – | old bugs live on until revoked | runs on the fixed engine | fixed engine | fixed engine |

## Recommendation

- **Keep raw images** for in-process and same-build use (stress modes, prelude caches, fast
  resume). They are the only design that suspends anywhere (G5), and nothing restores faster.
- **Make them small now, at no engine cost:** collect before `snapshot()` and before caching the
  prelude image in js/pi/vm.ts (943 → 504 KB), and store snapshots zstd-compressed with the
  recomputable reference image as dictionary (→ 22–30 KB; node's built-in zstd: 36 KB in
  1.5 ms at level 3). The header carries the reference descriptor instead of the reference.
- **For long-term durability across engine versions, use B with A as the fast path:** the
  journal (pi's deterministic protocol, D1730) is the durable record; a same-build compact
  image is a cache in front of it; in-flight sessions across a deploy resume on their pinned
  engine (registry by build hash), and replay on the current engine with semantic-trace checks
  when the pinned engine is gone or revoked. This is what durable-execution systems converged
  on (Temporal: deterministic replay + worker versioning by build id). Golden journals in the
  gate make behavior changes that break replays visible and deliberate.
- **Do not build logical snapshots (C) now.** They are the only design that is small,
  cross-version and replay-free at once, but the size win over A+B is 20–25 KB per light
  session, and the price is a permanent compatibility contract on the compiler's lowering
  and the builtins' records, for the 100–500 bytes of continuation state. Revisit when sessions
  become long *and* compute-heavy (replay too slow) and pinning is unacceptable, or when pi
  keeps state across executions (then start with C0, which needs no continuations), or when
  snapshots must be inspected or migrated. If continuations are needed then, evaluate C' (a
  pinned source transform) before stack maps in the compiler.
- **Drop the old-generation delta plan from ARCH "Deltas"** (proposal for the coordinator):
  measured, it loses to dictionary compression by 4x and needs a second GC mode.

## Result

Report only, no engine changes. The issue file holds the inventory (§1), measurements on six
realistic pi codemode scripts at all 144 of their idle points (§2), the candidate designs A, B,
C (+ C0, C'), the measured ARCH delta D, the comparison and the recommendation. The
measurement harness is in work/scratch/0088 (gitignored, not committed): snaplab, the scripts,
the extracted pi prelude and the last report (`out/report.txt`, `out/points.csv`).

Known gaps: the logical encoder has no decoder (sizes only; path indexes and patch records are
approximate: patches store the whole changed object, not a property diff); restore and replay
times are native dev-build numbers, not wasm; no cross-version replay was run (an old djs build
would have to be compiled; divergence risk is argued from the corpus and the inventory).

## Follow-ups

- js/pi/vm.ts: collect before `snapshot()` and before caching the prelude image (both are
  ~940 KB uncollected vs ~500 KB live); docs/host.md's "450 KB per prelude image" is stale.
- js/pi/vm.ts + docs/host.md: snapshot compression with a reference-image dictionary and a
  reference descriptor in the header (design A, first half).
- An engine registry and side-by-side adapter/engine bundles for pinned restores (A, second
  half), with a revocation list.
- Semantic trace digests (call names + args, output) in D1730 journals and pi's idle reports,
  and golden journals replayed by every build in the gate (B).
- ARCH "Deltas": replace the old-generation plan with dictionary compression (or mark it
  superseded), if the coordinator agrees.
- If C ever becomes necessary: compiler suspension ordinals + stack maps and binding scope ids
  are the long pole; start with C0.
