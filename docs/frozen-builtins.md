# Frozen builtins and a session-only heap (0144)

Status: design and prototype, for the owner. Nothing is built on main. The prototypes are
patches in work/scratch/0144 (see "Reproducing"). This replaces docs/empty-heap.md's
steps 2–4 and docs/shared-segment.md (0138).

The owner's direction:
- "We want to bring down the overall cost of heap snapshots. Putting all builtins eagerly
  into the heap is useful for determinism, but the order in which these objects are
  created, and when, should not necessarily matter."
- "What if djs gains an entire mode where all builtins start out frozen? That would
  greatly simplify things."

## The answer in short

- **Frozen from birth works, and it is cheap.** A prototype freezes every builtin when
  the base image is built, and fixes the override mistake in `[[Set]]` (two places). pi's
  prelude then runs without its lockdown:

  | pi session start (prelude run and sealed, native) | today | frozen base, no lockdown |
  | --- | ---: | ---: |
  | time | 3.07–3.19 ms | 0.48 ms |
  | image | 479.9 KB, 8,330 cells | 422.4 KB, 7,556 cells |
  | with a born-frozen representation (est., 4.2) | | ~375 KB, ~6,350 cells |

  Scripts see ordinary semantics, checked in the prototype:
  - `this.name = "E"` in an Error subclass, and `obj.toString = f`, define own
    properties;
  - assigning to a builtin itself (`Array.prototype.push = 1`, `Math.PI = 3`) and adding
    to one throw `TypeError` in strict code;
  - `globalThis` stays extensible.
- **"Outside the arena" should not mean a second address space.** If a heap reference
  can point either into the session arena or into a builtin space, every heap read
  checks which. The prototype puts that check into `Arena::read`/`write`/`slice` and the
  hot loop's loads. It costs **+3.3% to +6.2% wasm instructions on every workload** (pi-like +3.3%,
  props +6.1%, json +6.2%) and +11.8 KB of code, before anything reads from the second
  space. The code-only variants of 0138 cost +0.3–2.4%. Both rule out a separate
  builtin space as the runtime representation.
- **The owner's goal follows from freezing, without laziness.** Frozen builtins never
  change, so they can be an **immutable prefix** of every arena: the base image at the
  same offsets in every VM of a build. The session heap is everything above it.
  - Nothing about builtins is created at run time, so order and time cannot matter.
  - Snapshots, `canonical_hash`, the collector's work and determinism are defined over
    the session region only.
  - Session cells reference builtins by build-fixed offsets: free to read, and
    translatable to stable registry ids for exports.
- **What is not frozen today has to move out of the prefix.** An instrumented run of
  all 300 harvested codemode cases recorded every write into the base region (table in
  section 3). The only writes are:
  - the value stack, the interpreter's registers, roots, jobs and handles;
  - the atom table's index and counters;
  - lazily computed string hashes (8 strings);
  - prototype flags in 7–10 object headers;
  - descriptor counts of 3 shapes;
  - the global object's property table and one realm slot.

  No writes went into functions, templates, bytecode or pools.
- **Laziness is a memory optimization, and it conflicts with the goal.**
  - Builtins created lazily *inside* the session arena land in touch order, so the
    layout depends on it.
  - Created *outside* it, they cost the dual-space check.
  - Created at *fixed offsets inside a reserved prefix*, in wasm they still occupy the
    whole reserved range (memory grows only at the end and is counted whole).

  Not recommended: the snapshot goal doesn't need it. The prefix is a 322 KB copy at
  `vm_new`, ~4 µs, or decoded straight into the arena since 0139.
- **Snapshots:** a pi session's image shrinks from ~480 KB plus its data to ~53 KB (the
  prelude's own state) plus its data. Compressed containers stay about the same size
  (2–8 KB for waiting sessions), since they are already deltas against the prelude image.
  A host that stores raw images, or the Sandbox in a browser without zstd, gets the
  whole difference.

**Recommendation:** build the frozen mode as a module variant for pi, in three phases
(section 8): ~5–7 weeks, medium risk. It replaces:
- 0143's step 2 (engine lockdown), now measured;
- step 3 (static atoms), subsumed by the immutable prefix with session atoms above it;
- step 4 (lazy realm), dropped;
- 0138 (shared segment), dropped: two address spaces cost +3–6%.

Keep 0143's step 1, the builtin registry, for exports (tier 2, 0142).

## 1. Prototype

All on a scratch branch, native release and wasm. Nothing on main.
1. **Frozen base** (`freeze-all.js` run before `prepare_base_image`): pi's lockdown walk,
   from the globals and the hidden intrinsic prototypes, with `Object.freeze` on every
   object and no override accessors.
2. **Override-mistake fix** (`frozen-proto.patch`): in `Exec::ordinary_set` and
   `Exec::set_property`, an inherited non-writable data property whose holder lies in the
   frozen base does not stop the assignment; it falls through to defining an own property
   on the receiver. The prototype's mode switch is a global `FROZEN_END` (the end of the
   frozen base); a real build would make it the prefix boundary.
3. **pi's prelude** (the tests/pi copy) run on the normal base as today, and without its
   `lockdown()` on the frozen base, then sealed.
4. **Dual address space** (`dual-dispatch.patch`): reads at offsets ≥ 2^31 go to a second
   byte slice, in `Arena::read`/`write`/`slice` and the hot loop's `ld`/`ld32`/`window`.
   It measures the dispatch alone: no data lives there.
5. **Write audit** (`audit.py`): every arena write and hot-loop store below the base
   image's end is counted per word, from VM creation until the first collection (which
   can move cells). Run over the 300 harvested codemode cases and the codemode corpus,
   then attributed to cells and fields.

Not prototyped: lazy materialization of the Array family by registry id. The dual-space
number (+3–6% on all reads) and the order argument (section 6) settled the question it was
meant to answer. Building it would have measured a mechanism the design doesn't use.

## 2. Reference representation and access cost

| variant | how a session cell names a builtin | cost, wasm instructions (current main as base) |
| --- | --- | --- |
| **immutable prefix (recommended)** | an ordinary offset below the prefix end B, the same in every VM of a build | none |
| separate builtin space, any reference | offsets ≥ 2^31; every heap read and hot load dispatches | strbuild +5.5%, higher-order +5.0%, arrays +4.4%, json +6.2%, jsonh +5.9%, forof +5.8%, pi-like +3.3%, props +6.1%, strings +4.8%, loop +5.3%, calls +4.4%; +11.8 KB code |
| separate space for code only, eager (0138) | templates and bytecode | +0.1–2.4% |
| separate space for code only, copied on first call (0138) | only frame entry dispatches | +0–0.9% |
| stable ids with a per-VM table (0143 §5.4) | an id; an indirection per builtin access, plus a second object kind on every property path | not measured: 0143 estimated 3–4 months and a cost on method calls |

The immutable prefix needs no new reference kind:
- Inline caches keyed on builtin holders and shapes keep working: the offsets never
  change within a VM and are the same in all VMs.
- For exports, prefix offsets map to registry ids through a table generated at build (0143
  step 1). That is where "reference by stable id" belongs (0142 tier 2).

## 3. State that is not frozen today, and where it goes

Writes into the base region, harvested corpus (300 runs, counted until the first
collection):

| what was written | words | writes | where it goes in the mode |
| --- | ---: | ---: | --- |
| value stack (a base cell) | 254 | 1,156,339 | session region: a fresh stack per VM |
| atom table: hash index and `STRINGS` count | 45 | 57,112 | base atoms stay in the prefix, read-only (entries, index, strings); new atoms go to a per-VM table above it. Interning probes the base index, then the session's. `atom_entry(id)`: `id < BASE_ATOMS` picks the table |
| `Interp` registers, `Roots` fields (jobs, handles, shape counter), the job queue | ~15 | ~60,000 | session region: roots, interp, jobs, handles are per VM by nature |
| string hash caches (`Str8` HASH, 8 strings) | 8 | 1,136 | computed at build for every base string |
| object header flags (7 ordinary objects, Array/String/Number prototypes, a builtin closure) | 11 | ~3,300 | the prototype flag (D1621's `OBJ_IS_PROTO`), set at build for every builtin |
| shape descriptor counts (3 shapes) | 3 | 4,663 | two shape trees: the builtins' (frozen, in the prefix) and the session's, rooted at a per-VM root shape. A session object never extends a builtin's shape |
| the global object's property table | 1 | 300 | the global object goes above the prefix (~2.2 KB copied per VM). Its builtin bindings are frozen data properties pointing into the prefix |
| one realm slot (`Realm.INTRINSICS`) | 1 | 300 | the realm's mutable slots (clock, PRNG seed, registries, RegExp statics, `Symbol.for` registry) move to a per-VM realm cell |

Not seen in the corpus but possible, with where they go:
- **Inline caches of builtin templates:** privileged code fills none today. If it did,
  per-VM cache tables (0138 §2).
- **Collections and promises created by builtins at run time:** ordinary session cells.
- **WeakMaps keyed by builtins:** the key is in the prefix, which is always live, so
  nothing changes.
- **Debug build:** a write barrier assertion (`off < B` in `Arena::write`) catches any
  write that misses this list.

## 4. The override fix

### 4.1 Semantics

The mode's `[[Set]]` (OrdinarySet, 10.1.9.2), for an assignment `receiver[key] = v`:
- If the property found on the chain is a data property that is **not writable**, its
  holder is **a builtin of the frozen prefix**, and the holder is **not the receiver**,
  proceed as if it were writable. The receiver gets an own data property (writable,
  enumerable, configurable), or fails as usual if the receiver itself is not extensible
  or has a conflicting own property.
- Everything else is unchanged:
  - assigning to the builtin itself fails, a TypeError in strict code;
  - accessors call their setter;
  - proxies take over;
  - `defineProperty`, `Reflect.set` with a builtin receiver and freezing behave as
    specified.

This is the semantics of Node's `--frozen-intrinsics` and of SES's override taming, but
for every property, not a list. pi's lockdown emulates it for 7 keys and all of
`Object.prototype` with ~680 accessor closures, and those accessors then show up in
`getOwnPropertyDescriptor`. Under the mode, the builtins' properties stay data
properties, non-writable and non-configurable.

### 4.2 Representation

The prototype froze with `Object.freeze`, which converts shape-mode objects into
dictionaries:
- 513 objects get property tables, +30 KB;
- 563 new shapes and their transition lists, +22 KB;
- PropTable indexes, +12 KB;
- minus the shape-mode values, −17 KB.

In all, the frozen base is 369.5 KB and 6,981 cells against 322.4 KB and 5,770.

A born-frozen representation avoids that: a frozen bit on the object (non-extensible
plus all own properties non-writable and non-configurable) that property reads already
consult with the flags they load. Freezing at build then costs nothing, and the frozen
base stays ~322 KB.

### 4.3 Spec deviations and test262

- **The deviation:** the override fix is one. test262 expects a TypeError when
  assigning over an inherited non-writable property. That case doesn't arise in the
  default mode, because builtins aren't frozen there.
- **The mode is opt-in,** a module build for pi:
  - test262, the difftest corpus and the Sandbox keep running the default module;
  - the frozen module gets its own test set: the override semantics, pi's tests, and
    difftest cases run without pi's lockdown.
- **Frozen builtins also change ordinary results.** `Object.isFrozen(Array.prototype)`
  is true, and scripts that patch builtins fail, exactly as under pi's lockdown today.

## 5. GC, snapshots, determinism

- **Collector:**
  - the prefix is never marked, walked or moved;
  - full collections compact from B up (the D2041 machinery with B as a floor that
    never moves);
  - the old region (pi's sealed prelude) sits above B, as now;
  - since the prefix holds no reference into the session region (section 3, enforced in
    debug), the old-region walk drops the 322 KB base entirely. Today that is about 1%
    of instructions after D2055.
- **Images:**
  - a session image is the arena from B, plus the build hash that fixes the prefix;
  - restore = `vm_new` (the prefix) + the session bytes at B;
  - offsets are unchanged, so no fixups;
  - containers compress against the prelude image, as today.
- **`canonical_hash`** is Cheney over the session region from the roots, leaving prefix
  references as they are (build-fixed).
- **Determinism and order:**
  - nothing about builtins is created at run time, so two runs that touch builtins in
    any order leave identical session regions, given the same session history;
  - this holds by construction; there is no lazy state to compare;
  - the existing gate checks (raw images across slicing and restore, native vs wasm)
    carry over unchanged.

## 6. Why not lazy

Laziness was the means to the owner's end. The end, creation order and time not
mattering, comes from freezing plus a fixed prefix. Laziness only buys memory, and each
form of it conflicts with something:

| where lazily created builtins live | order-independent? | access cost | memory per VM in wasm |
| --- | --- | --- | --- |
| in the session arena (0143 step 4) | no: they land in touch order | ≤0.5% (est.) | touched builtins only (~17 KB median) |
| a separate space (any reference) | yes | +3.3–6.2% (measured) | touched builtins only |
| reserved fixed offsets in the prefix, filled on first touch | yes | a materialization check on every access to an unfilled holder (est. ≤0.5%) | the whole reserved range anyway: memory grows only at its end and the page count is what a run costs |
| **eager immutable prefix (recommended)** | yes | none | 322 KB (the base), copied or decoded once at `vm_new` (~4 µs) |

If per-VM memory becomes the target later, the lever is the prefix's size, not
laziness. Families no codemode run touches (typed arrays, iterator helpers, Proxy,
generators, ~127 KB per 0143) can be left out of pi's frozen module, or loaded as a second
immutable chunk on first use with the same fixed-offset argument.

## 7. pi under the mode

Measured natively with the tests/pi prelude, two tools, no globals:

| | today | frozen base (via `Object.freeze`) | frozen, born-frozen representation (est.) |
| --- | ---: | ---: | ---: |
| base image | 322.4 KB / 5,770 cells | 369.5 KB / 6,981 | ~322 KB / ~5,770 |
| prelude run (start, seal) | 3.07–3.19 ms | 0.48 ms | ~0.45 ms |
| sealed start image | 479.9 KB / 8,330 | 422.4 KB / 7,556 | ~375 KB / ~6,350 |
| the session's own part of it | 157.5 KB (lockdown + prelude) | 52.9 KB (prelude) | ~53 KB |

- **What pi changes:** it drops `lockdown()` from its prelude (pi-private, the owner's
  call; not changed here) and loads the frozen module.
- **Time:** the prelude run is 6–7× cheaper. That is pi's cold start, and also tier 2's
  import cost (0142: 2.6 of 3.0 ms was re-running the prelude).
- **Snapshots:**
  - raw images lose the lockdown's ~105 KB, and with phase 2 the 322 KB prefix too;
  - containers against the prelude image stay at 2–8 KB for waiting sessions, since
    they were already deltas;
  - tier 2 exports name builtins by registry id, through the prefix offset table.

## 8. Revised plan (replaces 0143 steps 2–4 and 0138)

| phase | what | effort | risk | gives |
| --- | --- | --- | --- | --- |
| 0 (0143 step 1, unchanged) | stable builtin registry: bgen-generated, append-only, gate-checked; a build table from prefix offsets to registry ids | 1 week | low | names for exports and relocations |
| 1. frozen module for pi | a djs-wasm build feature whose base is frozen at build; a born-frozen object representation (no dictionary conversion); the override fix in `ordinary_set`/`set_property` and the put-cache paths; tests for the override semantics; pi's tests and difftest without lockdown on that module | 2–3 weeks | low–medium | pi start 3.1 → ~0.45 ms, start image 480 → ~375 KB, no accessor pairs |
| 2. immutable prefix | per-VM state out of the base (section 3: stack, interp, roots, jobs, handles, realm slots, global object, session atom table, session shape root); string hashes and prototype flags at build; a debug write barrier; the collector skips the prefix; images = session region; `canonical_hash` over it | 3–4 weeks | medium (atoms, shapes and the realm are central; the write audit and the barrier bound it) | snapshots and dumps hold only the session; old walk and full collections skip 322 KB; order-independence by construction |
| (dropped) static builtin atoms (0143 step 3) | subsumed: base atoms are part of the immutable prefix | – | – | – |
| (dropped) lazy realm (0143 step 4) | conflicts with order-independence or costs +3–6% (section 6) | – | – | – |
| (dropped) shared segment (0138) | a second address space: +3.3–6.2% on every heap read | – | – | – |
| later, if memory per VM matters | trim pi's frozen module of families no codemode run touches (~127 KB), or load them as a second immutable chunk | 1–2 weeks | low | ~−127 KB per pi VM |

**Total:** phases 0–2, ~6–8 weeks.

**Risks:**
- The default module must stay spec-conformant, so every change in phases 1–2 needs a
  test in both modes.
- Phase 2 touches the atom table, the shape tree's root and the realm layout. The
  layout hash and the JS reader change with it (regenerated as usual).
- The debug write barrier makes a missed write path a test failure, not a corruption.

## Reproducing

The patches apply to main as of 0144's branch point (work/scratch/0144, gitignored).
```sh
# frozen base, override fix, pi start (native)
git apply work/scratch/0144/frozen-proto.patch
P0144=$PWD/work/scratch/0144 cargo test --release -p djs-core --lib proto_frozen -- --ignored --nocapture
# dual address space: instructions per workload
git apply work/scratch/0144/dual-dispatch.patch && scripts/build-wasm.sh
python3 scripts/perf/wmeasure.py -n 3 --noqjs --wasm BASE.wasm --wasm DUAL.wasm pi-like props json ...
# write audit over the harvested corpus
python3 work/scratch/0144/audit.py . && cargo build --release -p djs-difftest
for f in tests/corpus/codemode-harvest/*.js; do
  DJS_WATCH_OUT=watch.txt target/release/djs-difftest --engine djs -j 1 codemode-harvest/$(basename $f); done
```
`prelude-nolock.js` and `freeze-all.js` are tests/pi's prelude split at its `lockdown()`
(the latter without the override accessors).
