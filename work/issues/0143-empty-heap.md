# 0143: Why is a fresh heap 300 KB? Towards a nearly empty heap (lazy / out-of-heap builtins)

status: done
agent: bloat
branch: issue/0143-empty-heap
depends-on: 0135, 0136, 0138, 0139
guide: - (later)
touches: docs/empty-heap.md (report), scripts/size/ or scripts/heap/ (analysis scripts), work/scratch/0143/ (prototypes); no engine changes on main

## Owner
"Why is our heap out of the box already 300K large? That is silly. Can we not create most
of those objects lazily on first use? Investigate that first. Where is all that going?"
And the idea behind it: object layouts (an array, a plain object) won't change between
djs versions; what makes snapshots version-bound is that session objects reference
builtins, prototypes, shapes and globals inside the same heap. "Maybe the solution is to
prevent all these objects from existing in the heap to begin with", or to represent
references to builtins differently (stable ids) so that part can change.

## Known (0135, 0138)
Base realm 5,768 cells / ~314–322 KB: 3,723 cells (217 KB) are the 874 builtin functions
(object + template + bytecode + tables; 509 reachable from globalThis, 365 internal prelude
helpers), 904 strings, ~750 property-storage cells, 257 closure boxes, atom table, stacks.
QuickJS: 1,097 allocations / 77 KB after JS_NewContext, 183 objects (lazy); 2,085 /
125 KB fully touched. 0138's shared read-only segment study: lazy-copy variant 6–7 weeks,
≤0.9% interpreter cost; recommended not now (pi memory gain small).

## Questions
1. **Exact accounting** of the base image, by builtin family (Array, String, Promise,
   RegExp, Intl-ish, TypedArrays, Iterator helpers, Temporal?, internal helpers...):
   cells and bytes each, split into function objects / templates / bytecode / constant
   pools / tables / property storage / strings. Which families does a typical pi script
   touch (use the 0130 feature data and run the harvested corpus counting which builtins
   are materialized/called)?
2. **What a minimal eager realm is**: the cells that must exist at vm_new (global object,
   a few prototypes, the atom table...) vs everything that could be created on first
   use, and the resulting fresh-heap size and cell count (target: QuickJS-like, < 100 KB
   / < 1,000 cells, ideally far less).
3. **Lazy materialization designs**, with their costs: QuickJS-style autoinit property
   kinds (a property whose value is created on first access), lazy prototypes/
   constructors per family, lazy function templates (bytecode compiled or copied on first
   call: 0138's lazy copy), determinism (materialization order is script-driven: does it
   matter for images, canonical_hash, the gate?), interaction with pi's lockdown
   (freezing ~700 builtins eagerly would materialize them all: how does pi's prelude
   need to change, e.g. freeze lazily/on materialization?).
4. **Out-of-heap builtins with stable references** (the owner's idea): builtin
   functions, their templates/bytecode, and immutable base objects living outside the
   arena (module data or a per-instance segment), referenced from the heap by stable
   ids (builtin path / spec name) rather than offsets, so snapshots contain only
   session-made cells and survive builtin changes; what about prototypes and globals that
   sessions mutate (copy-on-write into the heap?), shapes (the root shape and the shape
   tree from base objects), atoms (stable atom ids for builtin names?). Reuse 0138's
   measurements; re-evaluate its recommendation under the owner's goals (snapshot size +
   portability + memory, not just pi memory).
5. A ranked plan: what to do first for the biggest reduction with the least risk, with
   estimated fresh-heap size, vm_new time, module size, interpreter cost, snapshot size,
   and engineering time per step.

## Deliverable
docs/empty-heap.md, with numbers and a recommended plan. Coordinate with the portable agent
(0142, snapshots that survive updates), which considers the same stable-reference idea
from the snapshot side: share findings via the coordinator.

## Log
- Accounting tool: crates/djs-prelude/examples/base-families.rs (family by reachability
  and installed-intrinsic name, part by role; `--touched DIR` for the runtime data). Outside
  `touches:` (djs-prelude examples, next to image-size.rs), an example only.
- Runtime data: scripts/size/patches/touch.py instruments a scratch copy (prop_read and
  enter_template marks, difftest dumps offsets); the 300 harvested cases ran one process
  each (295 pass, 5 known ratchet failures), plus the corpus prelude alone.
- pi: reference.ts pi-prelude with two tools, diffed against the base with `djs heap`.
- No engine change; nothing on main but the report, the example and the patch script.

## Follow-ups
- Prototype one lazy family (Array.prototype as a lazy holder) to measure materialization
  time and the flag test before committing to step 4.
- pi-private: engine-level lockdown needs pi's prelude to drop its lockdown walk (owner).
- Share section 7 of docs/empty-heap.md with the portable agent (0142).

## Result
docs/empty-heap.md. A fresh VM is 322 KB / 5,770 cells: 217 KB are the 874 builtin
functions (code 60%), ~48 KB atoms and names, 36 KB property storage. 300 harvested runs
touch 105 of the 874 functions together, 26 per run (median); object-by-object lazy
materialization would build ~17 KB per run. 127 KB of families are never touched. pi's
lockdown adds 156 KB / 2,600 cells (471 KB per session) and would defeat laziness unless
it moves into the engine. Plan: stable builtin registry (1 wk) → engine-level lockdown
(1–2 wk, pi −120 KB) → static builtin atoms (2 wk, −48 KB) → lazy realm with lazy code
copy (4–6 wk): fresh VM ~100 cells / ~10 KB, pi start ~30–60 KB. Stable ids as a runtime
representation: not recommended (3–4 months, second object model); take the registry and
use it for export-time portability (0141/0142).
