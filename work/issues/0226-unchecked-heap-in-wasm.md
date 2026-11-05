# 0226: Ship the wasm modules with unchecked heap access (unsafe-heap), keep checks elsewhere

status: in-progress
agent: heapx
branch: issue/0226-unchecked-heap-in-wasm
depends-on: 0225
guide: pages on isolation, safety and limits (goals.md Isolation, limits.md, arena.md, the wasm boundary), status sizes
touches: scripts/build-wasm.sh, scripts/wasm-build-env.sh, crates/djs-wasm features, crates/djs-core/src/heap/layout/access.rs (only if needed), scripts/gate.py (a leg), tests/scores/size.tsv, docs/decisions.md, guide pages

## Goal (owner: do it)
D2520 kept bounds-checked arena access in the shipped module because unchecked access is
sound only for images the engine produced. That condition now always holds: heap images never
leave the process (D2560), and persistence is plain-JavaScript dumps that are compiled and run
like any script. In wasm, the instance's linear memory already confines any engine bug to the
instance; the checks only turn in-instance corruption into a clean trap.

Decision (owner, 2026-11-06): the **shipped and slim wasm modules** build with `unsafe-heap`.
Checks stay in:
- native builds (no wasm sandbox: unchecked access would be memory unsafety in the host
  process; djs-cli, native hosts, tests);
- the dump wasm module (the most complex heap walking);
- every test, difftest, fuzz, e2e and gate build, except the legs that test the shipped
  modules themselves.

Work:
1. Build: the shipped/slim builds (scripts/build-wasm.sh default and --slim) enable
   `unsafe-heap`; the dump and docs modules don't. DJS_STABLE_ONLY and the existing flags keep
   working.
2. Gate: the legs that run the shipped module (size budget, js tests, pi tests in next/slim,
   native-vs-wasm, wasm workloads) now run the unchecked module, so it is exercised; keep at least
   one stress leg (gc-stress / suspend-every) on the unchecked wasm module if cheap, so a bug that
   only corrupts silently is still caught by output differences.
3. Measure: size (expect ~-7.7 KB gzip shipped, slim similar) and wasm instructions per workload
   (the 12 of 0223/0225), especially call-heavy and tight loops, where D2520 saw +2-3%
   (run_hot register allocation). If any workload regresses by more than ~3%, find why and fix or
   report before merging.
4. Lower the size ceilings (`--update`), re-record status sizes.
5. Record the decision (amends D2520), update the guide's isolation/safety wording (goals.md
   Isolation says "every heap access is bounds checked": now true for native builds and checked
   modules; in the shipped wasm modules, the wasm sandbox is the boundary and engine bugs may
   corrupt state inside the instance instead of trapping), limits/arena/wasm-boundary pages, and
   AGENTS.md's "Heap access is bounds checked" line.

## Log

## Follow-ups

## Result
