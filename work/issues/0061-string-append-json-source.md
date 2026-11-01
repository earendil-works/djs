# 0061: In-place string append, JSON.parse source text access

status: done
agent: num
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
- **Design approved by the coordinator** (append ropes, no new cell kind, no gc.rs
  change), with the invariants (a)-(c). D1610 amends ARCH "Strings".
- **Changes** (heap/string.rs, cells.rs flags, walk.rs verify, layout.rs flag export,
  js/src/reader.ts, docs/wire.md):
  - concat: in place when the rope is at the buffer's end, a new doubled buffer for
    chains (once the left side is 256 units or more), ropes otherwise;
  - leaves carry clipped lengths;
  - str_view and str_view_flat read append ropes in place;
  - flatten freezes or copies.
- **Tests**:
  - heap/append_tests.rs (heap/mod.rs: one `mod` line): sharing; (b) an older shorter
    rope copies; (c) freezing; widening; (a) verify rejects escaped buffers; collect
    twice gives identical bytes; the same operations give the same image; image round
    trip with appends after the restore.
  - wire/tests.rs reader cases: append-rope, prefix, two-byte, all checked by
    reader-cross.ts.
  - tests/corpus/builtins/string-append.js, identical to qjs and under stress.
- **Profiling notes.** The first version was slower than ropes. A piece allocated after
  the buffer (a constant, after a GC) fell into a Vec-collecting path, and template
  literal results (ropes of 32+ units) allocated per append. Now flat pieces copy with
  `copy_within` and small ropes are walked on a stack array. Expression temporaries
  first got their own buffers, which cost 40 MB on app2c; buffers now start at 256
  units.
- **Numbers**, release djs-cli, min of 3:
  - 10M `s += "x"` in a function: 1.13 s / 574 MB -> 0.93 s / 150 MB (qjs 0.6 s).
  - 20M at the top level: RSS 924 MB -> about 180-290 MB.
  - 300k template literals: 0.21 s / 172 MB -> 0.17 s / 107 MB (qjs 0.15 s).
  - 2M lines with a number: 60 ms both, 48 MB both.
  - engine/djs/string-append (new): 65 -> 60 ms (qjs 35).
  - The rest of the engine track moves within layout noise: main plus one atom moves
    loop.js by ±10%.
- **JSON source text access** (D1611): records from a source walk, the reviver context,
  rawJSON/isRawJSON with ObjectClass::RawJSON = 29 (from the coordinator), and the
  stringify raw path. test262 built-ins/JSON 142/142 active and 21/21 of the former
  later group. Active 37,182 -> 37,203 after promoting json-parse-with-source.
  tests/corpus/builtins/json-source.js matches qjs. Duplicate keys follow the spec, not
  qjs, and the case avoids them.

## Follow-ups
- Mixed append-and-flatten loops (`s += x; s.indexOf(...)` with a callee that calls
  `flatten`) freeze the buffer and copy on the next append. That is no worse than
  ropes. Leaves that only read should use `str_view_flat`, which reads append ropes in
  place.
- The interpreter's per-iteration cost (global lexical lookups, `conv::add`) is now most
  of the gap to qjs in append loops.

## Result
- `s += x` chains build into a growable buffer in place (append ropes, D1610). Memory
  for append loops drops 3-6x and time 10-20%, with invariants enforced by verify,
  deterministic images and reader support.
- JSON.parse source text access, JSON.rawJSON and JSON.isRawJSON (D1611); the feature is
  promoted, 21 tests more active.
