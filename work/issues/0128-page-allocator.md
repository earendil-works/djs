# 0128: Page allocator in djs-wasm: in-place arena growth

status: in-progress
agent: heap
branch: issue/0128-page-allocator
depends-on: 0125, 0126 (no_std djs-wasm + dlmalloc global allocator)
guide: docs/guide/gc.md / arena.md (memory growth), docs/guide/host.md if visible
touches: crates/djs-wasm/src/alloc.rs (new or from 0126), crates/djs-core/src/heap/ (arena growth hooks only if needed), docs/decisions.md

## Goal
heap's 0125 finding: peak wasm memory is dominated by arena reallocs that dlmalloc can't
extend in place (old + new block coexist, e.g. 9.3 MB + 11.6 MB), which makes peaks a
chaotic function of allocation placement. A small page manager under dlmalloc: huge
blocks (>= 1 MiB, the arena) grow via memory.grow in place while last in memory, others
from a coalescing page free list. Expected: peaks ≈ arena + Rust heap, robust across
perturbations (strbuild ~25 → ~16 MiB?).

## Acceptance criteria
- [ ] Allocator built with the unsafe confined to djs-wasm and justified in a decision
      (djs-core stays safe); unit tests + a randomized allocator stress test.
- [ ] Peak memory measured with the 0125 perturbation method (24 runs, distributions) on
      all workloads and the pi bench, before/after; instructions not worse than +1%.
- [ ] Code size measured (size ratchet).

## Log

## Follow-ups

## Result
