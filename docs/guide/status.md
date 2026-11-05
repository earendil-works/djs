# Status in numbers

Where djs stands, in numbers that cannot go stale: every number on this page is
generated. Counts that follow from the repository (test262's grouping and
ratchet, the difftest corpus, the base snapshot, the builtin tags, the engine's tables)
are recomputed whenever the guide is regenerated, which the merge flow does on every
merge. Measurements that need a build or a benchmark (the wasm module's size, the speed
against quickjs-wasi, an agent's codemode test suite) are recorded in `tests/scores/status.tsv` by
`scripts/status.py`, with the commit they were measured at, and shown with it. The
repository's front page shows the same numbers.

## Conformance

<!-- gen:status conformance -->
- test262 (at dquickts' pinned commit): djs passes **38,069 of the 38,109** tests in the
  active group (99.9%; 53,597 tests in all, the rest skipped by design). The 40 known
  failures are listed with reasons in `tests/test262/expected-failures.txt`; dquickts'
  qjs fails 4 active tests under the same runner.
- The differential corpus (codemode scripts, console and result formatting, error
  messages, TypeScript, dquickts' own tests): **621 of 632** cases match qjs; 6 known
  failures (`tests/corpus/expected-failures.txt`) and 5 skipped as non-goals.
<!-- /gen -->

[How djs is tested](testing.md) explains the suites and the ratchets.

## Speed in wasm

Machine instructions retired by node running each workload once, minus an empty run, for
djs-wasm and for quickjs-wasi, the QuickJS build that codemode hosts use today
(`scripts/perf/wmeasure.py`).
Instructions repeat to within a fraction of a percent even on a loaded machine, unlike
time.

<!-- gen:status speed -->
| workload | djs-wasm | quickjs-wasi | ratio |
| --- | ---: | ---: | ---: |
| tight loop | 6.6G | 15.7G | 0.42 |
| property access (prop1) | 2.7G | 5.5G | 0.49 |
| strings (split, slice, join, ...) | 3.1G | 6.0G | 0.51 |
| arrays | 10.8G | 16.8G | 0.64 |
| JSON round trips | 1.3G | 2.2G | 0.57 |
| JSON with Dates | 0.9G | 1.9G | 0.48 |
| for-of | 5.8G | 7.2G | 0.81 |
| calls | 3.3G | 4.0G | 0.81 |
| push/pop and string methods | 4.3G | 3.1G | 1.40 |
| an agent-like script | 10.0G | 7.3G | 1.38 |
| forEach | 9.0G | 6.1G | 1.49 |
| map/filter/reduce (higher-order) | 13.9G | 4.8G | 2.92 |

Machine instructions per workload (`scripts/perf/w/`), measured at d780463b0d,
2026-11-01; lower is better.
<!-- /gen -->

## Size

<!-- gen:status size -->
`djs_wasm.wasm` is 1,471 KB (525 KB gzip, 423 KB brotli): 901 KB of code and 566 KB of
data, of which 186 KB is the base snapshot (the initialized realm with all builtins).
Module sizes measured at d780463b0d, 2026-11-01.
<!-- /gen -->

## The engine in numbers

<!-- gen:status inventory -->
| | count |
| --- | ---: |
| opcodes ([reference](ref-opcodes.md)) | 214 |
| cell kinds ([reference](ref-cells.md)) | 26 |
| intrinsics ([reference](ref-intrinsics.md)) | 452 |
| builtins in `builtins/*.ts` | 455 |
| of them `@native` (also compiled to Rust) | 8 |
| of them `@fast` (a Rust leaf entry) | 18 |
| difftest corpus cases | 632 |
| base snapshot, bytes | 186,376 |
<!-- /gen -->
