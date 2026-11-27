# Heap snapshots revisited

Status: research for the owner (issues 0299 and 0300), kept as written. Since then
snapshots are djs's persistence format (0301–0303, D3700) and continuation dumps are gone
(0304, D3730): the dumper, the suspension maps, the dump module and the dump tests this
report measures against no longer exist; its figures for them are what main had then. Sections 1–2 (0299): what psychoscript's snapshots are, and how often djs's
history would have broken a snapshot keyed by bytecode hashes and names instead of the
build. Sections 3–5 (0300): the prototype of such a snapshot for djs, its coverage, sizes
and the portability experiment. The summary is the coordinator's.

The question: continuation dumps (vm/dump*.rs, ~26.5k lines of Rust, ~270 KB gz of the
shipped module) re-enact a waiting run as JavaScript so that later builds can load it.
psychoscript persists runs as ordinary heap snapshots instead: a graph of records, code
never saved but compiled again on load and checked by a bytecode hash. Can a djs heap
snapshot survive upgrades in more situations than "same build", without the dump
machinery, and how small can it be?

## Summary (coordinator)

- **psychoscript is build-bound.** Its snapshot is a logical graph of named-field records,
  code recompiled on load, builtins by name; its compat key is the build (version plus
  opcode, compiler, prelude and codec hashes), so any release refuses older snapshots.
  It is small in code (~1.9k lines TS, 2.6k Rust) and works on our corpus (286/286 idle
  points, 3.6 KB zstd median vs 2.8 KB for plain dumps).
- **The idea worth taking is a finer key.** Key the snapshot on the *named* bytecode of
  the functions that can wait, the field names of the cell kinds it holds, and the paths
  of the builtins it names. Over 196 merges (0102..0298) that key broke at 6 merges (1 since
  0200); the build key breaks at ~75%, images at 70%, relocation images at 28% (0142).
  Half of the bytecode churn there was made *for* continuation dumps.
- **It works in djs, and it is simpler.** The prototype (branch
  `issue/0300-logical-snapshots`, feature `snapshot`): 3.1k lines and +60 KB gz, against
  the dumper's ~24.5k lines and 270 KB gz. Every harvest idle point (1,164) and every dump
  program (197) round-trips and continues exactly, chains included, also cases dumps
  refuse (private fields). Export 50 µs vs 492 µs for a dump (3.5-50x with large state: a
  1e6-number array 3 ms vs 153 ms); import 189 µs, most of it recompiling. Size without
  sources 2.7 KB zstd median (dump 3.4 KB); a delta against the previous snapshot of the
  run ~260 B. It survives renumbered intrinsics, atoms and opcodes, a moved base, changed
  cell layouts and changed code of every function that does not wait, and refuses with a
  reason when a waiting function's bytecode, a builtin path or a field name changes.
- **What dumps still do that snapshots cannot:** survive a lowering change in a waiting
  function. That needs a fallback. Note for pi: its durable code mode (pi-mono-djs branch
  `djs-durable`) has since dropped the journal and replay, so "fall back to replay" would
  have to come back, or the fallback is keeping the previous engine for a deploy, or a
  refusal surfaces as an interrupted call.
- **Recommendation:** move persistence to logical snapshots and retire continuation dumps,
  after three contracts that make the key real (section 5.1): a tested slot table for
  object classes, named records for builtin closure kinds, and a gate pin of the corpus's
  waiting-function bytecode. Within one build snapshots are already strictly better than
  dumps (faster, smaller, more coverage); across builds they need those contracts and a
  fallback for the rare refusal. Owner decisions: adopt or not, the fallback, and when the
  format window opens.

<!-- Sections 1-2: issue 0299. -->

## 1. psychoscript's snapshots

psychoscript (vendor/psychoscript, at e79ccd5) is a sandboxed JavaScript interpreter with
two implementations: the TypeScript reference (`src/`), whose VM is made of ordinary JS
objects (an `Obj` class with a `props` Map, `Arr`, `Fn`, `Frame`, `PromiseObj`, ...), and a
Rust port (`rust/crates/psychoscript`), whose heap is a `Vec` of Rust enums addressed by
index. Both suspend a run, write it as JSON and resume it in another process.

### 1.1 What a snapshot holds

TypeScript (`src/snapshot.ts`, `src/snapshot/codec.ts`, `src/snapshot/snapshot.ts`):

| part | content |
| --- | --- |
| header | `build` (see 1.3), `hosts` (a hash of the host globals' names, namespaces and hooks), `runId`, `sources` (each program source's id, full text, text hash and bytecode hash), limits, budget used, the host's metadata |
| roots | the `Vm`, the interpreter, the global scope, then every builtin the run changed |
| records | one per reachable object: `{t: class, f: [fields in spec order], p: proto, x: extensible, o: [key, value, attrs]...}` for objects; `Map`, `Set`, `Array`, plain `Object`, `Error`, `Date`, `URL`, typed bytes as base64 |
| values inside records | JSON primitives or tags: `["u"]` undefined, `["n","NaN"]`, `["b",hex]` BigInt, `["s",name]` symbol, `["i",name]` an unchanged builtin, `["a",source,index]` an AST node, `["x",data]` something the VM names (code), `["r",id]` another record |

- **Generic codec, per-class specs.** Each VM class has a `ClassSpec`: its saved fields in
  order, fields it skips (caches, configuration) and a `reset` that rebuilds derived state
  (an `OrderedTable`'s index, a RegExp's compiled program). An own field the spec does not
  list throws "Codec gap: Class.field is not saved", so new VM state cannot be dropped
  silently. 48 specs in all: 37 in codec.ts (objects, promises, generators, collections,
  host calls, the interpreter) and 11 VM classes in snapshot.ts (`Vm`, `VmScheduler`,
  `HostCalls`, `Frame`, `Fn`, ...).
- **Code is never saved.** A function's code is `["x", ["code", source, index]]`. Restore
  re-parses and recompiles every saved source and refuses one whose bytecode hash differs,
  "so a pc always means the same instruction". The prelude's and the VM's internal helpers'
  code are named by the build id, which hashes their bytecode.
- **Frames** are records: function, code, pc, sp, the registers up to sp (locals and the
  live operand stack), `this`, args, mode, the promise or generator they settle. Restore
  checks each frame against its code before anything runs: pc within the code, sp within
  locals..locals+maxStack, no holes on the operand stack, finally tags in range, a known
  mode that fits the code.
- **Builtins by name.** Every object made while the intrinsics are created gets a static
  id (its creation index, deterministic). The first runtime of a process names them once
  by a breadth-first walk from the globals (`Array.prototype.map`,
  `Map.prototype.size[[Get]]`, `%Generator%` for unreachable ones). An object carries a
  dirty bit: every write to its properties, prototype, extensibility or elements marks
  it. Unchanged builtins are saved as `["i", name]`; changed ones are saved whole, as roots.
  Host globals get `@name` paths per runtime.
- **Host calls in flight** by kind: an idempotent async call is saved as queued and runs
  again on resume with the same `call.key`; a durable call (`Host.durableFn`) saves the
  JSON its `start` returned and waits on it again; any other call in flight makes
  `suspend()` reject with a `SuspendError` naming it, and the run goes on.
- **Functions inside host results** are saved through the owning host function's
  `snapshot: {encode, decode}` hooks, or become `Lost` placeholders (reported in `lost`;
  a strict save throws).

The Rust port (`snapshot.rs`, 2,584 lines) has the same contract with a positional
representation, which is the closer model for djs's cell heap:
- **Builtins by position.** A fresh run makes the same objects, strings, cells and host
  functions in the same order before the program starts (the base: five counts). The
  exporter builds a fresh baseline VM, encodes each base object of both and saves those
  that differ as `[id, object]`; everything after the base is saved whole, by index.
- **Natives by index** in a table of every native the VM can make, built once per process.
- **Strings and atoms** after the base's are saved as a table; values refer to heap ids.
- A hash of the state's JSON in the header refuses damaged or edited snapshots.

### 1.2 The compat key: the build, then the bytecode

The bytecode hash is not the compat key; the build is. Restore checks the build id first:
- TypeScript: `buildId()` is the package version plus a hash of the snapshot format, the
  opcode table, the frame modes, `COMPILER_VERSION`, the prelude's and the internal
  helpers' bytecode, the static intrinsic names in walk order and the codec shape (every
  spec's name and field list).
- Rust: the package version plus a hash of the format and the counts of what a fresh VM
  makes (natives, base heap size, atoms).

Any release therefore refuses older snapshots, as the README says: "resuming it in
another build ... rejects with a SnapshotError". The per-source bytecode hash is a second
check within a build. It hashes every function of the source together: TS `hashOf` over
the raw opcode words, captures, handlers, finally tables, sites and constants; Rust over
the `Debug` text of each proto's code and positions. One changed function anywhere in the
program refuses it. psychoscript never tried to survive upgrades. The idea 0300 tests,
dropping the build from the key and keeping per-function bytecode hashes plus names, goes
further than psychoscript does.

### 1.3 What it refuses

At restore:
- another format or build;
- other host globals (`hosts` hash);
- a source whose text does not match its hash, or that compiles to other bytecode;
- a frame that does not fit its code (1.1), a job stack out of order, a closure whose
  cells do not match its captures;
- malformed records (every field is type-checked as it is decoded).

At suspend:
- a blocking or non-idempotent host call in flight;
- a call holding host functions;
- a state over `maxBytes`;
- a codec gap: an unknown class or an unlisted own field.

Values with no data form are reported in `lost`, or thrown with `strict`.

### 1.4 Sizes, measured

Commands are in "Reproducing" at the end. TypeScript reference under bun:
- **README's suspend example** (a loop awaiting `fetchNumber`): 5,652 bytes of JSON,
  1,571 bytes zstd -3. The global scope alone is 57 binding records (one
  `{value: ["i","Object"]}` per global), a fixed ~2.5 KB raw per run.
- **The 300 harvested codemode cases** (tests/corpus/codemode-harvest), run like djs's
  corpus runner: `later` is an idempotent async host call, one answered per idle point,
  oldest first. Suspended at the first and at the last idle point, resumed in a fresh
  runtime, compared with the uninterrupted run:
  - 286 cases have an idle point (14 make no call);
  - all 286 × 2 snapshots restore and continue with the same logs and result, apart from
    wall-clock values the scripts print (`elapsedMs`);
  - none is refused.

Sizes, median (p90) over those idle points, against djs's plain dumps of the same cases
at the same idle points (`djs-difftest --dump-out`, today's main):

| idle point | case source | psychoscript JSON | zstd -3 | djs plain dump | zstd -3 |
| --- | ---: | ---: | ---: | ---: | ---: |
| first | 2.6 KB (4.8) | 13.3 KB (20.8) | 3.6 KB (4.8) | 9.6 KB (13.1) | 2.8 KB (3.7) |
| last | 2.6 KB (4.8) | 13.1 KB (21.5) | 3.6 KB (4.9) | 8.4 KB (11.7) | 2.6 KB (3.4) |

- psychoscript is 1.3–1.4× the plain dump after zstd (median ratio per idle point).
- Where its bytes go (mean per snapshot at the first idle point):
  - the sources in the header: 4.3 KB, the case text JSON-escaped;
  - the global scope: plain `Object` binding records 3.6 KB and the globals map 1.2 KB;
  - `Obj` records 1.8 KB, arrays 1.8 KB, `Fn` records 1.4 KB;
  - promises, host calls, frames and resolving records: ~0.3–0.5 KB each.
- The run's own state is a few KB. The rest is JSON verbosity, the source text and the
  per-run global table. A binary djs format that refers to the source by hash, and to
  untouched globals not at all, starts well below both columns (section 3).
- The djs dumps carry the corpus prelude (8.3 KB of source) re-enacted, and psychoscript
  carries its little prelude in the case text: neither column is "the run alone".
- Suspend takes a median 0.13 ms (first idle point) and 0.21 ms (last) under bun.

### 1.5 How much code

| | lines |
| --- | ---: |
| TypeScript: snapshot.ts 385, codec.ts 1,131, snapshot/snapshot.ts 143, runtime/intrinsic-names.ts 200 | 1,859 |
| Rust: snapshot.rs (plus a few hooks in vm.rs, run.rs, collector.rs, host.rs) | 2,584 |
| djs: vm/dump*.rs today | 26,528 |

Most of the TS codec is the generic part (encoder, decoder, validation). The per-class
code is the 48 specs, mostly a few lines each, plus a few `reset`s.

### 1.6 What djs can take, and what does not transfer

Takes over directly:
- **The overall contract.** Snapshots only at points where no native frame is live; code
  by (source, function index) and recompiled; builtins by name; host calls by id with the
  host's own state; refuse with a reason rather than guess.
- **"An unknown field is a codec gap."** psychoscript gets it from JS reflection
  (`Object.keys` of each instance against the spec). djs's equivalent is the `cells!` layout
  table: every cell kind's fields are named and typed, so a table-driven codec can refuse
  a cell kind or field it has no rule for.
- **Builtins by path with dirty bits.** djs has the registry
  (tests/builtins/registry.tsv: every builtin by path and every intrinsic by name), which
  is psychoscript's walk, committed. Frozen builtins and the base prefix's write latch play
  the role of the dirty bits: an unchanged base object is a path, a written one is saved.
- **The Rust port's base diff.** "Save what differs from a freshly made base" is cheap for
  djs: every VM starts as a copy of the base image, so the base cells a run wrote can be
  found by comparing with the image.
- **Frame checks before running.** Verify pc on an instruction boundary, sp within the
  frame, handler depth, against the recompiled code, before anything runs.
- **A round-trip test mode.** psychoscript's tests can save and restore at every step
  boundary (`tripEvery`). That is djs's `--suspend-every` stress mode, pointed at
  snapshots.

Does not transfer, or needs work djs does not have yet:
- **Named slots.** psychoscript's records name every field because its VM objects are JS
  class instances. djs's object classes keep their state in positional internal slots,
  whose meaning is written only in prose (heap/object.rs: "slot 0 the state, slot 1 the
  result..."). Builtin records are positional too (`NewRecord(n)`,
  `RecordGet(r, i)` in builtins/*.ts). A by-name djs snapshot needs a machine-readable
  slot table per object class and per builtin closure kind's record. That table does not
  exist yet; it is the main new contract.
- **Positions as names.** The Rust port's "builtins by position" works because the base
  is identical within a build. Across djs builds, positions (registry ids, cell offsets,
  atom ids, intrinsic ids) move all the time (section 2.4), so they must become paths,
  names and strings at the boundary.
- **A whole-source bytecode hash** is too coarse for djs's goal. Section 2 shows that a
  per-function hash survives far more often, and only functions with a frame need it.
- **JSON** is 3–4× the size of the content. A binary record format with a string table
  is the obvious change.
- **Strings, shapes and property tables** are trivial in psychoscript (JS strings, Maps).
  In djs they are ropes, slices, shape trees and dictionary tables, and the snapshot
  flattens them to values and rebuilds them. That is real codec work, but internal: it
  never reaches the format.

## 2. Stability evidence: how often a bytecode-keyed snapshot would have broken

A snapshot keyed by bytecode and names survives a djs change that keeps:
- (a) the bytecode the compiler emits for the run's functions;
- (b) the meaning of the named fields of the cells and slots it holds;
- (c) the builtin, intrinsic and closure-kind names it refers to.

This section measures how often each of those changed, merge by merge, over djs's
history.

### 2.1 Method

- **Merge points.** main is linear (every issue lands by fast-forward), so each issue's
  last commit was main's tip once. The points are those last commits for issues
  0102..0298: 196 points, 195 consecutive pairs. Older issues are left out.
  - The recent window, 0200..0298 (96 points), is the development pace a stored snapshot
    faces now; it is reported separately.
  - Points are in merge order, which is not issue order (0151 merged after 0162).
- **Builds.** `djs-cli` at every point: dev profile, one detached worktree, one shared
  target dir, niced, `-j 6`. All 196 built, each in 4–18 s incremental.
- **Corpus.** Today's tests/corpus/codemode/*.js (31) and codemode-harvest/*.js (300):
  331 scripts, 2,129 functions (median 5 per script), 696 of them containing an
  await or yield.
  - Every build compiles the same text, wrapped as difftest wraps it: a Script holding
    `(async (tools, console) => {SOURCE\n})`.
  - Every build compiled all 331.
- **Hash levels.** Each function's `djs disasm` output is hashed at three levels. The
  disassembler prints atoms, intrinsics and constants by name or value, so atom and
  intrinsic ids never reach the hash.
  - **exact:** everything but source positions (`; line:col`) and regexp literal cells'
    heap offsets (`internal@24336`): pcs, opcodes by name, operands, IC slot numbers,
    the header (params, locals, stack, captures) and the handler table.
  - **pc:** what a saved frame relies on: exact without IC slot numbers, constant pool
    indexes, max stack size and locals' debug names. pcs, opcodes, operands, local slots
    and handlers stay.
  - **frames:** pc equality for the functions that contain an await or yield (the ones
    that can hold a frame at an idle point). For the others, only their interface: params,
    capture count and the captured names they read. A closure of such a function holds no
    pc: it refers to (script, function) and its captured boxes.
- **Survival.** A script survives a span of N issues when every function matches at both
  ends. Two numbers per span:
  - "scripts": the share of scripts that survive, averaged over all start points;
  - "all": the share of start points where every script survives.

### 2.2 Bytecode changes, pair by pair

Of 195 pairs, 10 changed the bytecode of any corpus script, and 6 changed a function that
can hold a frame. Since 0200: 4 of 95 changed any bytecode, and 1 a frame-holding function.

| merge | scripts changed: exact | pc | frames | functions changed | what |
| --- | ---: | ---: | ---: | ---: | --- |
| 0160 | 331 | 331 | 325 | 2,120 | 0151's short opcode forms and shortened jumps (D2443, landed inside this span): every pc moves |
| 0151 | 312 | 4 | 2 | 928 | its base-table work: constant pool order (pure numbering, normalized at pc level), plus 4 real changes |
| 0170 | 4 | 4 | 0 | 7 | for dumps: `jump_if_loading` around class static initializers |
| 0192 | 312 | 312 | 310 | 446 | for dumps: statement state (for-of records, switch, destructuring) in named locals instead of the operand stack |
| 0193 | 313 | 313 | 311 | 410 | for dumps: finally completions in temporaries instead of `gosub`/`ret` |
| 0194 | 3 | 3 | 3 | 3 | for dumps: for-await's exception in a temporary |
| 0205 | 4 | 4 | 0 | 7 | for dumps: `jump_if_loading` gone with format 2 |
| 0236 | 1 | 1 | 0 | 1 | the construct path: a derived constructor's return |
| 0259 | 51 | 51 | 17 | 88 | known builtins: `get_builtin` guards global builtin reads |
| 0280 | 10 | 10 | 0 | 20 | the construct path landed: `check_ctor` gone from constructors |

(A merge's span holds every commit since the previous point; 0151's opcode change landed
before 0160's tip and 0151's own tip later.)

The changes come in three kinds:
- **Changes made for continuation dumps** (0170, 0192, 0193, 0194, 0205): half of the
  ten. The dumper describes a suspended frame by construct, so the compiler moved
  statement state off the operand stack into named locals and replaced `gosub`/`ret`.
  Two of them moved every pc of every script. A heap snapshot that stores registers as
  they are would not have needed them.
- **An encoding rewrite** (0151's short opcode forms, in the 0160 span) that moves every
  pc.
- **Targeted lowering changes:** class bodies and constructors (0236, 0280), which never
  hold a frame at an idle point, and 0259's known builtins. 0259 is the only one that
  reached awaiting functions: it changes how every read of a global builtin (`JSON`,
  `Array`, `Promise`) compiles.

The 0200..0298 window had no rewrite.

### 2.3 Survival over spans

0102..0298 (195 pairs):

| span (issues) | exact: scripts | all | pc: scripts | all | frames: scripts | all |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 97.9% | 95% | 98.4% | 95% | 98.5% | 97% |
| 5 | 91.4% | 78% | 93.8% | 78% | 94.3% | 88% |
| 10 | 83.3% | 59% | 87.8% | 59% | 88.9% | 78% |
| 20 | 70.3% | 30% | 75.0% | 30% | 77.0% | 60% |
| 30 | 56.9% | 16% | 60.8% | 16% | 63.8% | 40% |
| 60 | 26.8% | 0% | 26.9% | 0% | 31.2% | 3% |

0200..0298 (95 pairs):

| span (issues) | exact: scripts | all | pc: scripts | all | frames: scripts | all |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 99.8% | 96% | 99.8% | 96% | 99.9% | 99% |
| 5 | 98.9% | 78% | 98.9% | 78% | 99.7% | 95% |
| 10 | 97.7% | 57% | 97.7% | 57% | 99.4% | 88% |
| 20 | 95.1% | 17% | 95.1% | 17% | 98.6% | 74% |
| 30 | 92.1% | 0% | 92.1% | 0% | 97.7% | 55% |
| 60 | 83.1% | 0% | 83.1% | 0% | 94.9% | 0% |

- A **whole-script hash** (psychoscript's choice; the exact and pc columns): across 30
  recent issues a given waiting script survives 92% of the time, but some script of the
  corpus always breaks.
- **Per-function hashes, required only where a frame is** (frames columns): a waiting
  script survives 30 recent issues 97.7% of the time, and in 55% of the 30-issue windows
  every one of the 331 scripts survives.
- The difference between the two comes almost entirely from constructor and class-body
  changes in functions that never wait.

### 2.4 Names, not bytes

The disassembler's names hide what the encoded bytes contain: opcode bytes, intrinsic
ids and atom ids. Over the same 195 pairs (schema.py, from the sources at each point):
- the opcode table renumbered existing ops in 5 pairs (0152, 0170, 0192, 0193, 0205), and
  removed ops in 3 (`gosub`, `ret`, `jump_if_loading`, `check_ctor`);
- intrinsic ids moved in 15 pairs, 9 of them in 0200..0298. The intrinsics table is in
  declaration order of the builtin sources, so a new builtin early in a file moves every
  later id.
- Non-well-known atom ids come from interning order, which moves with every change to
  the builtins; they were not measured, as they move far more often still.

Counting only opcode and intrinsic renumbering, a hash of the raw bytes would survive
one issue 89% of the time, 10 issues 36%, 20 issues 9% and 30 issues 0%. That is an upper
bound: any moved intrinsic is counted, used by the corpus or not. It is the order of
goldilocks' relocation images, which depended on the opcode table too; the
name-normalized hash above survives 10 issues for every script in 78–88% of the windows. The bytecode hash must therefore be computed over a normalized form: opcodes by
name, intrinsics by name, atoms by string, constants by value, pcs kept. It is cheap:
the disassembler already decodes every operand.

### 2.5 Named schemas

Read with `git show` at each point (schema.py):
- the layout table (js/src/layout.generated.ts): cell kinds, field names and types,
  value tags, flags, object class names;
- the object classes' slot counts and slot documentation (heap/object.rs);
- the registry's live paths and intrinsic names;
- the builtin closure kinds (`BUILTIN_CLOSURES`);
- the `NewRecord` sizes and highest `RecordGet`/`RecordSet` index per builtins file;
- the named constants that spell out stored state: the frame header and flags,
  generator/async states and slots, promise states, flags and slots, Map/Set entry
  layout, property store flags.

A change counts when a name a snapshot would use is gone or renamed, or a field's type or
a constant's value changed. Additions, offsets, ids and order do not count; additions
are listed apart.

| category (pairs with a change, of 195) | 0102..0298 | of them since 0200 |
| --- | ---: | ---: |
| engine sources changed (a build-keyed snapshot breaks) | 146 (75%) | 77 of 95 (81%) |
| layout hash changed (what an image depends on) | 14 | 4 |
| stored cells: a field gone or retyped | 0 | 0 |
| VM-only cells (Roots) and code cells (FuncTemplate: recompiled, never stored) | 2 + 3 | 1 + 0 |
| object class slots: fewer | 1 (0151: a closure's template left slot 0) | 0 |
| object class slots: more | 1 (0236: BuiltinClosure gained `length`) | 1 |
| user-reachable registry paths gone | 0 | 0 |
| internal paths gone (`%prelude:`, `%intrinsic:`) | 8 | 6 |
| intrinsic names gone | 3 | 2 |
| builtin closure kinds gone or reordered | 0 | 0 |
| record sizes or indexes changed (builtins/*.ts) | 2 (0232, 0248) | 2 |
| state constants gone or changed | 2 (0232, 0280) | 2 |

The four codemode-visible hits were read by hand:
- **0151:** a closure's template moved from slot 0 to the object's `ELEMENTS` field. A
  closure record that names its template by (script, function) does not change.
- **0232:** the new 7-slot record lives only while `Promise.all`/`any`/`allSettled` set
  up, never at an idle point. `AsyncFunctionThrow` was called by the engine itself; no
  waiting heap holds it.
- **0248:** module evaluation records; a codemode run holds no module state.
- **0280:** `FLAG_CONSTRUCT` went. A constructor's frame never waits at an idle point,
  and the other frame flags kept their values.

None of them would have broken a logical snapshot of a codemode run. What the tables do
show is the shape of future breaks:
- slots moved (0151) or added (0236) in an object class, which only a named slot table
  turns into something a codec can check;
- internal builtins renamed in bulk (0152 renamed 223 prelude-function paths, 0230
  removed 73). A snapshot must therefore never name internal prelude functions, only
  user-reachable builtins and closure kinds.

The proxies see names and declared constants only. A change of meaning that keeps every
name, such as a slot's encoding (flag bits written as literals) or a state machine
reinterpreted, is invisible to them. 0300's portability experiment and its golden
snapshots are the check for that.

### 2.6 Together, against goldilocks

A script's snapshot survives a span when its functions survive at the frames level and
no codemode-visible schema change lies in the span. The raw columns count every
proxy hit; the triaged ones drop the four hits of 2.5 read as not visible.

| span | 0102..0298: scripts | all | triaged: scripts | all | 0200..0298 triaged: scripts | all |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 96.5% | 95% | 98.5% | 97% | 99.9% | 99% |
| 5 | 84.4% | 81% | 94.3% | 88% | 99.7% | 95% |
| 10 | 71.1% | 65% | 88.9% | 78% | 99.4% | 88% |
| 20 | 48.7% | 39% | 77.0% | 60% | 98.6% | 74% |
| 30 | 29.1% | 20% | 63.8% | 40% | 97.7% | 55% |

Goldilocks (0142) measured, per issue over the first 142 issues and over its last 30:

| | breaks per issue | a 10-issue release survives | a 30-issue release survives |
| --- | ---: | ---: | ---: |
| an image (build hash) | 70% / 43% | 0% | 0% |
| relocation image (layout table, well-known atoms, opcodes) | 28% / 10% | 8% | 0% |
| **bytecode + names, frames level (this section), 0102..0298 / 0200..0298** | **3% / 1%** | **78% / 88% (every script)** | **40% / 55% (every script)** |
| engine sources changed, the build hash's proxy (this section) | 75% / 81% | – | – |

The two studies used different corpora and windows, so compare orders of magnitude, not
digits. Keyed by normalized bytecode and names, a snapshot breaks an order of magnitude
less often than a relocation image and two orders less than a build-keyed one. When it
breaks, it is for one reason a gate can see: the compiler emitted other bytecode for a
function that waits.

### 2.7 What it means for the design

- **Hash per function, by name, required only for functions with a frame.** A
  whole-script hash, as psychoscript uses, loses most of the gain to constructor and
  class-body changes. A raw-byte hash loses it to opcode and intrinsic renumbering.
- **Most of the history's breaks were deliberate compiler rewrites**, and half of all
  bytecode changes were made for the dumps (2.2). With heap snapshots there is no such
  pressure on the compiler. A test that pins the corpus's normalized bytecode (the hash
  of scripts/research/0299, as a gate leg) would make each remaining change a visible
  decision. It is the "pin the operand roles" test portable-snapshots (0141) proposed, at
  the bytecode level.
- **Named schemas were stable, and their weak spot is positional slots.** The format
  needs:
  - a slot table per object class and per builtin closure record;
  - defaults for added slots (0236's `length` is derivable from the closure kind).
- **Name only user-reachable builtins and closure kinds.** Internal paths churn in bulk
  (0152, 0230), user paths never changed.
- **The bad case is a pc that moves under a waiting frame.** A snapshot that refuses on a
  hash mismatch needs a fallback, as dumps or replay provide today. Over the last 95
  issues that fallback was needed at one merge (0259), for 17 of 331 scripts.

### 2.8 Left out

- Issues before 0102. The 0102..0199 window has the big rewrites already; earlier ones
  predate the harvested corpus.
- The prelude and builtin functions' bytecode: a snapshot at an idle point holds no
  builtin frame, so it does not depend on it.
- Capture descriptors, function flags and line tables, which `disasm` does not print. A
  real hash should include the descriptors and flags; they change with the code they
  describe.
- Non-well-known atom ids (2.4).
- The Rust port's sizes: its build pulls in ICU4X and was not built. The TS reference
  was measured.
- Semantic changes that keep every name (2.5).

### Reproducing

The scripts lived in `scripts/research/0299/` and were removed before the merge; they are
in main's history (`git log --all -- scripts/research/0299` finds the last version, also
kept on branch `issue/0299-snapshot-stability`). Outputs go to work/scratch/0299.

```sh
R=scripts/research/0299; S=work/scratch/0299
$R/points.sh 0100 > $S/points.txt                 # merge points: "pos sha issue"
sort -rn $S/points.txt | $R/build.sh              # djs-cli at each point -> $S/bins/<sha>/djs
python3 $R/disasm_all.py                          # $S/disasm/<sha>/<case>.txt
python3 $R/analyze.py [0200] > $S/bytecode.txt    # 2.2, 2.3, 2.6
python3 $R/schema.py 0100 > $S/schema.txt         # 2.4, 2.5 (git show only, no builds)
# psychoscript (1.4): a copy with bun install, then
cd $S/psy && bun exp/readme.ts && bun exp/harvest.ts 1 ../../../tests/corpus/codemode-harvest/*.js
cargo run -p djs-difftest -- --engine djs --dump-out $S/dumps codemode-harvest
python3 $S/sizes.py
```

<!-- Sections 3-5: issue 0300. -->

## 3. A logical snapshot for djs, as built (0300)

A prototype, on branch `issue/0300-logical-snapshots` (not on main): the module
`crates/djs-core/src/vm/snapshot/` behind a non-default cargo feature `snapshot`, its tests,
and two exports in djs-wasm. Everything in sections 3 and 4 was measured with it.

### 3.1 The interface

- `BaseInfo::new(base image, options)`: made once per base (per build). It holds the
  builtins' registry paths (offset to path and back, from `registry::reachable_names`), and
  the own properties of a fresh VM's per-VM objects, which the patches below are relative
  to.
- `vm.snapshot(&base) -> Result<Vec<u8>, Refusal>`, `vm.snapshot_with(&base, &opts)`:
  `SnapshotOptions { sources: false }` writes each script's hash instead of its text.
- `Vm::from_snapshot(&base, bytes)`, `Vm::from_snapshot_with(&base, bytes, sources)`: a
  fresh VM from the importer's own base, with the run in it. The host gets its handles
  (by index), its open calls (by id) and its host functions (by id) back as they were.
- `describe(bytes)`: sections, record types, builtins and functions, for people.
- djs-wasm: `vm_snapshot(vm)` and `vm_from_snapshot(len, limit)`, for size measurements;
  no Sandbox method.

A snapshot is taken only at an idle point, the moments a dump is taken: the run is
`Idle`, `Done` or `Threw`, no frame is on the stack, no job is queued, nothing waits in the
host inbox, and the builtins were never written (the frozen-builtins latch). Open async
host calls are fine.

### 3.2 The format

Binary, self-describing, written in one pass (export) and read in one pass (import):

| section | content |
| --- | --- |
| header | `DJSNAP`, format version 1 |
| flag sets | header-flag names per kind (`Object`: extensible, lengthReadonly, builtin, builtinCtor, hostAsync, lazyLength, lazyName; `BigInt`: negative; `Symbol`: private...) |
| types | every record type used: its name, its fields as (name, type), a 64-bit digest of both |
| strings | every string the records use: content, Latin-1 or UTF-16 |
| builtins | the base objects referred to, by registry path |
| intrinsics | the builtin closure kinds referred to, by intrinsic name |
| scripts | name, parameters, compile options, and the source (or its hash) |
| functions | (script, source span), a flag and a hash (3.5), the template objects made at its tagged templates |
| records | one per reachable cell above the base, each a type id and its fields in the type's order |
| `Vm` | the interpreter's per-run state, the host call table, the handle table, the realm's per-VM values |

Values are a tag byte and a payload: `undefined`, `null`, booleans, an int32 (zigzag
varint), an f64 (8 bytes), hole, uninitialized, a string, a record, a builtin, a function,
a script. A value keeps its form: an int32 stays an int32 and an internal reference stays
internal, so the imported heap holds the same bits the exporter's did, up to offsets.
References to strings and records are written as 0 for "the next one not referred to yet"
and as id + 1 otherwise. A breadth-first graph mostly refers to new records, and keys and
prototypes repeat, so both forms compress: a Map of 10,000 entries went from 88 KB to
26 KB zstd with it, compared with plain ids.

### 3.3 Records: kind names and named fields

- **Generic kinds**, straight from the layout table (`heap/cells.rs`): `ValueArray`,
  `EphemeronArray`, `U32Array`, `Bytes`, `BoxCell`, `HostCall`, `HandleTable`, `BigInt`,
  `Symbol`. Every field by its name and type (value, ref, uint, int, f64), the tail as its
  elements (a record tail field by field: `SLOTS[].VALUE`, `SLOTS[].NEXT`), and per kind a
  little spec (`schema.rs`): which field counts the tail in use, which fields are not state
  (a symbol's atom id), the header flags by name.
- **Objects**, by class: `Object.Promise`, `Object.Generator`, ... (33 classes). Each has
  its prototype, its header flags by name, its own properties, its elements, and its
  internal slots *by name* from a slot table in `schema.rs`
  (`Promise [state, result, reactions, flags]`,
  `Generator [state, frame, pc, hp, promise, kind, queue]`). Closures, records and
  iterator helpers have a variable tail (`captures`, `fields`). The table did not exist:
  heap/object.rs describes the slots in prose only. It is the main new contract, as 2.7
  predicted.
- **Properties** in storage order as (key, attributes, value or getter and setter). Keys
  are strings or symbol records. Shapes, property tables, accessor pairs and the atom
  table are not stored: the importer adds the properties in order and its heap builds its
  own shapes or dictionaries.
- **Per-kind code** only where a field is not self-describing:
  - strings (flat, ropes, slices, append buffers) are their content, in the string table;
  - an atom is its string; a symbol gets a new atom on import;
  - a Map or Set's hash index is its bucket count, rebuilt (`collection_rehash`); the
    entries, tombstones included, are a generic `ValueArray`, so live iterators keep
    their positions;
  - a RegExp's compiled program is compiled again from its source and flags;
  - a builtin closure's kind (slot 0, an intrinsic id) is the intrinsic's name;
  - the template objects of tagged templates live in the script's constant pool: they
    travel by (function, constant index), and the importer checks that its code has a
    `get_template_object` at that index.
- **The zero state.** A field the snapshot has and the importer's layout lacks refuses
  the snapshot, unless the field holds the zero state a new cell starts with. A field
  the importer has and the snapshot lacks starts at zero. That is cells.rs's own rule
  ("zero bytes must be a valid initial state").
- **A kind or class without a spec refuses** the snapshot with the record it was found
  in ("an object of class WeakRef"), as the dumper's refusals do. Refused as built:
  `WeakRef`, `FinalizationRegistry`, `TextCodec`, host refs, a script without its source,
  a lazy template, an unnamed builtin cell, a realm intrinsic above the base that the
  `Vm` record does not name.

### 3.4 Names

- **Builtins by registry path**: a value that refers to a cell below `BASE_END` is written
  as its path (`Promise.prototype.then`, `%ArrayIteratorPrototype%.next`). Median 14 paths
  per snapshot. A cell of the base without a path refuses.
- **Builtin closure kinds by intrinsic name** (`PromiseResolveFunction`).
- **Host identities as they are**: host function ids, call ids, handle indexes, the next
  call id. These are the host's contract already.
- **The per-VM objects of a fresh VM** (the global object, the global lexical record, the
  `Symbol.for` registry) are patches over the importer's own: the keys deleted, the
  properties that differ, and every property after the first one out of the fresh
  order. A record that refers to one of them refers to the importer's object.
- **The realm's per-VM values** are named fields of the `Vm` record: the random state,
  the clock, the time zone offset, the global lexical epoch, the kept objects, the
  registries, the modules. Caches (the RegExp cache, the unit strings) and the prelude
  function table, which is per VM but holds the base's functions, are not stored.

### 3.5 Code

Templates are never stored. A closure's template, a frame's callee, any internal reference
to a template is a function: (script, source span). The importer compiles every script
again and finds the function by its span; two templates with one span (class field
initializers) are told apart by the hash. A span rather than an index in compile order:
it is what `Function.prototype.toString` returns, and it does not move when the compiler
adds or drops a synthetic function elsewhere in the script.

Each function carries one of two hashes, as 2.7 recommends:
- **With a stored frame** (the callee of a suspended Generator, async function or async
  generator): the **bytecode hash**, so that the frame's pc, its registers and its header
  mean the same. It covers every instruction by opcode name, its byte offset and its
  immediates, with atoms by string, intrinsics by name and constants by value (the
  constant pool is the script's, so its numbering moves with other functions' constants);
  the frame shape (parameters, `length`, locals, stack depth, captures, flags); the
  capture descriptors; the handler table; the name; the children's spans in order. Left
  out: the line table and local names (stack-trace text, not the meaning of a pc), the
  inline caches, the template object a site made (state, carried separately).
- **Without one** (a closure only): its **interface hash**: parameters, `length`, flags,
  name, and the captured bindings by name in order. Its code may change: the closure
  runs the importer's code with the same Boxes.

A suspended frame is stored as it is: the Generator record's `frame` slot is a
`ValueArray` of callee, `this`, arguments, the 6-slot header, locals and the operand stack,
and its `pc` and `hp` slots. Nothing is described by role or rebuilt: the hash makes
that unnecessary. Median per snapshot: 22 functions, 2 of them with a frame (at most 18).

### 3.6 Import

1. Parse the whole snapshot; refuse a type field with an unknown type.
2. A fresh VM from the importer's base (3 µs).
3. Compile the scripts (with the given sources when the snapshot has hashes), index the
   templates by span, check every function's hash.
4. Resolve the builtin paths and the intrinsic names.
5. Allocate every record (objects with their slot count, closures with their template,
   generic cells with their tail), then fill them in by field name; symbols get atoms.
6. Rebuild the Map and Set indexes and the RegExp programs; put the template objects back.
7. Install the `Vm` record: interpreter state, host call and handle tables, realm values.

Nothing runs. The VM is idle where the exporter's was and the host resolves its open
calls as usual.

### 3.7 Outside the module

About 40 lines in djs-core, all behind the feature: raw property slot access and
`prop_insert` in heap/object.rs, a symbol's atom in heap/atom.rs, the cfgs of
`CompileOptions::from_bits` and `atom_string_of`, and the feature left out of the build
hash in build.rs. About 70 lines in djs-wasm for its two exports (and js/src/abi.ts's
operation table).

## 4. Results (0300)

All runs native, on the corpus runner's terms (vm/dump_harvest.rs): the corpus prelude,
fake tools answering through the async host function `later`, one call settled per idle
point, oldest first, frozen builtins. "Same" means the run continued from the imported VM
gives the uninterrupted run's transcript (output lines and the final result).

### 4.1 Coverage and equivalence

| corpus | idle points | exported | imported | continue the same | refused |
| --- | ---: | ---: | ---: | ---: | ---: |
| tests/corpus/codemode-harvest (300 cases, 288 with an idle point) | 1,164 | 1,164 | 1,164 | 1,164 | 0 |
| ... chained (a snapshot at every idle point, each run on in the import) | 288 cases | | | 288 | 0 |
| tests/dump and tests/dump/golden programs (45) | 197 | 197 | 197 | 197 | 0 |
| ... chained | 45 programs | | | 45 | 0 |

- Nothing was refused. The programs include sync and async generators, `finally`,
  destructuring, iterator helpers, `using`, and a class with private names, which plain
  dumps refuse (tests/dump/refusals.txt, `golden/class-fields@1`).
- A test round-trips what the corpus does not hold at an idle point: typed arrays,
  `ArrayBuffer`, `DataView`, `WeakMap`/`WeakSet`, `BigInt`, `Date`, local symbols as keys,
  accessors, non-enumerable and frozen properties, sparse arrays, a `Proxy`, a suspended
  sync generator and async generator, a bound function, Map and string iterators
  mid-way, a sticky RegExp with `lastIndex`, a tagged template's identity.
- A negative control (the importer drops elements) makes many runs differ, so the
  comparison sees a broken import.
- Plain dumps also write all 1,164 harvest idle points.

### 4.2 Sizes

At the 1,164 harvest idle points. Both formats carry the corpus prelude's state; the
snapshot also carries the prelude's source (8.3 KB) and the case's (the harvest's fake
results are in it) unless written without sources:

| | median | p90 | p99 | max |
| --- | ---: | ---: | ---: | ---: |
| snapshot, raw | 16.9 KB | 31.2 KB | 163 KB | 164 KB |
| snapshot, zstd -3 | 6.7 KB | 10.4 KB | 25.8 KB | 25.9 KB |
| snapshot without sources, raw | 5.4 KB | 10.7 KB | 69.9 KB | 70.5 KB |
| snapshot without sources, zstd -3 | **2.7 KB** | 4.3 KB | 12.1 KB | 12.2 KB |
| plain dump, raw | 12.8 KB | 28.2 KB | 104 KB | 177 KB |
| plain dump, zstd -3 | **3.4 KB** | 6.3 KB | 14.0 KB | 18.7 KB |
| plain dump, gzip -9 | 3.2 KB | 5.9 KB | 12.2 KB | 17.3 KB |
| `Vm::image`, raw | 239 KB | 288 KB | 481 KB | 501 KB |

Where a snapshot's bytes go (median per section over 1,361 snapshots): scripts 11.3 KB
(or 8 bytes each without sources), types 1.9 KB, strings 1.4 KB, records 1.3 KB,
functions 330 B, the `Vm` record 61 B, builtins 15 B. The schema is a third of a
sourceless snapshot. It is the same for every snapshot of a build: a host could keep it
once per build and name it by its digest.

The records, summed over the 1,361 snapshots: 38,611 ordinary objects, 34,982 closures,
30,714 Boxes, 18,259 `ValueArray`s, 11,582 promises, 8,348 arrays, 4,572 builtin closures
(in 659 snapshots), 3,574 suspended activations, 3,384 host calls, 3,308 builtin records
(in 682 snapshots), then Maps, array iterators, errors and a few others. A typical
snapshot holds 118 records.

**Consecutive snapshots of one run** differ by little: `zstd -3 --patch-from` the previous
snapshot makes the next one 259 bytes (median; p90 450 B, max 1.6 KB) over 876 pairs. The
same for plain dumps gives 191 bytes. Deltas need no format support: the export is
deterministic and breadth-first, so unchanged parts come out the same.

### 4.3 Times

Native release build, machine load 4–5, per idle point:

| | median | p90 | p99 |
| --- | ---: | ---: | ---: |
| export (`Vm::snapshot`) | **50 µs** | 89 µs | 299 µs |
| import (`Vm::from_snapshot`) | **189 µs** | 305 µs | 781 µs |
| ... of which a fresh VM | 3 µs | 5 µs | 7 µs |
| ... of which compiling the two scripts | 131 µs | 219 µs | 526 µs |
| plain dump (`Vm::dump`) | **492 µs** | 940 µs | 2.1 ms |

The graph itself costs about 55 µs of an import. A host that keeps the compiled prelude
(pi's prelude image) would mostly pay that.

### 4.4 Large state

The cases pi hit with dumps, as an async function holding the data across an `await` and
reading it afterwards; 21 runs each, native release:

| case | snapshot raw | zstd -3 | export median (p99) | import median (p99) | dump raw | zstd -3 | dump time | delta to the next idle point |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| object of 1,000 × 100-char strings | 114 KB | 4.3 KB | 0.30 (0.38) ms | 0.14 (0.17) ms | 110 KB | 2.6 KB | 1.6 ms | 56 B |
| array of 10⁶ numbers (⅓ doubles) | 4.9 MB | 2.4 MB | 3.0 (3.5) ms | 6.4 (7.1) ms | 8.3 MB | 1.8 MB | 153 ms | 786 B |
| Map of 10,000 `{i, s}` | 372 KB | 26 KB | 3.5 (4.4) ms | 2.2 (2.7) ms | 377 KB | 17 KB | 12.5 ms | 90 B |
| array of 20,000 log lines | 747 KB | 48 KB | 2.5 (2.7) ms | 0.36 (0.37) ms | 765 KB | 46 KB | 11.5 ms | 118 B |

- Snapshots are 3.5–50× faster to write than dumps and about as large raw (the array:
  0.6×). Compressed they are as large for the log lines and 1.35–1.65× larger for the
  rest: the dump's text repeats more regularly than tagged records do, and a double is 8
  raw bytes where the dump's decimal text compresses better.
- `Vm::image` takes 5–130 µs and restores in 0.1–1.6 ms, at 0.4–8.6 MB raw.
- The delta column is `zstd --patch-from` of the next idle point's snapshot against this
  one: a run that keeps 5 MB of state pays under a kilobyte per call.

### 4.5 Portability

Build A is the prototype (7b2acff4). Build B is A plus a scratch commit that changes what
a snapshot must not depend on, all at once (branch `scratch/0300-perturb`, kept, not for
main):
- three intrinsics inserted at id 0: every intrinsic id moves (`AsyncAwait` 386 → 389);
- two well-known atoms inserted after `""`: every atom id moves (`length` 2 → 4);
- a new builtin function first in the base prelude, installed last: every template and
  object of the base moves (`BASE_END` 180,552 → 181,504; `JSON.parse` at offset 48,864 →
  48,960);
- `BoxCell` gains a field before `VALUE` (offset 8 → 16);
- `HostCall`'s `ID` and `FN` swap offsets, and it gains a field;
- `Interp`'s `HOST_CALL` and `NEXT_CALL_ID` swap offsets.

The build hash (88e0ee33 → 60ba6f4f) and the layout hash change; an image of A does not
load into B. Each build loads the other's 1,361 snapshots (harvest and programs) at their
idle points of its own run, which then continues:

| exported by → imported by | load and continue as the importer's own run | refused |
| --- | ---: | ---: |
| A → B | 1,361 | 0 |
| B → A | 1,361 | 0 |

B → A first refused every snapshot: B's `HostCall` has a field A lacks. The zero-state
rule (3.3) accepts it, since the field is 0. Then the changes a snapshot must notice,
each on top of B, against A's snapshots:

| change | load and continue the same | refused, with the reason |
| --- | ---: | --- |
| two more instructions before every for-in and for-of loop (a lowering change) | 94 | 1,267: "the bytecode of the function at 1..10073 of script 1, which has a stored frame, differs in this build" |
| two more instructions at the start of every function that is neither async nor a generator | **1,361** | 0: no frame is in such a function at an idle point, and their closures keep their interface |
| `Array.prototype.push` renamed | 1,254 (+3 that continue unlike B's own run, +91 at idle points B's own run never reaches) | 13: "no builtin Array.prototype.push in this build" |
| `BoxCell::VALUE` renamed `CONTENT` (a field removed, another added) | 156 (programs without Boxes) | 1,205: "BoxCell: no field VALUE in this build (and the snapshot's is not 0)" |

- The three runs that continue "unlike B's own run" are not import errors: without
  `push`, B's own run had already failed before that idle point, while the snapshot holds
  A's past. A build that behaves differently can only be compared with itself from the
  restore on, as with dumps.
- The for-of change refuses 93% because the codemode body is one async function: a
  lowering change to any construct the body uses reaches the function that waits. That is
  what 2.2's "frames" column counts: since 0200, one merge (0259) did it, for 17 of 331
  scripts.
- Tried and dropped: guarding the builtin closure kinds' positional records (3.8) with a
  hash of each kind's code. It refused 2 snapshots after B (the prelude function table
  numbers moved) and 659 after the sync-function change (the builtins are compiled by the
  same compiler). Code hashes are the wrong guard for builtins.

### 4.6 Complexity

| | lines (code lines) | shipped module, gzip -9 |
| --- | ---: | ---: |
| the snapshot module: export 1,004, import 1,233, mod 301, wire 222, schema 190, hash 140 | 3,090 (2,759) | +59.9 KB (427,888 → 487,758 B) |
| the dumper: vm/dump*.rs without tests, call_site.rs | 21,836 (18,745) | +270.6 KB (427,888 → 698,441 B) |
| ... plus the compiler's suspension maps, compiler/smap*.rs | 2,719 | |

The snapshot's 60 KB were not tuned: they include the intrinsic names (8 KB of strings
that the shipped module otherwise leaves out), the registry walk for builtin paths,
generic Rust with `BTreeMap`s and formatted refusal messages. The dumper's were tuned
(0298, `#[optimize(size)]`).

### 4.7 What surprised

- **It worked at once.** The first complete run covered all 1,164 harvest idle points
  with no refusal. djs's heap was nearly self-describing already: values are tagged, and
  cells are a named, typed layout table. Only the object classes' slots needed names; the
  rest of the per-kind code is small rebuilds.
- **The registers do not need to be understood.** 0141 spent most of its design on
  naming every slot of a suspended frame by role. With the bytecode hash as the guard the
  frame travels as a plain `ValueArray`, and the interface hash covers closures that hold
  no frame.
- **The schema is the largest part** of a small sourceless snapshot (1.9 of 5.4 KB raw).
- **Import is mostly compilation** (70%).
- **Breadth-first graphs need reference encoding to compress**: plain ids made the Map
  case 3.4× larger after zstd.
- **The builtins' records are the soft spot**: half the snapshots hold one (a
  `Promise.all` element, a `finally` closure's data), their slots are positional, and no
  cheap check guards their meaning.

## 5. What it would take to ship, and what it cannot do (0300)

### 5.1 To ship

In the order a release needs them:
1. **The slot table as a checked contract.** `schema.rs`'s class table generated from (or
   tested against) heap/object.rs's constants, so that a slot moved or added shows up as a
   failing test, like the registry's paths.
2. **Named records for builtin closure kinds.** The records that builtin closures hold as
   data (`Promise.all` elements, `finally` data, async-from-sync iterators) get a schema
   per kind: a field list in builtins/*.ts that `NewRecord`/`RecordGet` use by name, and
   that the snapshot writes by name. Until then a builtin change that reorders a record's
   slots goes unnoticed (4.5's last row is why a code hash does not do).
3. **A pin of the corpus's frame bytecode in the gate** (2.7): the bytecode hash of every
   function of the corpus that can hold a frame, as a committed list. A change to it is a
   deliberate decision, with a format note.
4. **Golden snapshots**: a selection exported by each release and committed (a few
   hundred KB), loaded by every later build in the gate, until the format window closes.
   The branch's `snapshot_import_dir` test is the loader this needs.
5. **A host API and pi's adapter**: `Sandbox.snapshot()`, `sandbox.loadSnapshot(bytes)`;
   pi keeps the scripts and passes them by hash; deltas against the previous snapshot
   (`--patch-from`-style, 4.2) on the host side; the schema per build kept apart.
6. **pi's real prelude** in the harvest: its lockdown, its `tools` Proxy and its globals
   have not been through a snapshot (the corpus prelude has no lockdown).
7. **Size work** on the module (4.6), and the `WeakRef`/`FinalizationRegistry` and
   `TextCodec` classes when a host needs them.

Estimate: 1 and 2 are a week each (2 touches every builtin that makes a record); 3–5 a
week together; 6 and 7 a few days each.

### 5.2 What it cannot do

- **Survive a lowering change in a waiting function.** The snapshot refuses with the
  function and the reason; the session needs another way to continue (a replay of its
  host calls, or the old engine). A dump re-enacts the run as JavaScript and survives
  such changes; this is the one thing dumps can do that snapshots cannot.
- **Snapshot anywhere.** Idle points only, as dumps. An image stays the in-process,
  same-build form for arbitrary instructions.
- **Survive renames.** A field, slot or builtin path that is renamed is gone for the
  snapshot (4.5). Keeping old names readable is compatibility code, which djs does not
  write; the alternative is a refusal and the fallback.
- **Guard the builtins' record layouts** without 5.1's named records.
- **Keep the old build's behaviour.** From the restore on, the importer's semantics apply,
  as with dumps and replay.
- **Be byte-identical across a round trip**: the imported heap has its own offsets,
  shapes, collection points and hash key. Nothing a script can observe depends on them
  with frozen builtins and strong weak references; with collected weak references, a
  collection may happen at another point.

### 5.3 Recommendation (0300)

Make logical snapshots pi's persistence format and retire continuation dumps, with a
fallback for refusals:
- Coverage is at least the dumper's (all harvest points, everything the dump programs
  hold, and what dumps refuse), equivalence is exact on the corpus, and the importer
  checks every name it relies on.
- It is 10× faster to write than a dump at typical sizes and 3.5–50× with large state.
  Without its sources (which pi has) it is smaller than a dump: 2.7 against 3.4 KB zstd
  at the median, 12.1 against 14.0 KB at p99; with them about twice. Deltas make the
  per-call cost of a long run a few hundred bytes, for both.
- It is an eighth of the dumper's code, and 60 KB instead of 270 KB of the shipped module.
- It survives every engine change that keeps the waiting functions' bytecode and the
  named schema: intrinsic, atom and opcode numbering, the base layout, cell layouts and
  the code of every function that does not wait. By 2.6 (triaged, 0200..0298) a single
  issue keeps every corpus script loadable 99% of the time, and over 30 issues 97.7% of
  the scripts survive.
- The refusal case needs a fallback, and that is the one real cost of dropping dumps: pi's
  journal replay, which it has, or keeping the previous engine for a deploy. 5.1's gate pin
  keeps that case rare and visible.

Do 5.1's first three items before relying on it across releases; until then, use it
within a build (where it is already a faster, smaller dump) and keep replay as the
cross-release record.

### Reproducing (0300)

On branch `issue/0300-logical-snapshots` (the module, its tests and
`scripts/research/0300/`); outputs go to work/scratch/0300.

```sh
F="-p djs-core --features snapshot --lib"; W=$PWD/work/scratch/0300
# coverage and equivalence (4.1), sizes and times (4.2, 4.3), files for zstd
DJS_SNAP=1 DJS_SNAP_MEASURE=1 DJS_SNAP_OUT=$W/harvest.tsv DJS_SNAP_DIR=$W/snaps \
  cargo test --release $F vm::snapshot::tests::snapshot_ -- --ignored --nocapture
python3 scripts/research/0300/sizes.py $W/harvest.tsv $W/snaps; python3 scripts/research/0300/deltas.py $W/snaps
# large state (4.4)
DJS_SNAP=1 DJS_SNAP_DIR=$W/large cargo test --release $F vm::snapshot::tests::large_state -- --ignored --nocapture
# portability (4.5): build B in a worktree of branch scratch/0300-perturb (or -bytecode,
# -syncfn, -builtin, -field), then load A's snapshots there
DJS_SNAP_IMPORT=$W/snaps cargo test --release $F vm::snapshot::tests::snapshot_import_dir -- --ignored --nocapture
# the shipped module with and without the dumper and the snapshot module (4.6)
scripts/research/0300/wasm-sizes.sh
```
