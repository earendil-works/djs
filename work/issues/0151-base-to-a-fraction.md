# 0151: Shrink the base heap to a fraction, without removing anything

status: in-progress
agent: bloat
branch: issue/0151-base-fraction (one branch per landing: issue/0151-<step>)
depends-on: 0143, 0145, 0147, 0148, 0150
guide: docs/guide/builtins.md, bytecode.md, arena.md (as mechanisms change)
touches: crates/djs-core/src/builtins/ (the TS builtins, natives), crates/djs-bgen/ (backends, a new direct-native binding), crates/djs-core/src/compiler/ + interp/ (bytecode encoding, if changed), crates/djs-core/src/heap/ (template/function-object layout), crates/djs-prelude/ (base build), tests/builtins/registry.tsv (registry-update.sh), tests/scores/size.tsv

## Owner
"With moving some code to Rust and coming up with more compact bytecode we should be able to
drop to a fraction of what we have today. I mean it. Even without removing anything. In
particular the typed arrays, which should be able to share most of the code."

## Today (base-families example, main)
325 KB / 5,852 cells per VM (322 KB after 0150). 874 builtin functions: function objects
38.9 KB, templates 48.9 KB, bytecode 99.6 KB, constant pools 13.5 KB, capture descriptors
14.0 KB, property storage 37.1 KB, strings 28.9 KB, atom/stack/index arrays 35.0 KB.
Typed arrays alone: 137 functions, 856 cells, 44 KB (16 KB bytecode).

## Goal
A base heap that is a **fraction** of today's (aim: under 100 KB raw per VM), with every
builtin kept, spec behaviour unchanged (test262 active, difftest, pi suite, determinism,
order-independence D2067, prefix barrier all green), and no workload slower than +1%.
The **primary metric is the raw base image (per-VM memory)**; module gzip is secondary:
Rust growth that buys large per-VM savings is an acceptable recorded trade against the
size ceiling (state the exchange rate in each decision).

## Levers (measure each, land the ones that pay, in separate merges)
1. **Share code in families**: typed arrays (one implementation of %TypedArray%.prototype
   methods parameterized by element kind; per-constructor objects as thin data; why 137
   functions and ~40 internal helpers?), DataView get/set (one generic accessor per
   width), iterator helpers (one state-machine shape), Promise combinators, etc.
2. **Direct native binding** for thin builtins (coerce args, call one intrinsic): a
   builtin function object that points straight at a Rust entry, with no template,
   bytecode, constant pool or capture cells (QuickJS-style C function), via bgen. Classify
   all 874 builtins (thin / calls back into JS / complex), estimate per-VM bytes vs Rust
   bytes, prototype on Math/String, then roll out.
3. **Compact function objects and templates** for builtins: what of the ~45 B object +
   ~56 B template is needed for a builtin (no line tables, fixed flags)? A shared or
   smaller template form; name/length from the registry/descriptor instead of per-object
   properties where unobservable.
4. **Denser bytecode for privileged code**: short forms, narrower operands, fewer
   instructions per spec step (0127 dropped density because the codec hid it in module
   size; per-VM memory is uncompressed, so re-measure on the raw base). Interpreter cost
   must stay within budget.
5. **Property storage and strings**: builtin property tables (37 KB) and name strings
   (29 KB, deduped by 0150): a compact frozen representation for builtin objects
   (frozen mode exists, D2304; the prefix never changes, D2064)?
6. **Capture descriptors and boxes** (14 + 4 KB): avoid closures in builtin code where
   bgen can pass state otherwise.

## Acceptance criteria
- [ ] A plan with the measured potential of each lever (first deliverable, in the issue log,
      reported to the coordinator), then landings in order of payoff.
- [ ] Base raw size and cells after each landing; module gzip, vm_new, wasm instructions on
      all workloads, snapshot sizes.
- [ ] Registry updated where builtin functions change (ids kept for kept builtins).
- [ ] Guide pages updated.

## Log
### Plan (measured on main 2e784f25, base 325,008 B / 5,852 cells)
Tools: work/scratch/0151 (fnsizes, ophist, classify.py; a copy of the tree that keeps
template names to join cells to sources).

**Where the bytes are.** The 874 builtin functions are 237 KB of the 325 (73%); the tail
(atom table entries 15.6 KB + intern index 16.4 KB, stack, global table) ~40 KB; prototype
and namespace property tables ~20 KB; strings 29 KB. Per function, 139 KB is fixed
overhead (object 32 B + 8 per capture, name/length PropValues 32 B, template 56 B, cell
headers of code/pool/capture descriptor) and 96 KB payload (86 KB bytecode).

**Classification (874):**
| | fns | bytes | code |
| --- | ---: | ---: | ---: |
| visible (own registry path) | 539 | 143 KB | 43 KB |
| installed intrinsics (%intrinsic:) | 112 | 32 KB | 15 KB |
| internal helpers (capture-only) | 223 | 62 KB | 28 KB |
| leaf (never run user code) | 223 | 39 KB | 7.7 KB |
| reentrant | 651 | 198 KB | 78 KB |
| thin: one `return %I(args)` / `return F(args)`, args plain or coerced | 214 | 37 KB | 4.5 KB |
| complex leaf | 134 | 25 KB | 6 KB |
| complex reentrant | 526 | 175 KB | 75 KB |

Typed arrays are already one implementation: %TypedArray%.prototype has one function per
method; the 12 constructors are 28-byte calls of TypedArrayConstruct(kind). The family's
44 KB is 137 functions × ~180 B of per-function overhead plus 16 KB bytecode, so sharing
code there saves ~2 KB; the per-function costs are the lever.

**Levers, measured potential (per VM):**
1. L6, no closures in builtin code: calls between prelude functions go through IIFE boxes
   (257 BoxCells 4.1 KB), capture descriptors (652, 14.0 KB) and capture slots in the
   function objects (11.0 KB); 1,580 calls are `get_cap; undefined; args; call`. A
   privileged `call_builtin idx argc` / `get_builtin idx` over one prelude function table
   (2 KB): **−27 KB cells, −5 KB code.**
2. L4, denser bytecode: 86 KB in 31,100 instructions; operands are 2–4 bytes wide but
   7,749/7,749 locals, all args/caps/argcs, 3,470/3,552 intrinsic ids, 1,076/1,076
   consts and 2,074/2,291 jumps fit 8 bits. Narrow operands with wide forms −30 KB
   (0127 measured 99 → 70 KB), implied intrinsic arity −3.5 KB, 1-byte forms for
   loc/arg 0–3 ~−8 KB, 762 dead `return_undef` after return/throw: **−40 KB.**
3. L3, compact functions and templates: the code cells' headers and padding 13.6 KB, 394
   pool cells 13.5 KB for 901 entries (one deduped pool: −8 KB), privileged templates
   carry unused NAME/CHILDREN/IC/CAPTURE_DESC (56 → ~32 B: −20 KB), name/length of 489
   functions in 32-byte PropValues (from the template until first changed: −15.6 KB):
   **−50 KB**, the last two the riskiest (call path, property layer).
4. L2, direct native binding for the 214 thin builtins (a function object naming an
   existing intrinsic/builtin with an argument map and constants, QuickJS's magic; no
   new Rust per builtin): −25 KB today, **−11 KB after L3/L6**. Covers DataView get/set,
   the TypedArray constructors, every/some (lever 1).
5. L1 rest (shared families with magic): **−3..−5 KB**.
6. L5, frozen dictionaries (81 PropTables 14 KB + 15 hash indexes 6 KB): **−5..−8 KB.**
7. Optional trade: the 134 complex leaf builtins to Rust through bgen (no state machine
   needed): **−16 KB per VM for ~+12 KB gzip** (estimate, to be measured).

**Projection:** 325 → ~295 (L6) → ~255 (L4) → ~205 (L3) → ~190 (L2/L1) → ~182 (L5) →
~166 (L7) KB, before 0150 (strings, atom table: the 32 KB atom tail is its area).
Under 100 KB with every builtin kept needs the 75 KB of reentrant bytecode out of the
VM. Natives that can suspend cost ~2 KB wasm each (D2020: 26 KB raw for 13), so 526
of them are not a trade. Running builtin bytecode from module memory instead of the
arena drops the raw base by ~90 KB but the instance's memory only by the coded copy
(data segments are per instance, D2252), so I do not count it as a saving.

**Order:** L6 → L4 → L3 (code/pool cells, then templates, then name/length) → L2+L1 →
L5 → L7 if accepted. Each its own branch and decision; measurements per landing.
### L6: prelude functions in a realm table (branch issue/0151-closures, D2440)
- Compiler (privileged): top-level prelude function declarations referred to by other
  functions get `Storage::Builtin`; `get_builtin`/`put_builtin` over
  `Intrinsic::PreludeFunctions` (257 slots). No Boxes, capture descriptors or capture
  slots remain in the base. Ops in step, fast_step, run_hot.
- Registry: `%prelude:Name%` (names from `compiler::prelude_function_names`); 223 `~N`
  entries tombstoned (they were D2371's fragile paths), 224 added.
- Base (after 0150) 322,720 → 295,952 B, 5,752 → 4,848 cells; module gzip 459,027 →
  453,853 (ceiling lowered to 454,863); vm_new 3.77 → 3.42 µs; wasm instructions ±0.8% (n=5/9); session
  images unaffected (prefix not stored).
- Guide: builtins.md (prelude paragraph, registry paths). ARCH.md's "closures capture
  boxed cells" stays true for user code; proposed note: privileged prelude functions
  refer to each other through the realm's table (D2440).
- 0150 landed: atom renumbering (1,460 holes, 11.7 KB/VM) joins L3/L4 (the remap touches
  templates, bytecode immediates, pools, shapes).

## Follow-ups

## Result
