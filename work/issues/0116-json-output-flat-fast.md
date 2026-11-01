# 0116: JSON output written in place; flat_fast inline; next pi-like hot spots

status: in-progress
agent: perf
branch: issue/0116-json-output-flat-fast
depends-on: 0110
guide: docs/guide/builtins.md (if the JSON or string paths it describes change)
touches: crates/djs-core/src/json/, crates/djs-core/src/heap/string.rs (flat_fast, cleared), crates/djs-core/src/builtins/ (json leaves)

## Goal
0110's follow-ups: JSON (~9% of pi-like) builds its output in a Rust Vec and copies it into
the arena again (write it in place, as the 0012 follow-up suggested); `flat_fast` is still an
out-of-line call in wasm (~0.9%); then re-profile pi-like and take the next hot spots that
aren't the GC (heap's, after 0101).

## Acceptance criteria
- [ ] wasm instruction counts before/after (pi-like, json, jsonh, json-*), size, no behavior
      change (the json corpus, stress, determinism).

## Log

## Follow-ups

## Result
