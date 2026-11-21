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
- test262 (at dquickts' pinned commit): djs passes **39,325 of the 39,365** tests in the
  active group (99.9%; 53,597 tests in all, the rest skipped by design). The 40 known
  failures are listed with reasons in `tests/test262/expected-failures.txt`; dquickts'
  qjs fails 17 active tests under the same runner.
- The differential corpus (codemode scripts, console and result formatting, error
  messages, TypeScript, dquickts' own tests): **655 of 666** cases match qjs; 6 known
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
| tight loop | 3.9G | 15.3G | 0.26 |
| property access (prop1) | 1.5G | 5.4G | 0.28 |
| strings (split, slice, join, ...) | 1.6G | 5.8G | 0.28 |
| arrays | 5.2G | 16.3G | 0.32 |
| JSON round trips | 1.0G | 2.0G | 0.50 |
| JSON with Dates | 0.8G | 1.7G | 0.46 |
| for-of | 3.5G | 7.0G | 0.50 |
| calls | 2.1G | 3.9G | 0.53 |
| push/pop and string methods | 2.9G | 2.9G | 1.00 |
| an agent-like script | 5.4G | 6.9G | 0.78 |
| forEach | 3.6G | 5.8G | 0.62 |
| map/filter/reduce (higher-order) | 3.9G | 4.5G | 0.85 |

Machine instructions per workload (`scripts/perf/w/`), measured at 1a4c20de31,
2026-11-21; lower is better.
<!-- /gen -->

## Size

<!-- gen:status size -->
`djs_wasm.wasm` is 992 KB (429 KB gzip, 363 KB brotli): 854 KB of code, 41 KB of data,
and the base snapshot (the initialized realm with all builtins, 191 KB) coded to 45 KB
in a custom section. The slim build is 864 KB (363 KB gzip); the dump module, the
shipped build with the dumper, 1,884 KB (774 KB gzip). Module sizes measured at
f8cb1e7924, 2026-11-21.
<!-- /gen -->

## The engine in numbers

<!-- gen:status inventory -->
| | count |
| --- | ---: |
| opcodes ([reference](ref-opcodes.md)) | 216 |
| cell kinds ([reference](ref-cells.md)) | 26 |
| intrinsics ([reference](ref-intrinsics.md)) | 473 |
| builtins in `builtins/*.ts` | 458 |
| of them `@native` (also compiled to Rust) | 8 |
| of them `@fast` (a Rust leaf entry) | 50 |
| difftest corpus cases | 666 |
| base snapshot, bytes | 190,920 |
<!-- /gen -->

## Every recorded measurement

Each line is one number in `tests/scores/status.tsv`, with the commit it was measured
at; `scripts/status.py` says how each is taken. The measurements of the agent host djs
was built for, its tests and its benchmark, are listed [on its page](pi.md). Sizes are in bytes, times in
milliseconds (`-ms`) or microseconds (`-us`), speeds in millions of machine
instructions.

<!-- gen:status recorded -->
| measurement | value | measured at |
| --- | ---: | --- |
| `bounds/gzip` | 9222 | 9041ad864d, 2026-11-21 |
| `bounds/instructions-max-pct` | 31.4 | 9041ad864d, 2026-11-21 |
| `bounds/instructions-median-pct` | 23.5 | 9041ad864d, 2026-11-21 |
| `bounds/load` | 3.3 | 9041ad864d, 2026-11-21 |
| `compile/typescript-lazy-time-pct` | 57 | 7989df32e2, 2026-11-21 |
| `dump/bytes` | 1884014 | f8cb1e7924, 2026-11-21 |
| `dump/extra-gzip` | 344745 | f8cb1e7924, 2026-11-21 |
| `dump/gzip` | 774164 | f8cb1e7924, 2026-11-21 |
| `dumps/corpus-cases` | 300 | 4af5a6f108, 2026-11-21 |
| `dumps/idle-points` | 1063 | 4af5a6f108, 2026-11-21 |
| `dumps/load` | 3.7 | 4af5a6f108, 2026-11-21 |
| `dumps/log-raw` | 1758796 | 4af5a6f108, 2026-11-21 |
| `dumps/log-zstd` | 402678 | 4af5a6f108, 2026-11-21 |
| `dumps/raw-p50` | 2573 | 4af5a6f108, 2026-11-21 |
| `dumps/raw-p99` | 12378 | 4af5a6f108, 2026-11-21 |
| `dumps/refused` | 0 | 4af5a6f108, 2026-11-21 |
| `dumps/resume-ms-p50` | 0.279 | 4af5a6f108, 2026-11-21 |
| `dumps/resume-ms-p99` | 2.119 | 4af5a6f108, 2026-11-21 |
| `dumps/write-ms-p50` | 0.233 | 4af5a6f108, 2026-11-21 |
| `dumps/write-ms-p99` | 1.601 | 4af5a6f108, 2026-11-21 |
| `dumps/zstd-p50` | 1006 | 4af5a6f108, 2026-11-21 |
| `dumps/zstd-p99` | 2962 | 4af5a6f108, 2026-11-21 |
| `fuel/slice-us` | 9.2 | 93f515bb17, 2026-11-21 |
| `heap/append-10m-memory` | 33947648 | 51646c9e90, 2026-11-21 |
| `heap/json5/dictionary` | 152.0 | 51646c9e90, 2026-11-21 |
| `heap/json5/shape` | 104.0 | 51646c9e90, 2026-11-21 |
| `heap/literal4/dictionary` | 112.0 | 51646c9e90, 2026-11-21 |
| `heap/literal4/shape` | 72.0 | 51646c9e90, 2026-11-21 |
| `heap/load` | 3.7 | 51646c9e90, 2026-11-21 |
| `heap/sandbox-memory` | 393216 | 51646c9e90, 2026-11-21 |
| `heap/string-10mib-memory` | 15990784 | 51646c9e90, 2026-11-21 |
| `heap/string-10mib-staged-memory` | 31981568 | 51646c9e90, 2026-11-21 |
| `native/arrays-x` | 3.74 | 35ed5ab789, 2026-11-05 |
| `native/foreach-x` | 2.1 | 35ed5ab789, 2026-11-05 |
| `slim/base` | 133624 | f8cb1e7924, 2026-11-21 |
| `slim/bytes` | 863569 | f8cb1e7924, 2026-11-21 |
| `slim/gzip` | 363363 | f8cb1e7924, 2026-11-21 |
| `speed/arrays/djs-wasm` | 5240.5 | 1a4c20de31, 2026-11-21 |
| `speed/arrays/qjs-wasi` | 16343.3 | 1a4c20de31, 2026-11-21 |
| `speed/calls/djs-wasm` | 2072.4 | 1a4c20de31, 2026-11-21 |
| `speed/calls/qjs-wasi` | 3905.1 | 1a4c20de31, 2026-11-21 |
| `speed/foreach/djs-wasm` | 3592.8 | 1a4c20de31, 2026-11-21 |
| `speed/foreach/qjs-wasi` | 5788.8 | 1a4c20de31, 2026-11-21 |
| `speed/forof/djs-wasm` | 3471.4 | 1a4c20de31, 2026-11-21 |
| `speed/forof/qjs-wasi` | 6953.7 | 1a4c20de31, 2026-11-21 |
| `speed/higher-order/djs-wasm` | 3867.9 | 1a4c20de31, 2026-11-21 |
| `speed/higher-order/qjs-wasi` | 4540.0 | 1a4c20de31, 2026-11-21 |
| `speed/json-date/djs-wasm` | 784.8 | 1a4c20de31, 2026-11-21 |
| `speed/json-date/qjs-wasi` | 1717.4 | 1a4c20de31, 2026-11-21 |
| `speed/json/djs-wasm` | 1021.2 | 1a4c20de31, 2026-11-21 |
| `speed/json/qjs-wasi` | 2028.4 | 1a4c20de31, 2026-11-21 |
| `speed/load` | 3.8 | 1a4c20de31, 2026-11-21 |
| `speed/prop1/djs-wasm` | 1509.5 | 1a4c20de31, 2026-11-21 |
| `speed/prop1/qjs-wasi` | 5397.9 | 1a4c20de31, 2026-11-21 |
| `speed/push-pop/djs-wasm` | 2874.1 | 1a4c20de31, 2026-11-21 |
| `speed/push-pop/qjs-wasi` | 2874.9 | 1a4c20de31, 2026-11-21 |
| `speed/strings/djs-wasm` | 1610.4 | 1a4c20de31, 2026-11-21 |
| `speed/strings/qjs-wasi` | 5774.8 | 1a4c20de31, 2026-11-21 |
| `speed/tight/djs-wasm` | 3921.6 | 1a4c20de31, 2026-11-21 |
| `speed/tight/qjs-wasi` | 15335.4 | 1a4c20de31, 2026-11-21 |
| `startup/instantiate-us` | 17.6 | 8643704466, 2026-11-21 |
| `startup/load` | 3.7 | 8643704466, 2026-11-21 |
| `startup/sandbox-run-us` | 72.2 | 8643704466, 2026-11-21 |
| `startup/sandbox-us` | 55.5 | 8643704466, 2026-11-21 |
| `startup/vm-new-us` | 2.6 | 8643704466, 2026-11-21 |
| `wasm/base-coded` | 45014 | f8cb1e7924, 2026-11-21 |
| `wasm/brotli` | 363126 | f8cb1e7924, 2026-11-21 |
| `wasm/bytes` | 991808 | f8cb1e7924, 2026-11-21 |
| `wasm/code` | 854050 | f8cb1e7924, 2026-11-21 |
| `wasm/data` | 41133 | f8cb1e7924, 2026-11-21 |
| `wasm/gzip` | 429419 | f8cb1e7924, 2026-11-21 |
| `wasm/ops` | 69 | f8cb1e7924, 2026-11-21 |
| `wire/decode-vs-json-objects` | 2.21 | 6738f3ebb5, 2026-11-21 |
| `wire/decode-vs-json-string` | 2.86 | 6738f3ebb5, 2026-11-21 |
| `wire/read-vs-json-objects` | 1.58 | 6738f3ebb5, 2026-11-21 |
| `wire/read-vs-json-text` | 9.06 | 6738f3ebb5, 2026-11-21 |
<!-- /gen -->
