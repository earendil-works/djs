# 0287: Short result strings: allocation cost of tiny strings

status: in-progress
agent: pidle
branch: issue/0287-short-result-strings
depends-on: -
touches: crates/djs-core/src/heap/string.rs (short-string allocation paths only; pgc's 0285 changes hashing in the same file: coordinate), heap/arena.rs (small-cell allocation), the leaves that produce short strings (String(i), toFixed, padStart, slice/substring results, charAt/[i], template pieces)
guide: heap.md if the string story changes
decisions: D3560-D3569

## Goal
Several red ops-table rows allocate a short result string per call (String(i) 1.23,
toFixed 1.14, padStart 1.27, slice 1.15, template appends 1.09). Measure what a short
string allocation costs now (cell header, size class, zeroing, hash caching) and cut it:
e.g. a per-VM cache of one-character strings (charAt, s[i], single-char slices, split('')),
small-integer strings for String(i) of small ints, writing digits straight into the cell,
skipping work a fresh short string doesn't need. Determinism: any cache is image state or
rebuilt identically (collections, images, dumps must not depend on cache hits; check with
the determinism and every-stop tests).

## Acceptance criteria
- [ ] per-op instructions vs QuickJS 2026 before/after; workloads; memory unchanged (codemode-memory leg)
- [ ] size reported (~1 KB of the owner's allowance left, shared); gate green; decisions recorded
## Shared context (all of 0256-0260)
The owner asked to push both memory and speed as far as they go (2026-11-21). Background,
fresh profiles and the ranked idea list: work/scratch/perfreview/ of the primary checkout
(BRIEF.md, IDEAS.md; prof.ts and cat.py profile the codemode workloads in wasm). Read
IDEAS.md's section for your issue first; its numbers are estimates, not measurements.

Five issues run in parallel (0256 per-VM/per-byte costs, 0257 idle memory, 0258 GC,
0259 builtin calls, 0260 byte kernels). Stay in your `touches:`; announced overlaps are in
each issue's Notes.

Measuring (D016: wasm32 under node is the yardstick):
- run time and memory per workload: build the shipped module in your worktree
  (`scripts/build-wasm.sh`, or `--keep-names` for profiles) and run
  `DJS_WASM=$PWD/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm node js/bench/engines/run.ts --engines djs --reps 25`
  before and after, on main's build and yours, at load below ~4 (`uptime`; say the load).
  Memory claims: scripts/perf/mem-perturb.sh (D2048).
- instructions: scripts/perf/wmeasure.py / wmicro.py (TESTING.md "Instruction ratios").
- Report per workload: ms and MiB, main vs branch, and the module's gzip bytes.

Size: the shipped module is ~1.3 KB gzip under the D020 budget (ceilings, D3266). Report
the bytes of every change; find offsets where you can. If a measured win needs more room
than that, stop and tell the coordinator with the numbers: do not raise ceilings yourself.

Never weaken determinism (collection points a function of the program, D1710), suspension
anywhere, images, or the unsafe rules (AGENTS.md).

## Log

## Result
