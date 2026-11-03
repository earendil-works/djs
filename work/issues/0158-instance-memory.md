# 0158: Instance memory: smaller reservation, arena at the end, smaller stack, base outside memory

status: in-progress
agent: heap
branch: issue/0158-instance-memory
depends-on: 0128, 0139, 0151 (L4 landed)
guide: docs/guide/arena.md, gc.md, host.md as affected
touches: crates/djs-wasm/ (allocator.rs, base decode/inject, stack size in the build), crates/djs-core/src/heap/ (initial reservation, capacity), scripts/build-wasm.sh / wasm-build-env.sh (stack size flag), js/src/snapshot.ts (base handling, if the compressed base leaves memory); bloat (0151 L3) works in heap/ templates and objects: coordinate via the coordinator

## Goal
docs/architecture-bloat-audit.md step 5: a fresh Sandbox is 19 pages (1.19 MiB, now 18
after L4): 4 stack, 3 static data, 11 arena (base + 409 KiB reserved and never touched,
because the reservation is 2x the base), 1 Rust heap. Target ~11 pages:
- reserve less up front, **with the page allocator keeping the arena at the end of
  memory** so it can still grow in place (the audit's prototype at 12 pages made growing
  heaps end 1–4 pages higher because the arena got boxed in);
- a 128 KiB wasm stack if the deep-recursion tests and D1941's parser depth limits still
  pass (or make the stack size a build option with 128 KiB for the shipped module);
- keep the compressed base out of wasm memory (the host holds it; D2252/D2060 paths).
Measure with mem-perturb on all workloads, pi warm/cold, an empty Sandbox, and
scripts/size/sandbox-memory.ts; no workload's peak worse beyond noise.

## Log
- Stack, measured on the debug wasm (the gate's nesting test), scratch probes in
  work/scratch/0158/ (nesting-probe.ts, runtime-probe.ts):
  - parser nesting, deepest level without a trap at 128 KiB vs the accepted limit at
    MAX_DEPTH 1000: parens 248/332, arrays 266/333, objects 248/332, calls 398/498, funcs
    164/199, blocks 723/995, trys 721/995, template 248/332, classes 81/110, methods
    101/124; arrows, ifs, fors, switches, whiles, patterns, regexp groups fit. Worst
    ratio 0.72 (trys, blocks): MAX_DEPTH ~650 leaves >= ~10% margin everywhere.
  - runtime recursion (JS calls, JSON both ways, join, proxy/bound/prototype chains,
    getters, a 300k-deep list through the GC, regexps, sort, async recursion, toString,
    flat): identical results at 256 and 128 KiB.

## Follow-ups

## Result
