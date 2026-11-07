# 0244: TextEncoder and TextDecoder (UTF-8) (B1)

status: done
agent: encoding
branch: issue/0244-text-encoding
depends-on: -
touches: builtins/ (a new encoding.ts), builtins/intrinsics.d.ts, crates/djs-core/src/interp/run/intrinsics.rs (leaf intrinsics), crates/djs-core/src/interp/realm.rs (ABSENT_GLOBALS for slim), tests/builtins/registry*.tsv/txt, tests/corpus (cases), tests/scores/size.tsv (only if D3172 allows)
guide: docs/guide/language.md (builtins list)

## Goal

D3172. `TextEncoder` (`encode`, `encodeInto`, `encoding`) and `TextDecoder` (`utf-8` only: the
Encoding standard's labels for UTF-8, `fatal`, `ignoreBOM`, `decode(input, { stream })` with
state kept across calls, `encoding`/`fatal`/`ignoreBOM` getters; input any BufferSource:
ArrayBuffer, typed arrays, DataView), per the WHATWG Encoding standard (WebIDL conversions
and error types as browsers/node: RangeError for unsupported labels, TypeError for fatal decode
errors with node's/the spec's message where one exists). Lone surrogates encode as U+FFFD.
Over leaf intrinsics that reuse the engine's UTF-8 transcoders (wire decode, `str_to_string`,
the base64/hex code of Uint8Array) where possible, fuel-charged by bytes.

Full build only (`// @slim-omit`, globals in `realm::ABSENT_GLOBALS`, registry-slim). Measure the
shipped module's size: if it costs more than 1.5 KB gzip, stop and report (D3172 says to cut it
from the shipped module like a slim family then); otherwise raise `ceiling_gzip` by exactly the
measured amount with a size decision (as D3164 does), approved: coordinator.

## Acceptance criteria
- [x] both classes with the full option surface above; `toString` tags, `length`s, property
      attributes as WebIDL classes in browsers (check against node)
- [x] tests: corpus cases where qjs can be the oracle are not possible (qjs has no
      TextEncoder): use e2e/unit tests with expectations checked against node, and mark them so
- [x] registry and slim updates; size measured and recorded
- [x] performance: encoding/decoding 64 KiB ASCII and non-ASCII is memcpy-like / one pass
      (measure; native release)

## Log

2026-11-07 (encoding):
- Implemented both classes (builtins/encoding.ts, crates/djs-core/src/builtins/encoding.rs,
  `ObjectClass::TextCodec` = 35, 5 leaf intrinsics in one dispatch arm, `// @slim-omit`,
  `ABSENT_GLOBALS`, registry + registry-slim, js/src/layout.generated.ts, node's prototype
  key order via crates/djs-bgen/src/qjs_order.rs (outside `touches`, 2 entries),
  vm/mod.rs one `mod` line). Design in D3210.
- Tests: tests/data/encoding/cases.js + cases.expected (node's transcript, regenerate with
  gen-expected.sh; labels, options, BufferSource kinds incl. detached/resizable/OOB, BOM,
  fatal, streaming splits, encodeInto short destinations and surrogate pairs, 400 random
  byte strings/strings hashed), run by `cargo test -p djs-core --lib vm::encoding_tests`,
  which also asserts the deliberate differences from node (WebIDL conversion order,
  UTF-8-only labels, messages) and fuel by bytes. tests/corpus/builtins/
  prototype-property-order.js skips the two globals (qjs has neither). All vm:: tests pass.
- Guide: docs/guide/language.md (builtins list, "left out" table, slim list); regen'd pages.
- **Size (blocked here)**: shipped module (scripts/build-wasm.sh, budget.py), branch point
  57675c79 426,792 gzip (980,158 raw) -> 429,179 (984,807 raw): **+2,387 gzip**, over the
  1.5 KB D3172 allows. Parts: code +3.5 KB raw (+1.36 KB gzip; the leaves, mostly
  decode with streaming/fatal/BOM and encode/encodeInto), the base image section +944
  (10 function templates, ~25 strings: messages, labels, names; 2 prototypes), data +83.
  Iterations: first version +3,167 (own UTF-8 encoder/decoder, labels and BufferSource
  ranges in Rust) -> labels and ranges moved into the builtin, leaves over the existing
  `from_utf8_lossy`/`alloc_str`/`str_to_string` -> +2,349; encodeInto as a truncated
  encode; one dispatch arm (no gain). Still 1.497x quickjs-wasi, under the 1.5x budget
  (429,982) by ~800 bytes. Per the issue I stop and report; ceiling not raised.
- Performance (native release, machine at load 110-150, so only rough, best of runs, per
  64 KiB op): decode ASCII ~36 us (one copy into the string), decode mixed ~100 us,
  encode ASCII ~14 us (copy), encode mixed ~250 us (str_to_string per code unit: the
  slowest path; a dedicated UTF-16 -> UTF-8 loop over the flat view would be faster but
  costs code), encodeInto similar. One pass each; node's encode mixed is ~50 us.
- Next steps for whoever continues (coordinator decides first):
  1. Raise ceiling_gzip by the measured amount (re-measure after rebasing on main) with a
     size decision D3211, approved: coordinator; or
  2. cut the classes from the shipped module (a djs-core feature off in djs-wasm's shipped
     build only): needs a third prelude variant (bgen, djs-prelude's build, registry),
     much bigger than this issue; or
  3. more size work: shorter messages (node's are long; ~100 B), fewer string constants,
     leaner decode state handling; my estimate of the floor is ~1.8-2.0 KB.
  Then: merge (scripts/merge-to-main.sh); the gate also runs the slim legs, not run here.

2026-11-08 (encoding, continuation after D3175):
- Rebased onto main e269da5f (0241's embed/prelude-noembed); generated files taken from
  main in the conflicts and rewritten by scripts/regen.sh --layout.
- D3175: builtins/encoding.ts is `// @embed-only` (was `@slim-omit`), the leaves' dispatch
  arm is `if crate::EMBED`, vm::encoding_tests are `cfg(feature = "embed")`. The globals
  moved from `ABSENT_GLOBALS` to a new `realm::EMBED_GLOBALS`: builds without `embed` say
  "'TextEncoder' is not available in this build" (run.rs `undefined_global`);
  slim_tests::absent_globals and djs-prelude's registry test cover both lists (checked in
  default, slim, and `--no-default-features --features full,unicode` builds).
- vm::tests::reset_to_reuses_the_arena (0240's) failed after the rebase: its arena was only
  ~6 KB larger than the image, and the bigger base crossed it. It now makes an arena more
  than twice the image (an `Array(1 << 17)`), so the base's size cannot decide it.
- Size (scripts/build-wasm.sh, against main e269da5f's gate numbers): shipped 426,839 ->
  426,951 gzip (+112), slim 360,536 -> 360,610 (+74). Ceilings raised by exactly that,
  D3211 (approved: coordinator per D3175). D3210 amended with the embed-only part.
- djs-embed's cross_check example prints a `utf8` line (encode of the fetched body, decode
  with a split/invalid tail); outside `touches`, as are vm/tests.rs, run.rs, slim_tests.rs
  and crates/djs-prelude/tests/registry.rs.
- The gate's djs-bgen behavior test (prelude.js run in qjs) failed: qjs has no
  TextEncoder/TextDecoder for `InstallConstructor`/`RealmConstructor` to take the prototype
  from. crates/djs-bgen/tests/behavior.js now gives such a constructor a fresh prototype
  and resolves `RealmConstructor` to the installed one (outside `touches`).

## Follow-ups
- Dumps: a TextEncoder/TextDecoder object in a plain dump is not handled by the dumper
  (vm/dump.rs knows no TextCodec class); probably refused like other exotic classes.
- A faster encode for non-ASCII strings (a UTF-16 -> UTF-8 loop over the flat view) if a
  benchmark needs it.

## Result
TextEncoder and TextDecoder (UTF-8, the full option surface, D3210) in builds with
djs-core's `embed` (the native ones, djs-embed), `// @embed-only` per D3175; the wasm
modules leave them out and name them in their ReferenceError. Tests against node's
transcript (tests/data/encoding, vm::encoding_tests). Size: shipped +112 B gzip, slim
+74 B, ceilings raised by that (D3211). Performance as logged (one pass each; non-ASCII
encode ~5x node's, follow-up).
