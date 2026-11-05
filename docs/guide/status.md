# Status in numbers

Where djs stands, in numbers that cannot go stale: every number on this page is
generated. Counts that follow from the repository (test262's grouping and
ratchet, the difftest corpus, the base snapshot, the builtin tags, the engine's tables)
are recomputed whenever the guide is regenerated, which the merge flow does on every
merge. Measurements that need a build or a benchmark (the wasm module's size, the speed
against quickjs-wasi, an agent's codemode test suite) are recorded in `tests/scores/status.tsv` by
`scripts/status.py`, with the commit they were measured at, and shown with it. The
repository's front page shows the same numbers. Single figures in the prose of other
pages, such as what a dump costs or how many bytes an object takes, come from the same
records; [the last section](#every-recorded-measurement) lists them all.

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
| tight loop | 5.8G | 15.3G | 0.38 |
| property access (prop1) | 2.4G | 5.4G | 0.45 |
| strings (split, slice, join, ...) | 2.2G | 5.8G | 0.38 |
| arrays | 9.5G | 16.3G | 0.58 |
| JSON round trips | 1.2G | 2.0G | 0.60 |
| JSON with Dates | 0.9G | 1.7G | 0.52 |
| for-of | 5.1G | 6.9G | 0.73 |
| calls | 2.9G | 3.9G | 0.75 |
| push/pop and string methods | 3.8G | 2.9G | 1.33 |
| an agent-like script | 5.8G | 6.9G | 0.84 |
| forEach | 5.4G | 5.8G | 0.93 |
| map/filter/reduce (higher-order) | 7.8G | 4.5G | 1.73 |

Machine instructions per workload (`scripts/perf/w/`), measured at 12f469900e,
2026-11-05; lower is better.
<!-- /gen -->

## Size

<!-- gen:status size -->
`djs_wasm.wasm` is 1,034 KB (443 KB gzip, 372 KB brotli): 868 KB of code, 116 KB of
data, and the base snapshot (the initialized realm with all builtins, 186 KB) coded to
46 KB in a custom section. The slim build is 906 KB (378 KB gzip); the dump module, the
shipped build with the dumper, 1,966 KB (785 KB gzip). Module sizes measured at
1389e5d611, 2026-11-05.
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

## Every recorded measurement

Each line is one number in `tests/scores/status.tsv`, with the commit it was measured
at; `scripts/status.py` says how each is taken. Sizes are in bytes, times in
milliseconds (`-ms`) or microseconds (`-us`), speeds in millions of machine
instructions.

<!-- gen:status recorded -->
<!-- /gen -->
