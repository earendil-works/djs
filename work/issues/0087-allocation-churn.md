# 0087: Allocation churn and wasm memory (0081 proposal 5)

status: open
agent: -
branch: issue/0087-allocation-churn
depends-on: 0081, 0071
touches: crates/djs-core/src/heap/ (gc to-space reuse, the threshold), crates/djs-core/src/ (transient Rust Vecs on hot paths), crates/djs-wasm/ (allocator interaction)

## Goal
0081 F6: wasm linear memory never shrinks, and the Cheney GC allocates a fresh to-space
at every collection. After a run, wasm memory is 58 MiB for strbuild and 49 MiB for
higher-order (qjs native peaks: 15 MB each). A short `'a' + i` spends ~2,200 instructions
in malloc/free on temporary Rust Vecs.

## Acceptance criteria
- [ ] Reuse the to-space between collections (keep the semispace; no fresh allocation per
      GC), and make the GC threshold aware of wasm's 64 KiB pages and memory that can't be
      returned.
- [ ] Remove transient Rust Vec allocations from hot paths (string concatenation, number
      formatting, call setup …), found by profiling in wasm.
- [ ] Peak wasm memory and instruction counts before/after (0081 workloads); determinism
      checks (D1090/D1601, 0072) identical.
- [ ] Coordinate with heap's 0083 (shapes) through the coordinator.

## Log

## Follow-ups

## Result
