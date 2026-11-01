# 0090: Natives: collect-and-retry, resizable typed arrays, JSON.stringify slow path

status: open
agent: -
branch: issue/0090-natives-robustness-json
depends-on: 0080
touches: crates/djs-core/src/interp/run/native.rs + builtins/gen/ (generated), crates/djs-bgen/, crates/djs-core/src/builtins/ (typed arrays), crates/djs-core/src/json/stringify.rs + builtins/json.ts, ratchets

## Goal
0080's leftovers that don't wait for 0085:
1. Native builtins have no collect-and-retry on a cap OOM (0070's follow-up): an allocation
   failure inside a native builtin is an immediate catchable OOM even when garbage could be
   collected. Extend D1700 to native frames (the state-machine arms make this natural:
   no GC inside an arm, so a failing allocation can return to the loop, collect, and
   re-enter the same state).
2. The 3 test262 failures on typed arrays over resizable buffers that shrink during
   `fill`/`freeze`/`subarray` (0082).
3. JSON.stringify's slow path (replacer/toJSON/accessors/wrappers) as native, or a
   subtree fast path in json/stringify.rs so a single toJSON doesn't push the whole value
   to the slow path; measure first (JSON is already 0.5–0.6x qjs on plain data).

## Acceptance criteria
- [ ] 1 with a test like 0070's oom.js but going through native builtins (map/filter/
      push in a loop at a tight cap), identical sliced and restored; D1601's test passes.
- [ ] 2 with the test262 tests passing.
- [ ] 3 with wasm instruction counts before/after on toJSON-heavy and replacer data.
- [ ] Ratchets, both backends, stress modes and determinism checks identical.

## Log

## Follow-ups

## Result
