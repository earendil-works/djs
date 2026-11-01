# 0123: Why is djs so much bigger than QuickJS? (research)

status: done
agent: bloat
branch: issue/0123-why-bigger
depends-on: 0122 (its attribution; runs in parallel)
guide: - (a guide page may follow)
touches: docs/size-vs-quickjs.md (new report), scripts/size/ (analysis scripts), work/scratch/0123/; no engine changes

## Goal
Owner's question: what are the **fundamental** reasons the djs wasm module is ~1.85x
quickjs-wasi's gzip size (530 vs 287 KB; raw 1,480 KB)? Not a list of cuts (perf does
that in 0122) but an explanation: where the extra bytes come from structurally, compared
feature area by feature area with QuickJS.

## Questions to answer
- Like-for-like breakdown: djs vs quickjs-wasi by area (parser/compiler, interpreter,
  builtins by family, regexp, Unicode tables, GC/heap, JSON, number formatting, BigInt,
  TypeScript stripping, determinism/snapshot machinery, the host ABI), in raw and gzip,
  with the method stated (twiggy/name sections for djs; QuickJS's wasm with names, or a
  native build's symbol sizes per source file as a proxy, from the read-only
  ~/Development/dquickts sources and existing binaries — never build or modify it there;
  copying sources into work/scratch/0123 to build them there is fine).
- Things djs ships that QuickJS doesn't: the base image (QuickJS builds its realm at
  runtime from C tables), bgen-generated Rust fast paths alongside JS builtins, TS
  support, determinism, two collectors, etc. What each costs.
- Language/toolchain effects: Rust monomorphization and generics, inlining at
  opt-level 3 vs QuickJS's -Os/-O2 build flags, bounds checks and panic paths, core::fmt,
  wasm code density of Rust vs C output (e.g. compare the same small function), LLVM
  versions/flags. Quantify with experiments (e.g. djs at opt-level s/z, panic=abort with
  immediate-abort, no fmt) rather than guesses.
- Design choices: JS-implemented builtins in the image vs C natives, bytecode format
  density, shape/property representation, tables duplicated between compiler and runtime.
- What fraction is fundamental (would remain in any Rust engine with djs's goals) vs
  incidental (fixable), with rough estimates.

## Deliverable
docs/size-vs-quickjs.md: a clear explanation for the owner with the numbers, a ranked
list of root causes, and what would have to change to reach ~1.5x and ~1.0x. Coordinate
with perf (0122) so the two don't duplicate measurements; share scripts.

## Log
- Coordinated with perf (0122): reused its measurements (opt-level z, prelude from source,
  xz of the image, natives' cost, image ordering) and its scripts/size/{sections,
  image-kinds,families}.py instead of repeating them; my scripts in scripts/size/ use
  other names (areas.py, variant.sh, bytecode-density.py, density/, qjs/, patches/).
- QuickJS with names: dquickts's sources copied to scratch and built with zig cc
  (wasm32-wasi, -Os, -g; the core without Temporal/Intl/libc). Its gzip is within 4% of
  quickjs-wasi's, so it serves as the area-by-area reference; quickjs-wasi stays the
  budget reference. Function-to-file attribution from its DWARF line table.
- One area taxonomy for both engines (scripts/size/areas.py djs|c); gzip per area by
  compressing each area's bodies and scaling to the section total.
- Experiments (variant builds in scratch, never merged): opt-level 3 (+78 KB gz), no
  arena bounds checks (−11.2), nightly immediate-abort panics with build-std (−17.5 vs
  a build-std baseline), a link map for the data, a DWARF line-table build.
- Density: six kernels in C and safe Rust (1.64x raw); bytecode for the same source vs
  QuickJS (1.36–1.97x); base-image strings classified (965 internal function names, 529
  local names out of 2,360).
- No guide pages affected (docs/size-vs-quickjs.md is a design doc, not a guide page).

## Follow-ups
- Image strings: internal prelude function names and local names are interned as atoms
  and kept by the atom table (~52 KB raw of 80 KB string cells, ~15 KB gz plus their
  share of the atom index). Probably the cheapest image cut; for 0122 or its own issue.
- Unicode tables: a run-length encoding like libunicode's (107 → ~45 KB raw, ~−13 KB gz).
- Bytecode density for privileged code (short forms, narrower operands): ~−9 KB gz.
- A guide page could summarize docs/size-vs-quickjs.md (coordinator's call).

## Result
docs/size-vs-quickjs.md. The 231 KB gz gap to QuickJS (dquickts, built with names; 243
to quickjs-wasi): Rust code density on shared features +90 (~1.7x gzip per area; kernels
1.64x; inlined Vec growth, Result plumbing, bounds checks 11, panics/fmt 17), builtins
as bytecode in the shipped image +81 net (image 105; two thirds of its strings are
internal names; bytecode 1.4–2x QuickJS's), speed/embedding features +48 (`@native` and
fast paths 21, JSON 9, host ABI 9, suspension/wire 6, TS), Unicode tables +15, QuickJS-
only code −9. Not causes: GC, opt-level, LLVM, libm, monomorphization. About 45%
fundamental, 55% incidental; 1.5x is reachable within the design, 1.0x needs a different
engine. Scripts: scripts/size/{areas.py, variant.sh, bytecode-density.py, density/,
qjs/, patches/}.
