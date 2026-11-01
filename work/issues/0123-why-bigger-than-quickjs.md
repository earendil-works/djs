# 0123: Why is djs so much bigger than QuickJS? (research)

status: in-progress
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

## Follow-ups

## Result
