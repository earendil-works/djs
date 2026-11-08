# 0249: djs-embed: report and shrink an instance's heap after reset

status: done
agent: coordinator
branch: issue/0249-embed-heap-size
depends-on: 0240
touches: crates/djs-embed (instance.rs, tests, CHANGES.md, version), docs/guide/rust-api.md
guide: docs/guide/rust-api.md

## Goal
Gondolin (embedding djs-embed at 9cc1291d, crate gondolin-hooks-djs) drops pooled
instances after a call that read more than 256 KiB of body, because `reset` keeps the heap
allocation a call grew it to and there is no way to see or shrink it. Add both.

## Acceptance criteria
- [x] `Instance::heap_size` (the arena's allocation), `Instance::heap_used` (the bytes in
      use), `Instance::shrink_heap(keep)` (gives back the allocation beyond `keep`, never
      below what is in use)
- [x] a test: a call grows the heap, reset keeps it, shrink gives it back, later calls work
      and grow it again
- [x] crate version 0.1.1 and a CHANGES.md that lists API changes for hosts that pin a rev
- [x] guide

## Log
- Measured `reset` on Gondolin's shim snapshot (240 KiB): 3.2 µs, the same as a plain
  memcpy of the image (3.3 µs) on this machine at load ~45: reset is the copy. Cheaper
  resets need copying less, which is 0250.

## Result
`heap_size`/`heap_used`/`shrink_heap` on `Instance` (additive; djs-embed 0.1.1). A pool
does `vm.reset()?; if vm.heap_size() > LIMIT { vm.shrink_heap(0)?; }` instead of dropping
the instance. CHANGES.md records API changes from now on (D3247).
