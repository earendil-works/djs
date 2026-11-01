# 0094: Baseline wasm memory of a VM (3.9 MiB vs quickjs-wasi's 1.6 MiB)

status: open
agent: -
branch: issue/0094-baseline-memory
depends-on: 0087
touches: crates/djs-wasm/ (stack size, memory setup), crates/djs-core/src/heap/ (initial arena sizing), crates/djs-prelude/, crates/djs-core/src/compiler/ (only if the nesting limits must change), js/src/sandbox.ts, docs/host.md

## Goal
After 0087, a pi `return 1` run still peaks at ~3.9 MiB of wasm memory against 1.6 MiB
for quickjs-wasi. Find out exactly where the baseline goes and cut it. Candidates: the
1 MiB default Rust stack (vs the compiler's nesting limits; `deep_search.py`), the base
image in the data segment and its copy in the arena (both live?), the initial arena
reserve, dlmalloc's growth granularity, the Sandbox's staging buffers, and node's own
minimum.

## Acceptance criteria
- [ ] An accounting of the 3.9 MiB (data segment, stack, heap arena, allocator slack,
      buffers), measured.
- [ ] Reductions with numbers: for example a smaller stack if the compiler's recursion
      fits (or a deeper-but-iterative parse where needed), not keeping two copies of the
      base image live, initial arena sized to the base image plus a page.
- [ ] No behavior change (deep nesting tests still give the same SyntaxError limits or
      documented new ones), the pi bench and memory numbers before/after.

## Log

## Follow-ups

## Result
