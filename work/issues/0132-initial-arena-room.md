# 0132: Initial arena room for VMs from the base image (json peak regression)

status: in-progress
agent: heap
branch: issue/0132-initial-arena-room
depends-on: 0128
guide: docs/guide/gc.md (peak memory section) if behavior changes
touches: crates/djs-core/src/heap/ (initial capacity), crates/djs-core/src/vm/ (vm_new paths), crates/djs-wasm/src/allocator.rs

## Goal
0128 follow-up: json 6.2 → 6.8 MiB and json-replacer 4.43 → 4.83 MiB, because one of
dlmalloc's first segments lands right after the young arena, so the arena's first growth
can't extend in place. Give a VM made from the base image room up to its first
collection threshold at creation (or place the first dlmalloc segment before the arena),
measured with scripts/perf/mem-perturb.sh on all workloads and pi warm/cold.

## Acceptance criteria
- [ ] json and json-replacer back to main-before-0128 or better; no workload worse
      beyond noise; pi warm/cold memory and vm_new time not worse.
- [ ] Decision recorded.

## Log

## Follow-ups

## Result
