# Builtin code in a shared read-only segment: what it would take (0138)

Status: design study for the owner, not built. Prototypes and raw numbers are in
work/scratch/0138 (`proto-eager.patch`, `proto-lazy.patch`).

**Recommendation: don't build it now.** For pi's deployment (one VM per wasm instance) it
saves about 0.18–0.43 MiB of wasm memory per instance. It costs:
- a change in every place that reads a function template;
- a change to the image format;
- and either +1–2.4% interpreter instructions or a lazy-copy mechanism.

A much smaller change gets a comparable memory win: decode the base straight into the new
VM's arena instead of keeping a decoded copy per instance, −0.31 MiB, section 8. Do that
first. Revisit the segment if several VMs share an instance, where the saving multiplies by
the number of VMs, or if per-VM memory has to drop well below 2 MiB. If it is built, build
the lazy-copy variant (section 3.2), in the phases of section 7.

## 1. What would move

Measured on the current base image (322 KB, 5,768 cells):

| cells owned by the 874 builtin templates | cells | bytes |
| --- | ---: | ---: |
| FuncTemplate | 874 | 48,944 |
| bytecode (`Bytes`, CODE) | 874 | 99,216 |
| capture descriptors (`U32Array`) | 653 | 13,992 |
| constant pools (`ValueArray`) | 391 | 13,432 |
| handler tables (`Bytes`) | 57 | 2,032 |
| **total** | **2,849** | **177,616** |

- **What stays:** the 874 function objects (38,952 B), prototypes, PropTables, shapes,
  atoms, strings and the realm, about 144 KB in 2,919 cells per VM.
- **Constant strings:** the constant pools reference 367 strings (13.0 KB). None of
  them reference anything but strings.
- **Children:** privileged templates have no children: the prelude's top-level template
  is dropped after it ran.

## 2. What must stay per VM, and what that changes first

A shared cell must never be written by a VM, and must never point at a cell whose offset
differs between VMs.
- **Inline caches.** `FuncTemplate::IC` is set lazily, to a per-template `U32Array`
  allocated at the first fill (interp/run/ic.rs), and its slots hold arena offsets of
  shapes and prototypes. Shared templates need their caches elsewhere: a per-VM table
  indexed by a template id stored in the shared template (a `RefArray` in the realm),
  one extra load per cache lookup of builtin code. `clear_offset_caches` would walk that
  table instead of the templates.
- **Tagged-template caches** write template objects into constant pools (D1621's
  sibling, `consts` writes in run.rs). Privileged code would have to be compiled without
  them, and the build would assert it.
- **Constant strings** are JS values that flow everywhere (a builtin returns `"undefined"`).
  A string in the shared segment would make every string access in the engine check
  which space it reads (`str_view`, hashing, comparisons, the wire, the reader), so the
  strings stay in the arena. A shared constant pool then holds arena offsets, which is
  only sound if those strings sit at the same offset in every VM forever: a **pinned
  prefix**. The base image puts them right after the roots cell, and no collection ever
  moves or frees cells below its end P. They are live anyway: 1,946 of the base's
  strings are atoms (never collected, D1054), and the shared pools are roots for the
  others.
- **Atoms** are ids, not offsets. A VM's atom table starts as the base's, so the ids in
  shared bytecode mean the same in every VM. Nothing to change.
- **Function objects** keep identity and properties per VM. Their slot 0 (the template)
  becomes a shared reference.
- **Shapes** are not referenced from templates (only from caches, which move out).

## 3. The reference encoding and the access paths

**Encoding.** Arena offsets are `u32`, and the arena's limit is `MAX_ARENA` (4 GiB).
- Capping the arena at 2 GiB frees the top bit: offsets ≥ `RO_BASE = 0x8000_0000` name
  the shared segment, at `off - RO_BASE`.
- A `Value` keeps its tag (INTERNAL for templates); a `Ref` is the same `u32`.
- A wasm32 VM cannot reach 2 GiB of arena anyway (the page allocator, D2053, and the
  memory cap come first).

**Who dereferences such a reference:**
- frame entry (`enter_template`, `template_head`, and the hot loop's returns), which reads
  FLAGS..CONSTS, the code length and NATIVE/FAST;
- the bytecode fetch (`run_hot`'s `word`, `Exec::byte`/`imm_*`, 14 sites);
- constant loads (`constant`, the hot `PushConst`);
- closure creation (CAPTURE_DESC), exception unwinding (HANDLERS);
- `template_of` and its 11 users: stack traces, names, `Function.prototype.toString`,
  the debugger and disassembler;
- in all, 116 `FuncTemplate::` reads in 19 files, plus the collectors, the verifier,
  `canonical_hash`, the heap analyzer and the host-side reader (js/src/reader.ts).

### 3.1 Eager: dispatch every access

Every code and template read checks `>= RO_BASE` and reads the shared bytes or the
arena. Prototype (`proto-eager.patch`):
- the fetch, the immediates, constants, template heads and code lengths dispatch to a
  stand-in segment;
- the collector skips shared offsets;
- the cache lookup branches.

All workload code is in the arena, so this measures the dispatch alone. Wasm
instructions, current main → prototype:

| workload | Δ instructions |
| --- | ---: |
| loop | +2.4% |
| calls / call1 | +1.4% / +1.3% |
| arrays | +1.3% |
| props | +1.0% |
| forof | +0.9% |
| higher-order | +0.6% |
| pi-like, strbuild, json, jsonh, strings | +0.1 to +0.7% |

The fetch is one branch per instruction in `run_hot`. Avoiding it means two copies of
the hot loop, one per code space: about 15–20 KB of code, against a size budget that is
already over 1.5× quickjs-wasi's.

### 3.2 Lazy copy: dispatch at frame entry only

A shared template's bytecode, handlers and constants are copied into the VM's arena the
first time the VM calls the builtin. The per-VM table of 3.1's caches also holds the
copied code's offset. The fetch, the immediates and the constant loads stay arena-only,
exactly as today. Only frame entry and `template_of` check for a shared template, then
take the copy's offset from the table.

Prototype (`proto-lazy.patch`, the same minus the per-instruction branches): loop +0.5%,
call1 +0.9%, calls +0.6%, forof +0.4%, props and arrays +0.3%, the rest within 0.1%.
- **Cost:** per used builtin, ~200 B on its first call (pi-like touches a few dozen).
  The copies are ordinary young cells: images hold only the code a session used, and a
  full collection may free a copy that is no longer referenced from the table. The table
  is a root, so it isn't freed.
- **The catch:** the first call allocates. Frame entry today cannot fail with out of
  memory, so the copy has to sit where an allocation may fail and retry: the call
  instruction's retry path (D1700) or native builtins' (D1900).

## 4. GC, snapshots, determinism

- **Collector invariants:**
  - shared cells are never marked, moved or freed: `mark`, the old walk and the update
    pass skip offsets ≥ `RO_BASE` (in the prototype: one compare in each);
  - no arena cell below P moves, so a full collection is `mark_compact` from P, with the
    machinery minor collections use (D2041);
  - the pinned strings are leaves, so nothing in them needs tracing;
  - shared cells point only into the segment or at pinned strings, so they need no
    tracing either.
- **Snapshots:**
  - an image stays the arena;
  - shared references resolve against the module that wrote it, and images already load
    only into the same build (D1320) and record their reference by hash (D2260);
  - the base image splits into the segment (in the module's data, decoded once per
    instance like the base, D2252) and the per-VM base, about 144 KB;
  - containers compressed against the per-VM base stay as small as now (0136).
- **Determinism:**
  - the segment is a function of the build;
  - the lazy copies happen at deterministic points (first calls), so raw images remain
    equal for equal histories;
  - `canonical_hash` (Cheney on a copy) must copy [0, P) verbatim first and skip shared
    references, the Cheney variant of docs/old-generation.md §4.
- **Verification:** `Heap::verify` accepts shared references whose target is a cell of
  the right kind in the segment. The segment itself is verified at build time.

## 5. Lazy builtins and pi's prelude

- **Lazy builtins** (0135 proposal 4: materialize a constructor's methods on first
  access) are independent. Without the segment, materializing a builtin still copies its
  code cells; with it, a materialized builtin is one function object. Together they would
  bring a fresh VM to a few hundred cells. Each has its own risk: lazy property kinds in
  object/, and the layout depending on which builtins a script touched (fine for
  snapshots, which pin the build).
- **pi's sealed prelude** is ordinary user code compiled at run time, one copy per
  instance, so there is nothing to share between VMs. A second segment per instance would
  only exempt the prelude's code from collection, and the old region already does that
  (D2041, D2055). Not worth it.

## 6. Expected gains

Per VM, from the measurements above (estimates where marked):

| | today | with the segment |
| --- | ---: | ---: |
| base copied into the arena | 322 KB, 5,768 cells | ~144 KB, ~2,919 cells |
| arena after `vm_new` (base + room to the first threshold) | ~704 KB | ~448 KB (est.) |
| `vm_new` copy | 3.9–4.0 µs | ~2 µs (est.) |
| wasm memory per instance (one VM) | ~1.6 MiB (small script) | ~1.2–1.4 MiB (est.) |
| old-region walk per minor collection | 17,110 slots | ~10,000 slots (est.) |
| raw image of a fresh VM | 329 KB | ~151 KB |
| compressed snapshot container | 0.8 KB | ~0.8 KB |
| module size | — | about +2–3 KB code (est.: dispatch, the table, copying, verifier) |

Interpreter instructions: +1–2.4% eager, +0–0.9% lazy (section 3).

For pi, one VM per instance: −0.18 to −0.43 MiB of ~2.0–2.25 MiB per session (8–19%).
With N VMs per instance, the segment is shared and the saving grows to N × 178 KB.

## 7. Phases, if built (lazy variant)

1. **Per-VM caches** (1 week): caches out of templates into a per-VM table by template
   id, and no tagged-template caches in privileged code. Measurable alone (one more load
   per cache lookup).
2. **The split and the pinned prefix** (2 weeks):
   - the base build partitions the image into segment and per-VM base;
   - constant strings go to a pinned prefix, and full collections never compact below P;
   - `RO_BASE` references are understood by the verifier, the collectors,
     `canonical_hash`, `djs heap` and the host reader.
3. **Access paths** (2–3 weeks):
   - frame entry and `template_of` dispatch, with the lazy copy on first call and its
     retry;
   - an audit of the 116 template reads;
   - full test262, difftest with determinism and stress, and the native-vs-wasm image
     checks.
4. **Measurement and docs** (1 week): all workloads, mem-perturb (D2048), pi sessions,
   containers, the guide.

**Total:** 6–7 weeks of agent time.

**Risk: medium-high.**
- It changes the interpreter's frame entry (perf's area), the image layout (snapshot's),
  the host reader and every template read.
- A missed shared-write path is a cross-VM corruption bug in a multi-VM instance, and in
  a single-VM instance a write to the read-only data section. Wasm does not trap on that,
  so it would be silent.
- The pinned prefix adds a second boundary to the collector.

## 8. A cheaper alternative for the memory

Each instance keeps its decoded base (322 KB, 5 pages) for as long as it lives (D2252):
- `vm_new` copies it into the arena;
- `djs_base_ptr` hands it to the host once;
- the host keeps its own copy per module for snapshot dictionaries and injection
  (js/src/snapshot.ts).

With one VM per instance, the instance-side copy is used once. Decoding or injecting the
base straight into the new VM's arena (as `djs_image_staging` does for restored images,
D1942), without a long-lived decoded copy, would save ~0.31 MiB per instance for every
session. That is comparable to the segment's −0.18 to −0.43 MiB, with no interpreter,
collector or image-format change. Estimated at a few days in djs-wasm and js/src/snapshot.ts,
plus `vm_new` paths for a second VM in the same instance (decode again or keep the copy
only then).
