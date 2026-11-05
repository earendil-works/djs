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
86b0e17f6f, 2026-11-05.
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
at; `scripts/status.py` says how each is taken. The measurements of the agent host djs
was built for, its tests and its benchmark, are listed [on its page](pi.md). Sizes are in bytes, times in
milliseconds (`-ms`) or microseconds (`-us`), speeds in millions of machine
instructions.

<!-- gen:status recorded -->
| measurement | value | measured at |
| --- | ---: | --- |
| `bounds/gzip` | 7895 | 3af283e33e, 2026-11-05 |
| `bounds/instructions-max-pct` | 16.8 | 3af283e33e, 2026-11-05 |
| `bounds/instructions-median-pct` | 10.9 | 3af283e33e, 2026-11-05 |
| `compile/typescript-lazy-time-pct` | 56 | fb9223c8c4, 2026-11-05 |
| `dump/bytes` | 1966073 | 86b0e17f6f, 2026-11-05 |
| `dump/extra-gzip` | 342133 | 86b0e17f6f, 2026-11-05 |
| `dump/gzip` | 784722 | 86b0e17f6f, 2026-11-05 |
| `dumps/corpus-cases` | 300 | bf7e855bbd, 2026-11-05 |
| `dumps/idle-points` | 1063 | bf7e855bbd, 2026-11-05 |
| `dumps/log-raw` | 1758604 | bf7e855bbd, 2026-11-05 |
| `dumps/log-zstd` | 402669 | bf7e855bbd, 2026-11-05 |
| `dumps/raw-p50` | 2573 | bf7e855bbd, 2026-11-05 |
| `dumps/raw-p99` | 12378 | bf7e855bbd, 2026-11-05 |
| `dumps/refused` | 0 | bf7e855bbd, 2026-11-05 |
| `dumps/resume-ms-p50` | 0.391 | bf7e855bbd, 2026-11-05 |
| `dumps/resume-ms-p99` | 2.464 | bf7e855bbd, 2026-11-05 |
| `dumps/write-ms-p50` | 0.258 | bf7e855bbd, 2026-11-05 |
| `dumps/write-ms-p99` | 1.733 | bf7e855bbd, 2026-11-05 |
| `dumps/zstd-p50` | 1006 | bf7e855bbd, 2026-11-05 |
| `dumps/zstd-p99` | 2962 | bf7e855bbd, 2026-11-05 |
| `fuel/slice-us` | 13.0 | 80f9c059ef, 2026-11-05 |
| `heap/append-10m-memory` | 40501248 | ca860bee5d, 2026-11-05 |
| `heap/json5/dictionary` | 152.0 | ca860bee5d, 2026-11-05 |
| `heap/json5/shape` | 104.0 | ca860bee5d, 2026-11-05 |
| `heap/literal4/dictionary` | 112.0 | ca860bee5d, 2026-11-05 |
| `heap/literal4/shape` | 72.0 | ca860bee5d, 2026-11-05 |
| `heap/sandbox-memory` | 655360 | ca860bee5d, 2026-11-05 |
| `heap/string-10mib-memory` | 14024704 | ca860bee5d, 2026-11-05 |
| `heap/string-10mib-staged-memory` | 35127296 | ca860bee5d, 2026-11-05 |
| `native/arrays-x` | 3.97 | 314504b486, 2026-11-05 |
| `native/foreach-x` | 2.0 | 314504b486, 2026-11-05 |
| `slim/base` | 135056 | 86b0e17f6f, 2026-11-05 |
| `slim/bytes` | 906238 | 86b0e17f6f, 2026-11-05 |
| `slim/gzip` | 378178 | 86b0e17f6f, 2026-11-05 |
| `speed/arrays/djs-wasm` | 9518.0 | 12f469900e, 2026-11-05 |
| `speed/arrays/qjs-wasi` | 16337.5 | 12f469900e, 2026-11-05 |
| `speed/calls/djs-wasm` | 2915.0 | 12f469900e, 2026-11-05 |
| `speed/calls/qjs-wasi` | 3893.8 | 12f469900e, 2026-11-05 |
| `speed/foreach/djs-wasm` | 5389.2 | 12f469900e, 2026-11-05 |
| `speed/foreach/qjs-wasi` | 5773.7 | 12f469900e, 2026-11-05 |
| `speed/forof/djs-wasm` | 5067.8 | 12f469900e, 2026-11-05 |
| `speed/forof/qjs-wasi` | 6941.7 | 12f469900e, 2026-11-05 |
| `speed/higher-order/djs-wasm` | 7821.1 | 12f469900e, 2026-11-05 |
| `speed/higher-order/qjs-wasi` | 4530.0 | 12f469900e, 2026-11-05 |
| `speed/json-date/djs-wasm` | 890.1 | 12f469900e, 2026-11-05 |
| `speed/json-date/qjs-wasi` | 1705.2 | 12f469900e, 2026-11-05 |
| `speed/json/djs-wasm` | 1207.7 | 12f469900e, 2026-11-05 |
| `speed/json/qjs-wasi` | 2017.4 | 12f469900e, 2026-11-05 |
| `speed/prop1/djs-wasm` | 2410.8 | 12f469900e, 2026-11-05 |
| `speed/prop1/qjs-wasi` | 5400.9 | 12f469900e, 2026-11-05 |
| `speed/push-pop/djs-wasm` | 3793.8 | 12f469900e, 2026-11-05 |
| `speed/push-pop/qjs-wasi` | 2861.9 | 12f469900e, 2026-11-05 |
| `speed/strings/djs-wasm` | 2221.6 | 12f469900e, 2026-11-05 |
| `speed/strings/qjs-wasi` | 5773.3 | 12f469900e, 2026-11-05 |
| `speed/tight/djs-wasm` | 5834.4 | 12f469900e, 2026-11-05 |
| `speed/tight/qjs-wasi` | 15335.5 | 12f469900e, 2026-11-05 |
| `startup/instantiate-us` | 40.7 | 5e9fd4c6bc, 2026-11-05 |
| `startup/sandbox-run-us` | 141.8 | 5e9fd4c6bc, 2026-11-05 |
| `startup/sandbox-us` | 112.7 | 5e9fd4c6bc, 2026-11-05 |
| `startup/vm-new-us` | 2.5 | 5e9fd4c6bc, 2026-11-05 |
| `wasm/base-coded` | 45685 | 86b0e17f6f, 2026-11-05 |
| `wasm/brotli` | 371761 | 86b0e17f6f, 2026-11-05 |
| `wasm/bytes` | 1033538 | 86b0e17f6f, 2026-11-05 |
| `wasm/code` | 867720 | 86b0e17f6f, 2026-11-05 |
| `wasm/data` | 115665 | 86b0e17f6f, 2026-11-05 |
| `wasm/exports` | 63 | 86b0e17f6f, 2026-11-05 |
| `wasm/gzip` | 442589 | 86b0e17f6f, 2026-11-05 |
| `wire/decode-vs-json-objects` | 1.89 | 314504b486, 2026-11-05 |
| `wire/decode-vs-json-string` | 16.24 | 314504b486, 2026-11-05 |
| `wire/read-vs-json-objects` | 1.93 | 314504b486, 2026-11-05 |
| `wire/read-vs-json-text` | 10.27 | 314504b486, 2026-11-05 |
<!-- /gen -->
