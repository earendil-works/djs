# 0135: Why are REPL heap dumps ~450 KB and the docs' heaps ~6000 cells?

status: done
agent: bloat
branch: issue/0135-heap-dump-size
depends-on: 0117, 0127
guide: docs/guide/snapshots.md, gc.md, the live example Heap tab text
touches: crates/djs-cli/src/repl/ (.snapshot/.heap), crates/djs-docs/assets/live.js (Heap tab), js/src/ (Sandbox bytes/heap stats if needed), research notes; base-image content changes only as proposals unless small. Coordinate with snapshot (0133, the heap dump analyzer, same area) and heap (0134, heap/)

## Goal
Owner: "why are the heap dumps we get from the REPL ~450 KB large? Also the heap dumps in
the docs show almost 6000 active cells. That's silly."

## Questions
- What exactly does the REPL's `.snapshot` write (raw image? collected? container?) and
  where do its ~450 KB go (base realm vs user data vs garbage vs slack)? Same for the docs'
  Heap tab: what are the ~6000 live cells (builtin functions, prototypes, shapes,
  atoms, templates...) by kind/class?
- Why does a fresh VM have ~6000 live cells, what would QuickJS have after
  JS_NewContext for comparison, and which of them are needed?
- Fixes, in two kinds:
  1. presentation/format: REPL dumps written as compact containers (0117) by default,
     sizes reported as "own data vs shared base", the Heap tab showing the VM's own cells
     separately from the base realm;
  2. substance: fewer cells in the base realm (e.g. lazily materialized builtins/prototypes,
     shared native function cells, atom/shape reductions), with estimates of cells and
     bytes saved and the cost to vm_new and determinism.

## Acceptance criteria
- [x] A clear answer with numbers (use 0133's analyzer if it has landed, or the REPL's
      `.heap` and the walker).
- [x] Land the presentation fixes (REPL default, Heap tab) if they're small.
- [x] Substance changes as a ranked proposal for the coordinator/owner, not built here
      unless trivial.

## Log
- Measured with the REPL's `.heap`/`.snapshot`, perf's image-kinds.py, a scratch walker
  (crates/djs-prelude/examples, not committed) and a QuickJS memory-usage program over
  dquickts's sources (copied to work/scratch/0123, never built in place).
- Presentation (D2340): the docs Heap tab shows the program's cells apart from the base
  realm (live.js; guide arena.md and gc.md sentences updated). After snapshot's 0133
  landed its container module and analyzer, the REPL uses them: `.snapshot` writes a
  container (new app 3 `repl`, docs/host.md) compressed against the base, `--raw` the
  image; `.restore` reads both; `.heap [sections]` is the analyzer against the base
  (default summary + diff). Removed the container/analyzer `#[allow(dead_code)]`s. A
  small REPL session: 16.4 KB (container) vs 341 KB (raw); the analyzer's diff names the
  atom entries array (30.3 KB of the 34.6 KB that differs) as proposal 1's target.

## Findings (main b8fb03ae; native REPL, same image as the wasm module's base)

### What `.snapshot` writes
A raw heap image after a full collection (`Vm::bytes()`), no container, no compression:
- a fresh REPL session: **340,784 bytes, 5,803 cells**; with `let o = {a: [1,2,3], s:
  "hello"}`: 341,368 (+584); after creating and dropping 20,000 objects: 341,200 (the
  garbage is collected; it was 2.5 MB while alive);
- of that: the base realm **321,960 bytes, 5,768 cells** (94.5%); the atom table's entries
  array reallocated by the first new atom at double capacity: **+15.5 KB, all zeros**
  (1,940 → 3,880 slots); the global object's property table doubled (+1 KB); the REPL's
  console functions and the user's data: well under 1 KB. No garbage, no arena slack (the
  arena's 720 KB capacity is not written).
- The owner's ~450 KB is older: the base image was 385 KB before 0127 removed the
  prelude's internal names, plus the doubled atom table; today it is 341 KB.
- As a 0117 container (zstd -3 against the base, `zstd --patch-from`): **16.2 KB** for the
  fresh session, 16.5 KB with the small object, 161 KB with 20,000 live objects. Plain
  zstd without the reference: 75 KB.
- Why 16 KB and not ~1 KB: only 15.6% of the base's 8-byte words stay in place. The first
  cell that grows (the global object's property table at offset 69,856; then the atom
  entries at 87,280 and the atom hash index) moves to the end; the compaction slides
  everything after it down, and every reference to a moved cell changes, so 941 of the
  950 objects and all 874 function templates differ from the base (snapshot's 0133
  analyzer: 30.3 of the 40 KB "changed" in a REPL dump is the atom entries array).

### The ~6,000 live cells (docs Heap tab, `.heap`)
Almost all of them are the base realm, which every VM starts as a copy of:

| what | cells | bytes |
| --- | ---: | ---: |
| builtin functions: function objects | 874 | 38,952 |
| their templates | 874 | 48,944 |
| their bytecode, constant pools, capture descriptors, handler tables | 1,975 | 128,672 |
| strings (atoms: property and function names, messages) | 904 | 28,704 |
| property storage: shape values, dictionary tables, shapes, transitions, accessors | 754 | 35,848 |
| boxes (closure-captured prelude variables) | 257 | 4,112 |
| atom table (entries, hash index), value stack, other arrays | 30 | ~33,000 |
| prototypes and namespace objects, realm, roots, symbols | ~100 | ~3,700 |
| **total** | **5,768** | **321,960** |

So 3,723 cells (217 KB, 67%) are "a builtin function": an object, a template, its code
and its tables, because builtins are written in JS and compiled into the image. Of the
874 functions, 509 are reachable from `globalThis` (methods, constructors, accessors);
365 are internal prelude helpers (`%ProxyGet`, iterator steps, species lookups).

### QuickJS after JS_NewContext (dquickts, native, JS_ComputeMemoryUsage)
- right after `JS_NewContext`: **1,097 allocations, 77 KB**: 183 objects (117 C
  function objects), 598 atoms, 123 shapes, 1,021 properties. QuickJS creates most
  builtin functions and prototypes lazily (`JS_PROP_AUTOINIT`): a constructor's
  prototype and methods exist only once touched;
- after touching every builtin (a walk over `globalThis`): **2,085 allocations, 125 KB**:
  631 objects, 557 C function objects, 677 atoms, 131 shapes.
- A builtin in QuickJS is one object pointing at C code in the module; in djs it is four
  to five cells (object, template, bytecode, tables) in each VM's heap. With everything
  materialized, djs has 1.4x QuickJS's objects (944 vs 631, the 365 internal helpers), but
  2.8x the cells and 2.6x the bytes, because code and tables live in the heap.

## Proposal (substance, not built here)
Ranked by what they do for dump size and cell counts; the base image is perf's and
heap's area.

1. **Growable base cells last, with slack** (dump size). Order the base image so the
   cells a session grows first (the atom entries array and hash index, the global
   object's property table, the root shape's transitions, the value stack) are the last
   cells of the image, and give them a little room (e.g. +256 atoms, +32 global
   properties: ~4 KB raw, ~0 gzip). A grown cell then leaves a hole at the end instead of
   shifting 250 KB of base cells, and the session's container holds only its own data:
   est. **16 KB → 1–3 KB** for a fresh REPL/Sandbox (D2260 measured 33 KB for a new
   Sandbox for the same reason). vm_new: unchanged (same memcpy, +4 KB). Determinism:
   none affected (build-time order). Cost: small, in compact_for_image's ordering (D2251).
2. **Atom table growth by +25% (or +256) instead of doubling** (raw dumps): −15 KB of
   zeros in every raw image after the first new atom; no effect on compressed dumps.
   Cost: more reallocations for programs that intern many atoms (amortized O(n) either
   way with a proportional step).
3. **Builtin code outside the per-VM arena** (cells, memory): templates, bytecode,
   constant pools and tables of privileged code (1,975 + 874 cells, 178 KB) in a
   read-only segment shared by every VM of the module, referenced from the arena by
   offsets above a boundary. A fresh VM: 5,768 → ~2,900 cells, 322 → ~144 KB, vm_new
   memcpy halves (~3.6 → ~2 µs), raw dumps shrink by 178 KB. Cost: every code access
   distinguishes the two spaces (the interpreter loads code through a pointer it already
   holds, so mostly frame setup), the collector skips the segment, images name it by the
   build hash (already pinned, D2260). A large change in heap/ and bytecode/; the biggest
   cell and memory win.
4. **Lazily materialized builtins** (QuickJS's autoinit): create a constructor's
   prototype, methods and accessors on first access. With 3., a materialized builtin is
   one object; without it, the code cells still come along. Est. a fresh VM at 300–600
   cells instead of 2,900 (with 3.). Cost: a check on property lookup misses of
   builtin objects (or a lazy property kind), materialization order is driven by the
   program, so deterministic, but heap layouts then depend on which builtins a script
   touched (fine for snapshots, which pin the build). Moderate change in object/; most
   of the visible "6000 cells" headline, little real memory without 3.
5. **Shared native function objects for internal helpers**: the 365 internal helpers
   are never seen by user code; they could be called by template (an intrinsic id →
   template) without a function object each: −365 objects, −16 KB. Small.

Presentation fixes (this issue): the REPL writes containers by default and reports own
data vs the base; the docs Heap tab shows the program's own cells separately.

## Follow-ups

## Result
Answer and proposal above (Findings, Proposal). A fresh VM is ~5,800 cells and ~320 KB
because the builtins are JS: each of 874 functions is an object, a template, bytecode and
tables in every VM's heap (3,723 cells, 217 KB); QuickJS has 1,097 allocations / 77 KB
after JS_NewContext (lazy builtins; 2,085 / 125 KB fully touched). REPL dumps were raw
images (now containers: 16 KB for a small session). Dump size beyond that is mostly
layout: growing the atom table and the global object's properties moves them past the
base and shifts 250 KB of base cells. Presentation landed (D2340); substance ranked as a
proposal: growable base cells last with slack (16 → 1–3 KB dumps), atom table growth,
builtin code outside the arena (−178 KB and −2,850 cells per VM), lazy builtins,
helper functions without objects.
