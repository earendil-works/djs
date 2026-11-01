# 0087: Allocation churn and wasm memory (0081 proposal 5)

status: done
agent: num
branch: issue/0087-allocation-churn
depends-on: 0081, 0071
touches: crates/djs-core/src/heap/ (gc to-space reuse, the threshold), crates/djs-core/src/ (transient Rust Vecs on hot paths), crates/djs-wasm/ (allocator interaction)

## Goal
0081 F6: wasm linear memory never shrinks, and the Cheney GC allocates a fresh to-space
at every collection. After a run, wasm memory is 58 MiB for strbuild and 49 MiB for
higher-order (qjs native peaks: 15 MB each). A short `'a' + i` spends ~2,200 instructions
in malloc/free on temporary Rust Vecs.

## Acceptance criteria
- [x] Reuse the to-space between collections (keep the semispace; no fresh allocation per
      GC), and make the GC threshold aware of wasm's 64 KiB pages and memory that can't be
      returned.
- [x] Remove transient Rust Vec allocations from hot paths (string concatenation, number
      formatting, call setup …), found by profiling in wasm.
- [x] Peak wasm memory and instruction counts before/after (0081 workloads); determinism
      checks (D1090/D1601, 0072) identical.
- [x] Coordinate with heap's 0083 (shapes) through the coordinator.

## Log
- Plan (to-space reuse, page-rounded thresholds, arena growth to the threshold, concat
  without Vecs) sent to the coordinator and cleared with heap before touching gc/arena.
- Malloc callers (work/scratch/0087/mprof.py, a prof.py variant): strbuild's malloc was
  `concat` → `collect_leaves` (Vec + vec! stack, ~28% of native time); fixed (D1871).
  fromCharCode's per-call Vec fixed with a stack buffer. Elsewhere malloc is ≤ 2% (join,
  from_codes for big lists).
- Built four GC variants and measured wasm memory: to-space reuse (spare) raised memory;
  growth capped at the threshold without reuse lowered it; shrinking the to-space always
  vs only when > 2x threshold made no difference. Kept: target-capped growth, 1.25x past
  the target, page-rounded thresholds, conditional shrink (D1870).
- Native peak-live-heap tool (work/scratch/0087/peak, tracking allocator) and pi bench with
  instance memory (work/scratch/0087/pi-mem.bench.ts).
- Determinism: e2e (all modes, image equality), slices.rs incl. the ignored all-e2e sweep,
  native-vs-wasm.sh, difftest --determinism on builtins/codemode: all identical.

## Follow-ups
- djs-wasm still starts at ~3.9 MiB for `return 1` in pi (quickjs-wasi 1.6): likely the
  1 MiB Rust stack, the data segment and image copies; worth a look (stack size vs the
  compile-time nesting limits of deep_search.py).
- Native RSS rose for strbuild/forof (+5–13%) while wasm fell; if native embedders matter,
  a native-only growth policy could be considered.

## Result
wasm memory after a run: strbuild 57.8 → 35.5 MiB, higher-order 49.4 → 38.8, json 21.1 →
14.3, jsonh 21.8 → 13.9, pi tool-heavy 5.3 → 4.4 (7.1 → 4.9 without the prelude image);
strbuild −34% native / −10% wasm instructions. Arena growth stops at the GC threshold and
thresholds are whole 64 KiB pages (D1870; GC points moved, still deterministic); short
concatenation and fromCharCode allocate no Rust memory (D1871). To-space reuse was measured
and rejected (it raised wasm memory).
