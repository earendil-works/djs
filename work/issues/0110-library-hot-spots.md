# 0110: pi-like's remaining hot spots: string search, allocation, flatten, dense element access in natives

status: open
agent: -
branch: issue/0110-library-hot-spots
depends-on: 0107
guide: - (tell the coordinator if the builtins or interpreter chapters need updates)
touches: crates/djs-core/src/heap/string.rs (search, flatten), crates/djs-core/src/heap/arena.rs (alloc_init) (heap is idle pending the 0101 decision; coordinate through the coordinator), crates/djs-bgen/src/emit_rust.rs + builtins/gen (fewer states per element, a fused dense HasProperty+Get), crates/djs-core/src/interp/run/

## Goal
0107's findings: pi-like hands under 0.1% of its run to the general interpreter; its time
is in the builtins library: string `index_of`, arena `alloc_init`, JSON, GC, flatten; and in
native builtins each forEach element still costs ~300 native instructions in the state
arms, with HasProperty and Get reading the same dense element twice.

## Acceptance criteria
- [ ] A wasm profile of pi-like by function after 0107, and fixes for what dominates, for
      example: a SWAR/word-at-a-time `index_of` for one- and two-byte strings (wasm has no
      SIMD by default, D016/0077), a cheaper `alloc_init` fast path, fewer flattens
      (str_view_flat where read-only, D1610), a fused dense HasProperty+Get helper and
      fewer states per element in bgen's generated loops.
- [ ] wasm instruction counts (primary) before/after per change; images, fuel, stress and
      determinism checks unchanged; the size per change.

## Log

## Follow-ups

## Result
