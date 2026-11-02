# Why a fresh heap is 300 KB, and how to make it nearly empty

Issue 0143, measured at main f0f34fd9 (2026-11-07). Research only: no engine change. The
owner asked:

> Why is our heap out of the box already 300K large? That is silly. Can we not create
> most of those objects lazily on first use? Where is all that going?

The idea behind the question: object layouts (an array, a plain object) don't change between
djs versions. What ties a snapshot to a version is that session objects reference builtins,
prototypes, shapes and globals inside the same heap. So keep those objects out of the heap,
or reference them by stable ids.

## The answer in short

- **Where it goes.** A fresh VM is a copy of the base image: **322 KB in 5,770 cells**. Two
  thirds of it (217 KB, 3,725 cells) are the **874 builtin functions**. Each one is a
  function object, a template, its bytecode and its tables, because djs's builtins are
  written in JavaScript and compiled into the heap. The rest:
  - the atom table and builtin names: ~48 KB;
  - property storage: 36 KB;
  - closure boxes, prototypes and the realm: ~7 KB.
- **Almost none of it is used.** All 300 harvested codemode cases together touch only 105
  of the 874 functions; the median run touches 26.
  - Whole families make up 127 KB (40% of the image) and are never touched by any run:
    typed arrays, iterator helpers, Proxy and Reflect, DisposableStack, generators,
    decorators, WeakMap, WeakRef, BigInt, Symbol and DOMException.
  - Materializing on first use, object by object, a median run would build **~17 KB**
    (p90 25 KB, max 31 KB) instead of copying 322 KB.
- **QuickJS** has 77 KB and 1,097 allocations after `JS_NewContext`, because it creates its
  builtins lazily. Even with every builtin touched it has 2,085 allocations and 125 KB,
  because a builtin there is one object pointing at C code in the module. In djs the code
  lives in each VM's heap.
- **pi makes it worse.** Its prelude's lockdown walks every builtin, freezes it and adds
  override accessors: a pi session starts at **471 KB, 8,370 cells**. Of that, 156 KB
  and 2,600 cells are the lockdown, mostly property tables converted to dictionaries,
  new shapes and accessor closures. Lazy builtins only help pi if lockdown becomes cheap
  for unmaterialized builtins: an engine-level frozen-intrinsics mode.
- **A minimal eager realm** is about 10 KB and ~100 cells if builtin names become static
  atoms in the module. It is ~60 KB and ~1,000 cells if the atom table stays as it is.
  Everything else can wait until first use.
- **Stable ids instead of heap objects** (the owner's idea) is the right model for what a
  snapshot *names*. As a runtime representation it is the most expensive option: two
  object representations on every property path. The portability it would buy is cheaper
  to get at export time (the 0141/0142 work) from a stable builtin registry. Build the
  registry first: the lazy realm needs it too.

Recommended order (section 6):
1. A stable builtin registry.
2. Engine-level lockdown for pi.
3. Static builtin atoms.
4. The lazy realm, object by object, with code copied on first call.

Together they take a fresh VM from 322 KB / 5,770 cells to ~10 KB / ~100 cells, and pi's
starting heap from 471 KB to roughly 30–60 KB. Estimated 8–11 weeks of agent time.

## 1. Method

- **Accounting:** `cargo run -p djs-prelude --example base-families` (new). Every cell of
  the base image gets one *family* and one *part*:
  - **Family**, in this order:
    - what `globalThis` reaches, by the global it hangs off (breadth-first; typed arrays
      and errors grouped);
    - the installed intrinsics (`%InstallIntrinsic`), by the builtins/*.ts file their
      name comes from in builtins/gen/prelude.js;
    - everything else by its nearest owner over references. Helpers captured in closure
      boxes go with the builtin that captures them, and code, pools, property storage
      and constant strings go with their function or object.
  - **Part:** function objects, templates, bytecode, pools, tables, property storage,
    strings and so on.

  Privileged templates have no names, so functions are named by how they are reached.
  "realm" is what only the realm reaches: the atom table, the intrinsics table and 28
  functions referenced from realm slots.
- **What runs touch:** a scratch build instrumented by `scripts/size/patches/touch.py`
  (never merged). It marks the holder and the value of every property read
  (`Heap::prop_read`) and every template entered.
  - All 300 harvested codemode cases were run, one difftest process each, each
    preceded by the corpus prelude.
  - `base-families --touched DIR` maps the marked offsets back to functions, objects and
    families.
  - Limits:
    - inline-cache hits skip `prop_read`, but only after a first miss that marks;
    - the corpus prelude is not pi's (no lockdown, no tools proxy);
    - harvested scripts use fake tools.
- **pi:** `node scripts/heap/reference.ts pi-prelude` with two tools and no globals, then
  `djs heap --diff base.img prelude.img`.
- **QuickJS:** 0135's measurement, `JS_ComputeMemoryUsage` on dquickts's sources (copied,
  built in scratch).

## 2. Where the 322 KB go

By part:

| part | cells | bytes |
| --- | ---: | ---: |
| bytecode | 874 | 99,592 |
| templates | 874 | 48,944 |
| function objects | 874 | 38,920 |
| property storage (tables, shape values, shapes, transitions, accessor pairs) | 754 | 35,848 |
| arrays: atom entries 15.2 KB, atom index 4 KB, intrinsics table 3.5 KB, value stack 2 KB, … | 40 | 33,840 |
| strings (atoms, function names, constant strings) | 904 | 28,704 |
| capture descriptors | 652 | 13,960 |
| constant pools | 394 | 13,512 |
| closure boxes | 257 | 4,112 |
| handler tables | 57 | 2,032 |
| other objects (prototypes, namespaces) | 70 | 1,736 |
| realm, roots, interp, symbols, atom table | 20 | 1,168 |
| **total** | **5,770** | **322,368** |

By family (functions: all / reachable from `globalThis`):

| family | cells | bytes | functions | share of corpus runs touching it |
| --- | ---: | ---: | ---: | ---: |
| typed arrays, ArrayBuffer, DataView | 856 | 44,048 | 137 / 97 | 0% |
| realm (atom table, intrinsics, stack) | 199 | 38,856 | 28 / 0 | 100% |
| Array | 530 | 30,904 | 80 / 50 | 100% |
| String | 314 | 16,672 | 49 / 40 | 100% |
| Iterator helpers | 278 | 15,040 | 41 / 26 | 0% |
| Proxy | 241 | 14,912 | 43 / 2 | 0% |
| Promise | 281 | 14,216 | 52 / 15 | 100% |
| RegExp | 251 | 14,160 | 33 / 21 | 5% |
| Date | 298 | 13,344 | 54 / 49 | 4% |
| DisposableStack | 189 | 10,080 | 32 / 14 | 0% |
| Object | 194 | 10,048 | 30 / 24 | 100% |
| Math | 232 | 9,640 | 38 / 37 | 1% |
| Set | 166 | 8,760 | 26 / 17 | 1% |
| generators | 180 | 8,464 | 29 / 0 | 0% |
| errors | 200 | 8,056 | 23 / 15 | 100% |
| decorators | 116 | 7,184 | 20 / 0 | 0% |
| Reflect | 128 | 6,232 | 22 / 13 | 0% |
| JSON | 86 | 5,688 | 13 / 4 | 100% |
| Map | 114 | 5,480 | 19 / 15 | 100% |
| Number | 102 | 4,584 | 17 / 15 | 2% |
| globalThis's own (`globalThis` family) | 59 | 4,560 | 9 / 8 | 0.3% |
| atom-only names | 147 | 4,208 | – | – |
| WeakMap/WeakSet, DOMException, Symbol, BigInt, Function, WeakRef, URI, atob/btoa, … | 486 | 21,432 | 60 / 44 | 0% |
| shared iteration and conversion helpers | 74 | 4,224 | 15 / 0 | 100% |
| Boolean | 49 | 1,576 | 4 / 3 | 0.7% |

How to read it:
- About **60% of a family's bytes are code** (templates, bytecode, pools, descriptors).
  Function objects are 12%, property storage 11%, strings 9%.
- **Internal helpers** (365 of the 874 functions) are counted with the family whose code
  they serve: Proxy's traps, Promise's resolve functions, iterator steps.
- **"100%" families** are touched by every run largely through the corpus prelude itself
  (`JSON`, `Array`, `Object`, `Promise`, `Map`, errors, `String`). A family-granular lazy
  scheme therefore saves little: a median run would still materialize 134 KB.
  Object-granular or function-granular materialization is what pays.

## 3. What runs actually use

300 harvested codemode cases, each run after the corpus prelude:

| | median | p90 | max | all runs together |
| --- | ---: | ---: | ---: | ---: |
| base functions touched | 26 | 32 | 50 | 105 of 874 |
| materialized, function by function (the function, its template and code) | 9.8 KB | 13.3 KB | 19.7 KB | |
| materialized, object by object (a touched holder with all its function objects, code only for the functions used) | 17.3 KB | 24.7 KB | 31.4 KB | |
| materialized, family by family | 134 KB | 143 KB | 162 KB | |

- **The corpus prelude alone** touches 21 functions and 7 objects: 15.5 KB object by
  object. 22 functions are touched by at least 90% of runs.
- **Static usage agrees** (docs/codemode-feature-usage.md, 2,535 real scripts): no
  classes, no Proxy or Reflect, no typed arrays, no generators, no WeakMap, no BigInt,
  no Symbol, no Iterator helpers. Scripts use a few dozen string, array, `Object.*`,
  JSON, Promise, Date and Math members.

## 4. pi's prelude, and lockdown

pi's prelude runs first in every pi session. It does three things that matter here:
- **Freezes every builtin.** It walks all objects reachable from the globals and the
  hidden intrinsics, about 700, calls `Object.getOwnPropertyDescriptors` on each and then
  `Object.freeze`.
- **Rewrites overridable properties.** For `constructor`, `name`, `message`, `toString`,
  `toLocaleString`, `valueOf`, `toJSON`, and everything on `Object.prototype`, it replaces
  the data property with a getter/setter pair (two closures) to avoid the override
  mistake.
- **Installs `tools`, the store and its helpers.**

Measured (two tools, no globals): the session's starting image is **471 KB in 8,370
cells**, 156 KB and 2,600 cells more than the base:

| what lockdown and the prelude add | cells | bytes |
| --- | ---: | ---: |
| property tables (frozen builtins go to dictionary mode) | 709 new, 78 changed | 76 KB |
| shapes and transition arrays | 1,107 | 26 KB |
| strings | 300 | 27 KB |
| override accessors: functions, boxes, accessor pairs | ~680 | ~20 KB |
| the prelude's own code, `tools` proxy, `image` helper | ~300 | ~25 KB |

So under lazy builtins, today's lockdown would materialize every builtin and convert it
to a dictionary: a lazy realm by itself does nothing for pi. Two ways out:
- **Engine-level lockdown (recommended).** A VM option, `frozen_intrinsics`:
  - every builtin is born frozen, with no per-object work;
  - assigning an inherited read-only *builtin* property on an ordinary object defines an
    own property on the receiver instead of throwing (the override-mistake fix that SES
    and Node's `--frozen-intrinsics` implement), for the same keys pi uses.

  pi's prelude then drops its lockdown function (a change in pi-private, the owner's
  call). Gains:
  - about −120 KB and −2,300 cells per pi session even with today's eager realm;
  - no lockdown walk at pi's cold start;
  - lazy builtins become possible for pi.

  The one observable difference: the overridable properties stay data properties (read
  only) instead of becoming accessor pairs. `getOwnPropertyDescriptor` shows the
  difference; ordinary code doesn't.
- **Lazy freeze in JS.** Keep pi's lockdown but freeze on materialization. That needs an
  engine hook anyway, and keeps the accessor-pair cost for every materialized builtin.
  Worse on every count.

## 5. Designs

### 5.1 The minimal eager realm

What must exist at `vm_new`:
- the roots, the interpreter cell and the realm (with its intrinsics table, whose slots
  can start empty);
- the global object with its property table (about 70 globals, every value a lazy
  slot);
- the value stack;
- the well-known symbols;
- the atom table.

The prototypes the engine uses implicitly (Object, Function, Array, Error, Promise,
iterator and generator prototypes) can be lazy stubs too: an empty object with a "lazy"
flag and a descriptor id, materialized at the first property access or the first
`[[Prototype]]` walk that needs a property.

| eager realm | cells | bytes |
| --- | ---: | ---: |
| roots, interp, realm, symbols, intrinsics table | ~25 | ~5 KB |
| global object and its property table | 3 | ~2.2 KB |
| value stack | 1 | 2 KB |
| ~70 lazy holder stubs (constructors, prototypes, namespaces) | ~70 | ~2.5 KB |
| atom table with builtin names as today (entries 15 KB, index 4 KB, 904 strings 29 KB) | ~900 | ~48 KB |
| **total, atoms in the heap** | **~1,000** | **~60 KB** |
| **total, builtin atoms static in the module** | **~100** | **~10 KB** |

**Static builtin atoms.** Builtin names become predefined atoms, as QuickJS's `JS_ATOM_*`:
- the strings and the hash index live in the module's data;
- atom ids below a fixed bound are static, and session atoms start above it;
- an atom's string *value* (needed when a key flows into JS, `Object.keys`) is
  materialized into the heap on first use and cached in a small per-VM table.

They are also stable ids, which section 5.4 needs.

### 5.2 Lazy materialization in the heap (QuickJS's autoinit, object by object)

- **Descriptors.** bgen already knows, per builtin object, its properties: key,
  attributes, and value (a method by function index, an accessor, a constant). It emits
  them as static tables in the module (like QuickJS's `JSCFunctionListEntry`).
- **Materialization.** A lazy holder (a flag in the object header plus a descriptor id)
  is materialized on its first own-property operation: get or has on a miss, define,
  delete, ownKeys, freeze, preventExtensions, getOwnPropertyDescriptor. It becomes an
  ordinary object with all its function objects. A function object starts with a lazy
  template reference.
- **Code.** A function's template and code are copied from module data into the arena on
  its first call. This is 0138's lazy-copy design, at frame entry, with the allocation in
  the call's retry path (D1700) or in a native builtin's (D1900). Internal helpers
  (installed intrinsics, boxes captured by builtins) are materialized the same way on
  their first call.
- **Size and time.**
  - A median run materializes ~17 KB: ~7 holders and ~26 functions' code.
  - `vm_new` copies ~10–60 KB instead of 322 KB: ~0.3–0.8 µs instead of 3.9 µs.
  - Materializing costs more than copying the same bytes: per holder, a few dozen small
    allocations from a descriptor, est. 2–5 µs in wasm. A median run spends est. 20–40 µs
    on it, against ~3 µs saved on `vm_new`.
  - That is noise against a codemode script's milliseconds, but it is a time *cost*: the
    win is memory, cells and image size. Prebuilt relocatable chunks (a holder's cells
    laid out at build time, copied and fixed up on materialization) would make it a few
    memcpys, as a later step.
- **Interpreter.** One flag test where an own-property lookup misses (the header is
  already loaded), and one at frame entry for a lazy template. Est. ≤0.5% instructions
  (0138 measured +0–0.9% for the frame-entry check alone).
- **Determinism.** Materialization happens at deterministic points (the program's own
  property accesses and calls), in a deterministic order. Raw images stay equal for equal
  histories, `canonical_hash` is unaffected, and suspension/restore round trips carry the
  materialized state in the image. Heap layouts now depend on which builtins a script
  touched, which images already accept (they are per build and per history).
- **Identity and observability.** A method is created once per VM and stored in its
  holder, so `Array.prototype.map === [].map` holds. Property order is the descriptor's.
  `fn.name`, `length` and `toString` come from the descriptor. Nothing observable
  changes.
- **Snapshots.** Images shrink by what a session didn't touch. Containers against a
  smaller base are about as small as today (0.8 KB for an empty Sandbox after 0136),
  since what the session materialized is new data either way.

### 5.3 Builtin code out of the heap, shared (0138)

0138 studied moving templates and code (178 KB, 2,849 cells) into a read-only segment
shared by all VMs of a module. It recommended against: for one VM per instance the memory
gain was small next to the cost of an extra code space on every access path.

Under the owner's goals:
- **Memory and cells.** Section 5.2's lazy copy keeps the per-VM cost to the code
  actually used (≤ 20 KB), which is most of 0138's gain without a second code space. The
  module data that lazy copies come from *is* the shared segment, read only at
  materialization.
- **Several VMs per instance** still multiply the gain of executing in place. That
  remains a reason to build 0138's segment later, on top of 5.2.
- **Snapshot size:** no difference; snapshots are already 0.8–9 KB (D2260, 0136).
- **Portability:** no difference by itself. Code in the segment is still addressed by
  build-specific offsets.

**Verdict:** take 0138's lazy-copy mechanism as part of 5.2; don't build the execute-in-place
segment until VMs share instances.

### 5.4 Stable ids instead of heap references (the owner's idea)

The model: builtins never live in the heap. A `Value` can name a builtin by a stable id
(a registry: spec path → id, append-only across versions). A session's mutations of
builtins live in a per-VM overlay: added properties, frozen bits, a replaced prototype.
- **What it buys.**
  - A session image holds only session-made cells and the overlay, so it is small and
    references nothing build-specific *about builtins*.
  - Freezing everything is a per-VM bitset (874 bits).
- **What it costs.**
  - A second object representation that every property path must handle. Lookups that
    walk into `Array.prototype` (every array method call) go to static descriptors; inline
    caches must key static holders; `ownKeys`, `defineProperty`, `getPrototypeOf`,
    equality, the wire format, the heap reader, the verifier and the collectors all see
    two kinds of objects.
  - Estimated 3–4 months, a measurable cost on method calls unless ICs cover static
    holders well, and high risk (the object model is the engine's core).
- **What it does not buy.** Version-portable images. Session cells still hold user
  bytecode, templates, frames, shapes and compiler-specific layouts that change across
  builds. docs/portable-snapshots.md (0141) shows portability needs a by-meaning export
  at quiescent points anyway, and that export already names builtins by spec path.
- **What to take from it.** The *registry*: one stable, append-only table of builtin
  identities (spec path → id), generated by bgen and checked by the gate:
  - section 5.2's descriptors use it (descriptor ids);
  - static atoms use it (builtin name atoms);
  - snapshot export uses it (0141/0142 name builtins by it instead of by path strings);
  - session-made objects can then reference builtins *in exports* by stable id.

  This gets the portability the owner wants, at the layer where it is needed, without a
  second object model.

## 6. Ranked plan

| step | fresh VM (cells / bytes) | pi session start | vm_new | interpreter | module | effort | risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| today | 5,770 / 322 KB | 8,370 / 471 KB | 3.9 µs | – | – | – | – |
| 1. stable builtin registry (bgen-generated, append-only, gate-checked) | – | – | – | – | +1–2 KB data | 1 week | low |
| 2. engine-level lockdown (`frozen_intrinsics` + override fix); pi drops its lockdown | unchanged | ~5,900 / ~350 KB | – | one branch on a failed inherited set | +1–2 KB | 1–2 weeks (+ pi-private change) | low–medium |
| 3. static builtin atoms (predefined atoms in module data, lazy atom strings) | ~4,900 / ~274 KB | −48 KB | −0.6 µs | atom lookups check the static table first | ±0 (moves data) | 2 weeks | medium |
| 4. lazy realm: descriptors, lazy holders, lazy code copy (5.2) | **~100 / ~10 KB** | **~30–60 KB** (prelude + what it touches) | ~0.3 µs, plus est. 20–40 µs of materialization spread over a run | ≤0.5% | ±0 to −20 KB (descriptors replace the coded base image) | 4–6 weeks | medium–high |
| 5. execute-in-place shared segment (0138) | – | – | – | +0–0.9% | +2–3 KB | 4–5 weeks | medium–high |
| (not recommended) stable ids as a runtime representation (5.4) | ~100 / ~10 KB | – | – | +? on method calls | – | 3–4 months | high |

- **Order.** 1 is a prerequisite that also serves 0141/0142. 2 helps pi immediately and is
  required before 4 helps pi. 3 removes the largest eager piece that 4 cannot. 4 delivers
  the owner's "nearly empty heap". 5 only when several VMs share an instance.
- **Steps 1–4:** est. 8–11 weeks. Afterwards:
  - the docs' "6,000 cells" becomes ~100;
  - a codemode run's heap holds the ~20 KB of builtins it used;
  - pi sessions start at roughly a tenth of today's heap.
- **Snapshot sizes** don't change much (containers are already tiny). Raw images and
  memory do.
- **Measure first:** before 4, prototype one family (Array) as lazy holders. That measures
  materialization time and the interpreter flag test with real numbers, in about a
  week, and settles the 20–40 µs estimate.

## 7. Notes for the portable-snapshot work (0142)

- **Builtins in the base image** are ~5,800 cells today and ~8,400 in a pi session, but a
  session touches about 30 functions and 7 holders of them. An export that names builtins
  by registry id (step 1) instead of by path strings or offsets is small and survives any
  build that keeps the registry.
- **pi's lockdown** rewrites every builtin (dictionary mode, accessor pairs). In a
  by-meaning export, the builtins' frozen state and override accessors are a property of
  the *prelude*, not of the session. With engine-level lockdown (step 2) they become one
  VM option instead of 2,600 cells.

## Reproducing

```sh
# the base by family and part
cargo run --release -p djs-prelude --example base-families
# what runs touch: an instrumented scratch copy
rsync -a --exclude target --exclude work --exclude .git ./ /tmp/touch/
(cd /tmp/touch && python3 scripts/size/patches/touch.py && cargo build --release -p djs-difftest)
for f in tests/corpus/codemode-harvest/*.js; do n=$(basename $f .js)
  DJS_TOUCH_OUT=/tmp/out/$n.txt /tmp/touch/target/release/djs-difftest --engine djs -j 1 codemode-harvest/$n.js
done
(cd /tmp/touch && cargo run --release -p djs-prelude --example base-families -- --touched /tmp/out)
# pi's starting image against the base
node scripts/heap/reference.ts base -o base.img
node scripts/heap/reference.ts pi-prelude --data start.json -o prelude.img
cargo run --release -p djs-cli -- heap --only summary,kinds,diff --diff base.img prelude.img
```
