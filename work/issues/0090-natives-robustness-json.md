# 0090: Natives: collect-and-retry, resizable typed arrays, JSON.stringify slow path

status: done
agent: infra
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
- [x] 1 with a test like 0070's oom.js but going through native builtins (map/filter/
      push in a loop at a tight cap), identical sliced and restored; D1601's test passes.
- [x] 2 with the test262 tests passing.
- [x] 3 with wasm instruction counts before/after on toJSON-heavy and replacer data.
- [x] Ratchets, both backends, stress modes and determinism checks identical.

## Log
- 2 (a53c940): three separate bugs.
  - fill: the `Set` intrinsic on a typed array with an object value fell to
    `define_own_property`, which cannot run ToNumber: "'1' is read-only" for any object
    value (not only with resizing; `ta[i] = obj` worked because `put_elem` converts its
    operands first). `set_property` now hands such a store to the prelude's
    `%TypedArraySetElementSlow` (ToNumber/ToBigInt, then the store if still valid).
  - subarray read `TypedArrayField(o, 2)`, which is 0 for an out-of-bounds view; the spec
    uses [[ByteOffset]] (new field 7), so the new view fails its bounds check.
  - Object.freeze/seal/preventExtensions: [[PreventExtensions]] of a typed array that
    is not fixed-length (length-tracking or over a resizable buffer) returns false
    (heap `prevent_extensions` + `ta_fixed_length`, heap/object.rs and heap/typed.rs;
    told the coordinator); SetIntegrityLevel throws qjs's "cannot prevent extensions".
  Corpus case builtins/typed-array-resizable.js; ratchet lines removed.
- 1 (D1900): retry in the native API (`n_retry`): the final call of a state (n_set,
  n_create_data_property_or_throw, n_call of D1700's intrinsics) and whole-statement
  leaf calls of D1700's allocating leaves / string concat (bgen `take_root` →
  `n_leaf_gc`, `n_concat_gc`). Test `native_builtins_collect_and_retry` with
  scripts/determinism/js/oom_native.js (map's doubling growth while the callback makes
  garbage): at caps 4.8 MB and 5.2 MB it printed `.EEEEE` / `..E.E.` before and all dots
  now, identical sliced (7919) and restored. Below ~4.6 MB the live data itself no
  longer fits (the retry collects and still fails: a real OOM). D1601's test passes.
  Not covered: OOM inside a perf `@fast` entry (fastpath.rs) is thrown, not retried
  (0085's concern).
- Rebase note: after a plain `git rebase main` the merge driver keeps one side of the
  generated files; run scripts/regen.sh (my first commit's prelude was stale until the
  regen in this one).
- Items 1+2 merged (main 54d984b).
- 3 (D1901): measured first with new perf workloads scripts/perf/w/json-{date,replacer,
  indent}.js (wmeasure.py, wasm instructions, M): json 1,536 (qjs-wasi 2,187), json-date
  18,157 (1,858), json-replacer 8,894 (1,390), json-indent 14,802 (1,835): the full
  algorithm was ~10x qjs. Done: subtree fast path inside the full algorithm, Dates with
  original toJSON in the fast path, array replacer keys in the fast path, a cheap
  QuoteJSONString (it set up a serializer with a 128-entry key cache per string), lazy key
  cache, the full algorithm `@native`. After: json 1,547, json-date 1,220, json-replacer
  4,130, json-indent 2,758. The native full algorithm costs 10.5 KB of wasm for -20% on
  json-replacer (5,156 → 4,150 without/with); kept, easy to drop. Corpus case
  builtins/json-stringify-paths.js.
- Remaining replacer-function cost: per key a call of the replacer plus string concat
  (heap `concat` collects leaves into Vecs, perf F6) and GC: ~3x qjs-wasi.

## Follow-ups
- JSON with a replacer function: ~3x quickjs-wasi; a string builder for the full
  algorithm instead of concat/partial arrays would be the next step.
- OOM inside a perf `@fast` entry (fastpath.rs) is thrown without D1700's retry.

## Result
- Native builtins collect and retry at the heap cap (D1900): `Exec::n_retry` in the
  native API for a state's final call (n_set, n_create_data_property_or_throw, n_call of
  D1700's intrinsics) and for whole-statement calls of D1700's allocating leaves and
  string concat (djs-bgen emits `n_leaf_gc` / `n_concat_gc`). Test
  `native_builtins_collect_and_retry` (scripts/determinism/js/oom_native.js).
- The 3 resizable typed-array test262 tests pass: [[Set]] of a typed array element with
  an object value converts through `%TypedArraySetElementSlow` (it threw "read-only" for
  any object value through `Set`), subarray uses [[ByteOffset]], [[PreventExtensions]]
  of a typed array that is not fixed-length is false (heap `prevent_extensions`,
  `ta_fixed_length`). Corpus builtins/typed-array-resizable.js.
- JSON.stringify (D1901): plain subtrees, Dates and array replacers in the fast path,
  cheaper quoting, the full algorithm native. wasm instructions: toJSON/Date data
  18.1G → 1.2G (qjs-wasi 1.9G), getters + indent 14.8G → 2.8G (1.8G), replacers 8.9G
  → 4.1G (1.4G). Corpus builtins/json-stringify-paths.js; perf workloads
  scripts/perf/w/json-{date,replacer,indent}.js.
- Known gaps: replacer functions stay ~3x qjs-wasi; `@fast` entries do not retry at
  the cap.

