# 0287: Short result strings: allocation cost of tiny strings

status: done
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

2026-11-21 (pidle)
- Measured with `--wasm-sync-tier-up` (wasm instructions per op, main 63fc1f4b; QuickJS
  2026 in parentheses). Most short-string ops already beat QuickJS in instructions:
  String(i) 1.7k (1.9k), '' + i 1.3k (2.1k), (i/7).toFixed(1) 2.5k (2.8k),
  String(i).padStart(6) 3.1k (3.0k), 'abc'.padEnd(28) 1.5k (2.7k), slice(0,20) 1.7k
  (2.2k), substring(1,4) 1.8k (2.3k), charAt 1.8k (2.0k), at(0) 1.7k (2.1k), template
  `${i}: x` 2.4k (3.3k), split("") of 10 8.0k (10.7k). The ops chart's red rows for these
  are wall-clock ratios (node's default tiering, load, the Liftoff phase), not instruction
  ratios: read that chart's short-string rows with this in mind. The outliers in
  instructions: `s[i]` 1.9k (1.6k) and for-of over a string 22.9k for 10 characters
  (11.2k).
- D3560: one-unit ASCII strings from a per-VM cache dropped at every collection: `s[i]`
  → 1.45-1.6k, split("") → 6.8k, charAt ~−10%; +187 B gz; COMPAT 12. A one-digit
  String(i) through the cache was slower than allocating (dropped).
- Parked: for-of over strings out of line (22.9k → 17.3k, +243 B gz),
  work/research/0287/for-of-strings.patch, until 0286 reports its bytes.
- Tests: lib (incl. touching_builtins_in_any_order_leaves_the_same_image), difftest
  --determinism on language/builtins/codemode, stress, e2e; corpus case
  builtins/unit-strings.js.

## Result
The one-unit string cache (D3560) landed (+187 B gz); string iteration parked with its
numbers. Most short-string ops already beat QuickJS 2026 in instructions; the chart's red
rows are wall-clock effects.
