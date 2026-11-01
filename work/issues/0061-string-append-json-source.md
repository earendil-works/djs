# 0061: In-place string append, JSON.parse source text access

status: open
agent: -
branch: issue/0061-string-append-json-source
depends-on: 0056
touches: crates/djs-core/src/heap/string.rs (+ cells.rs for a string-builder cell kind, additive; coordinate the id), crates/djs-core/src/heap/gc.rs (only to trace or compact the new cell), crates/djs-core/src/interp/ (the `+=` / concat fast path only), crates/djs-core/src/json/ + builtins/json.ts (source text access), js/src/layout.generated.ts (regenerated), tests/test262.toml (promote json-parse-with-source), ratchets

## Goal
Two follow-ups from 0056 and 0050:
1. `s += x` in a loop builds rope chains that every GC traces: 2M appends take 320 ms vs
   80 ms in qjs. Codemode builds output strings this way constantly (ARCH "Strings":
   ropes for `+=`).
2. JSON.parse source text access (`json-parse-with-source`): the `{source}` context in
   the reviver, `JSON.rawJSON`, `JSON.isRawJSON`, and stringify support.

## Acceptance criteria
- [ ] A string representation for appends: for example a flat string cell with spare
      capacity that `+=` can extend in place when the left operand is the unique owner
      (or the last link), or ropes that are flattened incrementally / at GC. Record the
      design as a decision: the heap layout (ARCH says ropes, so explain what changes and
      why), the GC behavior, snapshot compatibility, and the JS reader (0021: it must
      still read the strings).
- [ ] Benchmarks: 2M single-char appends, building a 10 MB output from lines, template
      literals in a loop, against qjs. Also check that string-heavy codemode patterns
      don't regress (the engine track).
- [ ] JSON source text access per the proposal at the test262 commit; promote the feature.
- [ ] Determinism: append behavior and heap images stay deterministic (relevant to 0055).
- [ ] Ratchets, stress modes identical, docs/wire.md (reader) updated if the layout changes.

## Notes
- heap (0060, OOM recovery) works in heap/gc and arena at the same time. Coordinate the
  cell kind id and any gc.rs change through the coordinator.

## Log

## Follow-ups

## Result
