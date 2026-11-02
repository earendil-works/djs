# Goldilocks snapshots: minimal, and a chance to survive updates

Status: decision basis for the owner (issue 0142). Nothing here is built in the engine.
It builds on docs/portable-snapshots.md (0141, "by meaning") and docs/empty-heap.md (0143,
builtin registry and engine-level lockdown). The numbers come from the prototype in
`scripts/snapshot-research/portable` (`goldi-*` commands), run on pi's real prelude, the
six snapshot-research cases and snaplab's fake tools. Times are native release builds:
use them as ratios.

The owner's question: *it's fine if snapshots eventually fail, but we want snapshots that
are minimal and have a chance to survive djs updates. Doesn't even changing globals
invalidate the snapshot between runs?*

## The answer in short

- **Today:** a snapshot is the image compressed against pi's prelude image. That
  reference is keyed by:
  - the djs build;
  - the prelude source;
  - the tools, globals and store;
  - the memory limit and the deterministic flag.

  So a restore needs the exact start data of the run. A changed tool description, a new
  global, another store value, any prelude edit or any djs build all mean "refused".
  - The memory limit does not change the image at all (measured), so it can leave the
    key at once.
  - Snapshots are per execution: a *new* run never restores. "Changing globals between
    runs" only bites a host that restores an old snapshot with *current* start data.
- **Recommended: two tiers and one cheap change underneath.**
  1. **Layered reference (cheap, now).** pi's prelude splits into a stage that depends
     on nothing per run and a `configure(tools, globals, store)` step. The reference is
     the stage-1 image, keyed by (build, prelude) only; the session carries its own
     inputs in its heap.
     - Changing tools, globals, store or the limit becomes a non-event for same-build
       restores.
     - Cost: +1.6 KB per snapshot with small inputs, +3.8 KB with realistic ones (18
       tools with descriptions).
     - A start costs +0.04 ms.
  2. **Tier 1, same build and same prelude version:** today's image delta, unchanged.
     It is exact, 0.2 ms native, and can be taken anywhere.
  3. **Tier 2, otherwise:** a by-meaning export of only the session's own state.
     - What travels as data: the script's frames and objects.
     - What travels as names: builtins by registry id or path, pi's prelude entry points
       by name (`tools`, `console`, `text`…), pending tool calls by id, the prelude's
       counters and maps by name.
     - It is restored onto a fresh prelude run of whatever version the importer has.
- **Prototype results for tier 2:**
  - Same build: 133 of 144 idle points round-trip. The other 11 are refused, cleanly:
    an async generator, which the prototype does not cover.
  - Every one of pi's 8 prelude versions onto the current one: 133/133 each, and back
    from the current to the first.
  - djs main before 0080 + the first prelude → today's djs + the current prelude, and
    the reverse: 133/133.
- **Sizes and times:** tier 2 is as small as today's deltas.

  | case | tier 2 | today's delta |
  | --- | ---: | ---: |
  | ask | 2.3 KB | 1.8 KB |
  | sequential | 5.3 KB | 6.3 KB |
  | tool-heavy | 45 KB | 48 KB |
  | records | 233 KB | 337 KB |

  The tier-2 sizes are JSON, zstd-compressed; a binary form would be smaller. Export
  takes 1.2 ms. Import takes 3.0 ms; 2.6 ms of that is re-running the prelude, mostly
  its lockdown. It drops to ~0.65 ms with a cached stage-1 image, or with engine-level
  lockdown (0143).
- **"Image with symbolic relocations"** (the owner's idea, section 4): it carries few
  names. A waiting session's own cells reference 5–9 builtin objects, 2 prelude objects,
  4 prelude boxes, 2 shapes, 2 templates, 20–57 atom ids and 2–6 intrinsic ids, plus
  ~20 changed reference cells.
  - It survives builtin, intrinsic, base-layout and prelude-internal changes.
  - It does not survive cell-layout, atom-table or opcode changes. Those touched 28–36%
    of djs's merges, so during development a 10-issue release kept it intact 8% of the
    time, and a 30-issue release never.
  - It is a better fit once the formats are frozen by policy. By meaning survives
    either way.
- **What pi must keep stable for tier 2:**
  - its global names, which were identical in all 8 versions;
  - `settle`/`run`/`stalled`, which changed once, additively;
  - the rule that prelude code never awaits, which it never has;
  - a small `state()`/`restore()`/`adopt(id)` contract (section 3). In 34 days of
    history, the interface had zero breaking changes and the source changed 8 times.

## 1. Layers: what the reference depends on

| layer | content | changes when | today | recommended |
| --- | --- | --- | --- | --- |
| L0 base | the djs realm | every djs build | in the reference | in the reference |
| L1 prelude stage 1 | lockdown, helpers, `text`/`console`/`store`…, the `pending` map | the prelude source | in the reference | **the reference** (key: build + prelude) |
| L2 configure | `tools` and `ALL_TOOLS` from the tool list, declared globals, the store snapshot | per run | in the reference | **in the session** |
| L3 session | the script's frames, objects, pending calls | constantly | the snapshot | the snapshot |
| memory limit | – | per run | in the key | **not in the key** (images are byte-identical with a 16 MiB and the default limit) |

Measured (`goldi-images` + `refs.py`, zstd -3, the reference as a raw-content
dictionary as the adapter does). Waiting-session sizes, median over idle points:

| | today (vs exact prelude image) | vs empty-input prelude image | **layered (vs stage 1)** | vs base |
| --- | ---: | ---: | ---: | ---: |
| ask, small inputs | 1.8 KB | 14.3 KB | **3.4 KB** | 39.2 KB |
| ask, realistic inputs | 1.8 KB | 16.5 KB | **5.6 KB** | 41.3 KB |
| agent-loop / sequential, realistic | 6.0 / 6.3 KB | 21 KB | **9.8 / 10.1 KB** | 46 KB |
| tool-heavy / records / big-log, realistic | 48 / 337 / 407 KB | +15 KB | **52 / 341 / 411 KB** | +40 KB |

- The test inputs:
  - "realistic": 18 tools with 200–500-character descriptions (5.4 KB of JSON), 6
    globals and a 1.4 KB store. Their JSON alone is 1.1 KB compressed.
  - "small": snaplab's 10 tools, no globals or store.
- Same output and result in all six cases, today vs layered.
- Start (native release): restoring the exact image takes 0.18–0.20 ms; restoring stage
  1 plus `configure` takes 0.22–0.24 ms.
- Using the *unmodified* prelude with empty inputs as the reference costs +15 KB: the
  input-dependent cells sit in the middle of the prelude's allocations, so everything
  after them moves. The split is what makes it cheap.
- Two traps the prototype hit, both small engine or adapter fixes:
  - **A dead cell inside the reference** moves everything after it: stage 1 must hold
    what `configure` needs (the bridge, the getter), not drop it.
  - **Growable cells:** `configure`'s new object shapes grow the shape tree's transition
    lists, which are replaced and moved (10.8 KB instead of 5.6 KB). The prototype warms
    those shapes in stage 1; `Vm::seal` could put transition lists last, as D2057 does
    for the global object's table.
- **What pi changes:** the factory `(bridge, toolsJson, globalsJson, storeJson) => api`
  becomes `(bridge) => configure(toolsJson, globalsJson, storeJson) => api`.
  `scripts/snapshot-research/portable/factor-prelude.py` does it mechanically on pi's
  text. The one observable difference: the order of the global object's keys
  (`tools`, `ALL_TOOLS` and the declared globals come last).
- **What the adapter changes:** `preludeKey` drops the tools, globals, store and limit.

## 2. Tiers

| | when | content | exact? | survives | fails how |
| --- | --- | --- | --- | --- | --- |
| tier 1 | same djs build and same prelude version | image, zstd against the stage-1 image (D2260 container, reference kind "prelude stage 1") | yes, at any instruction | changed tools, globals, store, limit (with the split) | header check: other build or prelude version |
| tier 2 | different build or prelude version | by-meaning export of the session-owned graph (0141's format, below) | behaviour yes, bytes no; idle points only | djs builds and prelude versions that keep the names | refused at export or import with the missing name (below) |

**Tier 2, as prototyped for pi** (`goldi-tier2`, `goldi-import-all`):
- **Code:** the script's source travels. Functions are found by source span, awaits by
  position, operands by role (0141).
- **Data:** the script's own objects, closures, promises, builtin closures and records.
- **Builtins** are named by path today, and would use registry ids with 0143's step 1.
  A session touches about 30 builtin functions in ~7 holders; at the idle points here,
  the session's cells referenced 5–9 distinct builtin objects.
- **The prelude** is named, never carried:
  - Entry points by global path: the only ones a frame held at any idle point were
    `globalThis/tools` (216 references) and `globalThis/console` (133), the script's
    two parameters. A tool function the script keeps is `globalThis/tools/<target>/read`.
  - Pending calls by id. The exporter reads the `pending` map; the importer gets a
    fresh promise for each id from the new prelude (`adopt(id)`) and puts it where the
    old one was.
  - The prelude's per-run state by name: `nextId`, `finished`, `outputChars`,
    `outputItems`, `storedChars`, `stored`, `writes`. A name the new prelude lacks
    takes its default. Older versions had no output counters; the transfer still worked.
  - The script's result promise is subscribed again through the new prelude's `run`.
    Its old handlers are prelude code and do not travel.
  - The interface the session started with (the global names the prelude defined):
    the importing prelude must define them all, because the rest of the script may call
    any of them.
- **Inputs:** the session's tools, globals and store travel with it (1.1 KB
  compressed). A restore therefore never depends on the host's current tool list; a
  call to a tool the host no longer has is the host's to answer.

**Results** (133 idle points of five cases; agent-loop's 11 are refused, see below):

| export → import | result |
| --- | --- |
| same djs, same prelude (24382bedd) | 133/133 continue exactly |
| each of the 7 older pi preludes (8562bcf66 … 0fb1aee74) → 24382bedd | 133/133 each |
| 24382bedd → 8562bcf66 (downgrade) | 133/133 |
| djs 1b6c2098 (main before 0080) + prelude 8562bcf66 → djs today + 24382bedd | 133/133 |
| djs today + 24382bedd → djs 1b6c2098 + 8562bcf66, and + 24382bedd | 133/133 each |
| prelude without `text` (renamed) | refused at import: "this prelude does not define globalThis/text, which the session's did" |
| a prelude whose tool wrapper awaits (a prelude frame suspended while waiting) | refused at export: "a prelude function is suspended" |
| agent-loop (an async generator and `for await`, never used by codemode, 0130) | refused at export: "an activation of kind 2". Not built, as in 0141 |

**Sizes** (median, zstd -3 of the prototype's JSON; it carries the inputs and the
output so far):

| case | tier 2 | without inputs and output | today's image delta |
| --- | ---: | ---: | ---: |
| ask | 2.3 KB | 1.6 KB | 1.8 KB |
| sequential | 5.3 KB | 4.7 KB | 6.3 KB |
| tool-heavy | 45 KB | 44 KB | 48 KB |
| records | 233 KB | 236 KB | 337 KB |
| big-log | 401 KB | 400 KB | 407 KB |

**Times** (native release, medians):
- Export: 1.2 ms. Part of it is two realm walks for builtin paths, which a registry
  removes.
- Import: 3.0 ms in all. Running the prelude takes 2.56 ms, of which the lockdown is
  ~2.3 ms (without it, a prelude run is 0.74 ms and its image 393 KB instead of 496
  KB). Building the graph takes 0.45 ms.
- With a cached stage-1 image (0.22 ms) or engine-level lockdown, an import is
  ~0.7 ms. Restoring an image takes ~0.2 ms.

**One format for both tiers?** Possible, but not recommended for pi now.
- By meaning is as small as the delta and works at pi's idle points.
- On the same build it is ~3x slower than an image restore, exact only in behaviour,
  and loses the canonical hash.
- Written *alongside* the image, it costs 50–80% of the delta more when compressed with
  the session image as dictionary: ask 1.5 KB, tool-heavy 23 KB.
- The cheapest combination is **export on drain** (0141 §3.8). Images stay the only
  stored form; at a deploy, the old build converts the waiting sessions once. Write
  both forms per snapshot only for sessions expected to outlive a release (waiting on a
  human).

## 3. What pi's prelude must look like

From pi's prelude (pi-private, read-only), `djs heap` on the sessions and the tier-2
graphs:
- **No prelude frame is suspended while waiting.**
  - A tool call is `caller(...)`: `new Promise((resolve, reject) => { pending.set(id,
    { resolve, reject }); bridge(...) })`, synchronous.
  - At every idle point the only suspended activations were the script's (207
    activations, all in `codemode.js`).
  - `djs heap --only session` shows the same for a waiting session:
    `async function at an await, codemode.js:5:13`, reached through
    `handle[7]~pending<entries>[5].resolve<data><reactions>`.
- **What user state references in the prelude:**
  - the script's parameters `tools` and `console`;
  - the promises the callers made, whose resolvers sit in `pending`;
  - `run`'s subscription to the result.

  Everything else (frozen builtins, override accessors, helpers) is the prelude's own
  and is rebuilt by running it.
- **The contract** pi would keep (today it is emulated by binding names in the
  prototype):

  | part | shape | history (8 versions, 2026-09-29 – 10-12) |
  | --- | --- | --- |
  | factory | `(bridge) => configure(tools, globals, store) => api` (after the split) | `(bridge, toolsJson, globalsJson, storeJson)` in all 8 |
  | api | `settle(id, ok, payload, props?)`, `run(fn)`, `stalled()`, plus new `state()`, `restore(state)`, `adopt(id) → promise` | unchanged but for `props`, added in 0fb1aee74 (additive) |
  | globals | `tools`, `ALL_TOOLS`, `console`, `text`, `image`, `exit`, `store`, `load`, declared globals | identical in all 8 |
  | pending record | internal once `adopt` exists | `{ resolve, reject }` in all 8 |
  | rule | prelude code never awaits; tool calls are promises settled by `settle` | true in all 8 |

  `state()` returns the counters and maps as JSON, with a version; `restore` takes them
  back, and unknown or missing fields default. That is about 20 lines in pi.
- **Churn reality check:**

  | | changes | invalidates today's reference | breaks tier 2 |
  | --- | ---: | ---: | ---: |
  | pi prelude, 34 days (`git log` of prelude-source.ts; nothing since 10-12) | 8 commits | 8 | 0 (one additive parameter) |
  | djs, all 142 issues on main | 100 change the build hash | 100 (70%) | not file-detectable: behaviour and schema changes (0141 §3.5–3.6) |
  | djs, last 30 issues | 13 change the build hash | 13 | – |

  The internals change all the time and the interface does not. That is exactly the
  case tier 2 is for.

## 4. The owner's idea: an image with symbolic relocations

The idea: keep the raw image of the session's own cells, but write every reference into
the builtins and the prelude as a stable name, like relocations in a dynamic linker, so
the builtin part can change underneath. Or keep builtins out of the heap entirely.

**What the session's cells actually point at** (`goldi-reloc`, cells above `OLD_END` of
a session started from the sealed prelude image, realistic inputs):

| reference from session cells | ask | sequential | tool-heavy | records (1,500 records) |
| --- | ---: | ---: | ---: | ---: |
| session cells | 78 (4.8 KB) | 301 (19.5 KB) | 419 (151 KB) | 19,976 (1.29 MB) |
| builtin objects (count / distinct) | 12 / 5 | 82 / 7 | 106 / 5 | 4,612 / 9 |
| prelude objects by path | 2 / 2 | 2 / 2 | 3 / 2 | 3 / 2 |
| prelude boxes, templates, shapes, strings (no name today) | 6/4, 2/2, 10/2, 5/1 | similar | similar | 6/4, 2/2, 27/4, 17/2 |
| atom ids (shapes / session bytecode) | 8 / 20 | 11 / 38 | 7 / 57 | 24 / 56 |
| intrinsic ids in session bytecode | 0 | 2 | 2 | 6 |
| changed reference cells (`djs heap --diff`) | 20 (29 KB, 0.5 KB differing) | 20 | 20 | – |

The changed reference cells are:
- the atom table's entries and index;
- the realm (clock, PRNG);
- inline caches filled in prelude templates;
- the handle table;
- a shape transition list.

**What it needs, beyond the names:**
1. **Identical cell layouts** for every kind the session's cells use: objects, shapes,
   property values, strings, arrays, closures, boxes, promises, generators, templates
   and bytecode. Shapes arrived between 1b6c2098 and today (D1831), so these two builds
   could not exchange such an image.
2. **The same bytecode format and opcode meanings.** Session functions travel as
   bytecode, and suspended frames hold pcs into it. Compiler changes are fine (old code
   keeps running); opcode changes are not.
3. **Atom ids relocated**, by string, in shape descriptors, property tables and bytecode
   immediates. Non-well-known atoms get their ids from interning order, which moves
   with every builtins change. This needs per-field knowledge: the layout table
   (D1052) types refs, but atoms hide in `U32Array`s and immediates.
4. **Intrinsic ids relocated** by name, in builtin closures and `call_intrinsic`
   immediates.
5. **Shapes:** the session's shapes hang off reference shapes, so those are named by
   their key path from the root.
6. **The changed reference cells** become diffs by name. The prelude state (`pending`,
   counters, boxes) is the same named state as in tier 2. Prelude templates that
   session cells reference (`run`'s handlers) need tier 2's re-subscription once the
   prelude changes.

**Where it breaks**, measured with `scripts/snapshot-research/churn-tiers.py` on main's
history:

| | per issue (all 142 / last 30) | a release of 10 issues survives | of 30 issues |
| --- | ---: | ---: | ---: |
| today's image | breaks 70% / 43% | 0% | 0% |
| relocations (layout table, well-known atoms, opcodes) | breaks 28% / 10% | 8% | 0% |
| … also counting object/shape code | breaks 36% / 13% | 5% | 0% |
| by meaning | behaviour and schema changes only | survived the 406-commit jump (0141, here) | |

**Verdict:**
- Relocations are a real middle tier: they survive builtin, intrinsic, base and prelude
  changes. They name almost exactly what tier 2 names, and keep restore a copy plus a
  patch.
- But they turn the cell layouts, the opcode table and the atom encoding into a frozen
  ABI. While djs is developed, almost every release breaks that ABI, and a break means
  the session is lost unless tier 2 or replay backs it up.
- With the formats frozen by policy (a stable 1.0), relocations would be the faster
  tier 2. Until then they are an extra format to maintain for little extra survival.
- **Keeping builtins out of the heap** (0143's lazy realm, step 4) makes the builtin
  references trivial: dematerialize builtins on export, rematerialize them by registry
  id on import. Requirements 1–6 stay. 0143 rejects stable ids as the *runtime*
  representation (3–4 months, two object models) but keeps the registry for naming. That
  registry is what both tier 2 and relocations need, so build it either way.

## 5. Recommendation and phases

| phase | what | where | effort | module size | gives |
| --- | --- | --- | --- | --- | --- |
| A, now | drop the memory limit from `preludeKey`; split pi's prelude into stage 1 + `configure`; key the reference by (build, prelude); `Vm::seal` puts shape transition lists last (or a stage-1 warm-up) | js/pi/vm.ts, pi-private (~30 lines), djs-core (small) | days | 0 | changing tools, globals, store or limit no longer invalidates; +1.6–4 KB per snapshot |
| B, with 0143 steps 1–2 | builtin registry (append-only, gate-checked); engine-level lockdown; pi adds `state()`/`restore()`/`adopt()` and keeps "no awaits in the prelude" | djs-core, pi-private | 2–3 weeks (0143's estimate) + ~1 day pi | +2–4 KB | names for tier 2; prelude runs get ~4x cheaper (lockdown 2.3 of 3.0 ms) |
| C, when survival across releases is wanted | tier 2 in djs-core (0141 phase 1, plus the pi naming above); export on drain at deploys; tier 2 written alongside for sessions waiting on a human | djs-core, adapter | 3–4 weeks | est. +8–12 KB gzip (export, import, suspension maps) | sessions survive djs releases and prelude edits that keep the interface |
| not now | relocation images | – | – | – | revisit once layouts, opcodes and atoms are frozen by policy |

Fails cleanly, by tier:
- **Tier 1:** another build or prelude version gives a header refusal, then falls to
  tier 2.
- **Tier 2:** refused, with the name, when:
  - a builtin path or registry id is gone;
  - a prelude global of the session's interface is gone;
  - a function or await is not found (the script source differs);
  - operand roles differ;
  - a record schema is unknown;
  - an async generator or a prelude frame is suspended.
- **After both tiers:** replay (0140) remains the last fallback.

## Reproducing

```sh
S=work/scratch/0142
node --input-type=module -e "import {PRELUDE_SOURCE} from './tests/pi/prelude-source.ts'; \
  import {writeFileSync} from 'node:fs'; writeFileSync('$S/pi-prelude.js', PRELUDE_SOURCE)"
python3 scripts/snapshot-research/portable/factor-prelude.py $S/pi-prelude.js > $S/pi-prelude-factored.js
(cd scripts/snapshot-research/portable && cargo build --offline --release)
P=target/release/portable
$P goldi-images $S/img $S/pi-prelude.js $S/pi-prelude-factored.js scripts/snapshot-research/cases/*.js
python3 scripts/snapshot-research/portable/refs.py $S/img        # section 1's table
$P goldi-reloc $S/img/realistic/ask-today-1.img                  # section 4
python3 scripts/snapshot-research/churn-tiers.py                 # section 4's churn
# pi prelude versions (read-only): git -C ~/Development/pi-private show REV:packages/codemode/src/runtime/prelude-source.ts
$P goldi-tier2 $S/preludes/8562bcf66.js $S/preludes/24382bedd.js scripts/snapshot-research/cases/*.js
# across djs builds: the crate in a git archive of 1b6c2098, built with --features old (0141)
GOLDI_CORPUS_ROOT=$PWD OLD/target/release/portable goldi-tier2 --save $S/x OLD_PRELUDE OLD_PRELUDE CASES...
$P goldi-import-all NEW_PRELUDE $S/x
```
