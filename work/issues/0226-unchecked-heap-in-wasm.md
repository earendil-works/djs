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
- Build: scripts/build-wasm.sh passes `--features unsafe-heap` for the default (and
  --keep-names) and --slim builds; --dump/--docs unchanged. Same for gate.py build:size,
  build:size-slim and their release clippy legs (main, slim), djs-bench size/wasm groups,
  perf/setup.sh, size/variant.sh, and the doc'd commands in vec-sites.py,
  wasm-dwarf-lines.py, size/README.md. A feature, not wasm-build-env.sh: cargo features
  can't come from the environment, and --dump/--docs use that env too.
- First measurement (wmeasure n=5, checked vs unchecked shipped): −5..−27% on most, but
  tight +4.3%, loop +4.4%, calls +3.5%, call1 +2.8%. Cause: interp/run/hot.rs had its own
  `ld`/`st`/`ld32`/`window` with slice checks, so the tier-0 loop's slot accesses stayed
  checked under unsafe-heap and only run_hot's register allocation changed (V8 TurboFan
  dispatch head identical; the inlined heap helpers differ). Routed them through
  access.rs (`heap::layout::{arena_read, arena_write}`); outside `touches:` but minimal
  (hot.rs: 4 helpers + an import; layout.rs: a re-export). `word`, `fill`,
  `write_frame`, `write_values` keep their slice checks (Option / block checks).
- Final, against main 2bf9a5fe (wmeasure n=5, 21 workloads): shipped −4.5% (tight) to
  −30% (indexof); tight −4.5, loop −4.7, calls −9.6, call1 −7.7, pi-like −8.1, json
  −10.4, arrays −17.1. The checked build gains too (tight −4.8, calls −6.3). Slim (n=3):
  tight −4.9, loop −4.4, calls −10.0, pi-like −8.0. Size gzip: shipped 422,453 →
  414,390, slim 357,078 → 348,676. Ceilings lowered (budget.py --update).
- Stress leg on the unchecked wasm: native-vs-wasm on the size module isn't cheap (its
  images and build hash differ from the default-feature native driver's), so not added.
  Covered by the existing unsafe-heap difftest (release, suspend 97 + gc-stress) and
  pi's tests (next mode, slim) which now run the unchecked modules. Both pi runs pass
  locally (221/226, 213/226, no new failures).
- status.py bounds now builds the checked module and compares it to the shipped one
  (same keys, same sign: what the checks would cost).
- instructions.rs (D3084, dev profile, checked): 135.7M, unchanged.
- Docs: D3100 (amends D2520) + index; AGENTS.md bounds line; ARCH.md heap paragraph
  (minimal, proposed edit of the design text); TESTING.md gate list; access.rs and
  djs-wasm Cargo.toml comments. Guide: goals.md Isolation, arena.md (in short, point 3,
  the trade-off, where to look), limits.md, snapshots.md, host.md, testing.md.

## Follow-ups

## Result
