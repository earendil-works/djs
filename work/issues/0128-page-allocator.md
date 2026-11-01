# 0128: Page allocator in djs-wasm: in-place arena growth

status: done
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
- [x] Allocator built with the unsafe confined to djs-wasm and justified in a decision
      (djs-core stays safe); unit tests + a randomized allocator stress test.
- [x] Peak memory measured with the 0125 perturbation method (24 runs, distributions) on
      all workloads and the pi bench, before/after; instructions not worse than +1%.
- [x] Code size measured (size ratchet).

## Log
- Built on 0126's no_std djs-wasm: allocator.rs's internals were replaced, and `Base` is
  still the global allocator or the alloc-stats wrapper's inner allocator.
  - `pages::Pages<M: Memory>`: a free list of page runs (a fixed 64-entry array).
  - `pages::Sys`: dlmalloc's system allocator over those pages.
  - `pages::PageAlloc`: huge blocks (≥ 256 KiB) as page runs, everything else dlmalloc.
  - `wasm::Linear` / `wasm::Base`: the module's memory, with the pages after
    `__heap_base` free at first use.
  - The code is generic over the memory, so native tests run it over a fake memory.
- First version: huge blocks extend in place or move. Peaks halved on the growing
  workloads; json/json-replacer +0.4–0.6 MiB. A trace showed the cause: one of
  dlmalloc's first segments lands right after the young arena, blocking its first
  extension.
- Tried, all moving single workloads by MiB either way (D2053 lists them):
  - free gaps before huge blocks at the end;
  - dlmalloc granularity;
  - minimum dlmalloc segment sizes;
  - huge threshold 128 KiB–1 MiB;
  - a block moving down into the run before it;
  - huge blocks only at the end;
  - dlmalloc segment release.
  Kept the simplest: extend or move to the best fit, no release, no alloc_zeroed
  override. That got the code from +3.6 KB to +1.2 KB wasm (+588 B gzip, under the
  ratchet).
- Measured: 24-run distributions on all workloads, wmeasure, the pi bench, pi-mem (numbers
  in D2053; raw files in work/scratch/0128/pt24-*.txt, mem-*.txt).
- Checked:
  - js tests and native-vs-wasm with the debug build;
  - clippy for wasm32 release with alloc-stats and docs;
  - allocator unit and stress tests natively.
- Guide: gc.md ("Peak memory and the host allocator" replaces "Peak memory is chaotic"),
  arena.md (in-place growth). TESTING.md's memory note updated.

## Follow-ups
- json/json-replacer are +0.4–0.6 MiB against main: an early dlmalloc segment sits right
  after the young arena. A fix could make the base image's VM reserve its first
  threshold at creation, so the arena starts past its first growths. Measure it with
  mem-perturb.
- The size ratchet could be lowered once other changes land; this branch sits 457 B
  under it.

## Result
- **djs-wasm on wasm32:** the global allocator is a page manager with dlmalloc on top
  (D2053). Huge blocks (the arena) grow in place, by `memory.grow` at memory's end or
  into free pages after them; otherwise they move into the best-fitting free run. The
  unsafe code is confined to allocator.rs; djs-core is unchanged.
- **Peak memory** (mean of 24 perturbed runs, MiB): strbuild 38.6 → 19.5, arrays 28.0 →
  14.3, higher-order 28.9 → 15.5, foreach/forof 9.3 → 5.3, json-date 6.1 → 4.8, pi-like
  3.70 → 2.81, strings 11.97 → 11.26. pi bench starts unchanged (1.94–2.13).
  - Worse: json 6.2 → 6.8, json-replacer 4.43 → 4.83 (max 8.0 → 5.6), props/jsonh/
    json-text +0.1.
- **Instructions:** within ±0.8%, strbuild −1.8%.
- **Size:** +1.2 KB wasm, +588 B gzip, under the ratchet.
- **Known gap:** the json regressions; see follow-ups.
