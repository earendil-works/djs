# Heap snapshots revisited

Status: research for the owner (issues 0299 and 0300). Nothing here changes the engine on
main. Sections 1–2 (0299): what psychoscript's snapshots are, and how often djs's
history would have broken a snapshot keyed by bytecode hashes and names instead of the
build. Sections 3–5 (0300): the prototype of such a snapshot for djs, its coverage, sizes
and the portability experiment. The summary is the coordinator's.

The question: continuation dumps (vm/dump*.rs, ~26.5k lines of Rust, ~270 KB gz of the
shipped module) re-enact a waiting run as JavaScript so that later builds can load it.
psychoscript persists runs as ordinary heap snapshots instead: a graph of records, code
never saved but compiled again on load and checked by a bytecode hash. Can a djs heap
snapshot survive upgrades in more situations than "same build", without the dump
machinery, and how small can it be?

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

## 3. (0300)

## 4. (0300)

## 5. (0300)
