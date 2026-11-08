# 0250: Cheaper instance resets: copy back only what a call changed

status: open
agent: -
branch: issue/0250-cheaper-reset
depends-on: 0249
touches: crates/djs-core/src/heap (write tracking over the sealed region), crates/djs-core/src/interp (the hot loop's raw writes), crates/djs-core/src/vm (reset_to), crates/djs-embed
guide: docs/guide/rust-api.md, docs/guide/gc.md

## Goal
Gondolin resets a pooled instance per hook call; the reset is ~3-4 µs of a ~38 µs
header-only call, and it is a full copy of the snapshot image (0249 measured reset =
memcpy of 240 KiB). A call writes few of those bytes (its globals, inline caches, the
interpreter state); the rest of the image is the sealed old region (D2041), unchanged.

Design to evaluate (and record): track which pages/cards of the sealed region a run writes
(a dirty bitmap set by the arena's write paths and by the hot loop's raw slice writes; off
unless the embedder asks for it, so the wasm modules pay nothing: D016, D020), and make
`Vm::reset_to` copy back only the dirty cards plus the young region above the image's end.
The same tracking is the card table the minor GC lacks (D2044, 0247: a minor collection
walks the whole old region, ~12% of a 64 KiB body call). Measure both against the full
copy on the hooks bench (examples/hooks-bench) and on wasm instruction counts for the
shipped module (must not regress).

## Acceptance criteria
- [ ] measured design decision (or a recorded "not worth it")
- [ ] if built: reset cost proportional to what the call wrote; images and determinism
      checks unchanged; the shipped module's size and speed unchanged

## Log

## Follow-ups

## Result
