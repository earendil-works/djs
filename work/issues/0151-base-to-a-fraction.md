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

## Follow-ups

## Result
