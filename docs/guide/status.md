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
| tight loop | 5.6G | 15.3G | 0.37 |
| property access (prop1) | 2.2G | 5.4G | 0.41 |
| strings (split, slice, join, ...) | 2.0G | 5.8G | 0.34 |
| arrays | 7.7G | 16.3G | 0.47 |
| JSON round trips | 1.0G | 2.0G | 0.51 |
| JSON with Dates | 0.8G | 1.7G | 0.46 |
| for-of | 4.1G | 7.0G | 0.59 |
| calls | 2.7G | 3.9G | 0.68 |
| push/pop and string methods | 3.1G | 2.9G | 1.08 |
| an agent-like script | 5.1G | 6.9G | 0.73 |
| forEach | 4.5G | 5.8G | 0.77 |
| map/filter/reduce (higher-order) | 6.3G | 4.5G | 1.40 |

Machine instructions per workload (`scripts/perf/w/`), measured at 669ce55099,
2026-11-05; lower is better.
<!-- /gen -->

## Size

<!-- gen:status size -->
`djs_wasm.wasm` is 953 KB (414 KB gzip, 350 KB brotli): 806 KB of code, 98 KB of data,
and the base snapshot (the initialized realm with all builtins, 186 KB) coded to 46 KB
in a custom section. The slim build is 825 KB (349 KB gzip); the dump module, the
shipped build with the dumper, 1,835 KB (754 KB gzip). Module sizes measured at
31c4587e31, 2026-11-05.
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
| `bounds/gzip` | 7599 | 31c4587e31, 2026-11-05 |
| `bounds/instructions-max-pct` | 19.6 | 31c4587e31, 2026-11-05 |
| `bounds/instructions-median-pct` | 12.1 | 31c4587e31, 2026-11-05 |
| `compile/typescript-lazy-time-pct` | 56 | 35ed5ab789, 2026-11-05 |
| `dump/bytes` | 1835457 | 31c4587e31, 2026-11-05 |
| `dump/extra-gzip` | 339443 | 31c4587e31, 2026-11-05 |
| `dump/gzip` | 753819 | 31c4587e31, 2026-11-05 |
| `dumps/corpus-cases` | 300 | 7167411f0f, 2026-11-05 |
| `dumps/idle-points` | 1063 | 7167411f0f, 2026-11-05 |
| `dumps/log-raw` | 1758604 | 7167411f0f, 2026-11-05 |
| `dumps/log-zstd` | 402669 | 7167411f0f, 2026-11-05 |
| `dumps/raw-p50` | 2573 | 7167411f0f, 2026-11-05 |
| `dumps/raw-p99` | 12378 | 7167411f0f, 2026-11-05 |
| `dumps/refused` | 0 | 7167411f0f, 2026-11-05 |
| `dumps/resume-ms-p50` | 0.391 | 7167411f0f, 2026-11-05 |
| `dumps/resume-ms-p99` | 2.464 | 7167411f0f, 2026-11-05 |
| `dumps/write-ms-p50` | 0.258 | 7167411f0f, 2026-11-05 |
| `dumps/write-ms-p99` | 1.733 | 7167411f0f, 2026-11-05 |
| `dumps/zstd-p50` | 1006 | 7167411f0f, 2026-11-05 |
| `dumps/zstd-p99` | 2962 | 7167411f0f, 2026-11-05 |
| `fuel/slice-us` | 13.0 | 7167411f0f, 2026-11-05 |
| `heap/append-10m-memory` | 40501248 | 7167411f0f, 2026-11-05 |
| `heap/json5/dictionary` | 152.0 | 7167411f0f, 2026-11-05 |
| `heap/json5/shape` | 104.0 | 7167411f0f, 2026-11-05 |
| `heap/literal4/dictionary` | 112.0 | 7167411f0f, 2026-11-05 |
| `heap/literal4/shape` | 72.0 | 7167411f0f, 2026-11-05 |
| `heap/sandbox-memory` | 655360 | 7167411f0f, 2026-11-05 |
| `heap/string-10mib-memory` | 14024704 | 7167411f0f, 2026-11-05 |
| `heap/string-10mib-staged-memory` | 35127296 | 7167411f0f, 2026-11-05 |
| `native/arrays-x` | 3.74 | 35ed5ab789, 2026-11-05 |
| `native/foreach-x` | 2.1 | 35ed5ab789, 2026-11-05 |
| `slim/base` | 135056 | 31c4587e31, 2026-11-05 |
| `slim/bytes` | 825264 | 31c4587e31, 2026-11-05 |
| `slim/gzip` | 348662 | 31c4587e31, 2026-11-05 |
| `speed/arrays/djs-wasm` | 7734.4 | 669ce55099, 2026-11-05 |
| `speed/arrays/qjs-wasi` | 16344.9 | 669ce55099, 2026-11-05 |
| `speed/calls/djs-wasm` | 2672.8 | 669ce55099, 2026-11-05 |
| `speed/calls/qjs-wasi` | 3905.6 | 669ce55099, 2026-11-05 |
| `speed/foreach/djs-wasm` | 4453.3 | 669ce55099, 2026-11-05 |
| `speed/foreach/qjs-wasi` | 5786.3 | 669ce55099, 2026-11-05 |
| `speed/forof/djs-wasm` | 4118.4 | 669ce55099, 2026-11-05 |
| `speed/forof/qjs-wasi` | 6950.7 | 669ce55099, 2026-11-05 |
| `speed/higher-order/djs-wasm` | 6347.7 | 669ce55099, 2026-11-05 |
| `speed/higher-order/qjs-wasi` | 4536.8 | 669ce55099, 2026-11-05 |
| `speed/json-date/djs-wasm` | 793.0 | 669ce55099, 2026-11-05 |
| `speed/json-date/qjs-wasi` | 1719.5 | 669ce55099, 2026-11-05 |
| `speed/json/djs-wasm` | 1042.6 | 669ce55099, 2026-11-05 |
| `speed/json/qjs-wasi` | 2031.2 | 669ce55099, 2026-11-05 |
| `speed/prop1/djs-wasm` | 2213.4 | 669ce55099, 2026-11-05 |
| `speed/prop1/qjs-wasi` | 5398.3 | 669ce55099, 2026-11-05 |
| `speed/push-pop/djs-wasm` | 3100.7 | 669ce55099, 2026-11-05 |
| `speed/push-pop/qjs-wasi` | 2867.8 | 669ce55099, 2026-11-05 |
| `speed/strings/djs-wasm` | 1958.8 | 669ce55099, 2026-11-05 |
| `speed/strings/qjs-wasi` | 5779.2 | 669ce55099, 2026-11-05 |
| `speed/tight/djs-wasm` | 5644.4 | 669ce55099, 2026-11-05 |
| `speed/tight/qjs-wasi` | 15331.2 | 669ce55099, 2026-11-05 |
| `startup/instantiate-us` | 40.7 | 7167411f0f, 2026-11-05 |
| `startup/sandbox-run-us` | 141.8 | 7167411f0f, 2026-11-05 |
| `startup/sandbox-us` | 112.7 | 7167411f0f, 2026-11-05 |
| `startup/vm-new-us` | 2.5 | 7167411f0f, 2026-11-05 |
| `wasm/base-coded` | 45685 | 31c4587e31, 2026-11-05 |
| `wasm/brotli` | 350406 | 31c4587e31, 2026-11-05 |
| `wasm/bytes` | 953410 | 31c4587e31, 2026-11-05 |
| `wasm/code` | 805917 | 31c4587e31, 2026-11-05 |
| `wasm/data` | 97925 | 31c4587e31, 2026-11-05 |
| `wasm/exports` | 63 | 31c4587e31, 2026-11-05 |
| `wasm/gzip` | 414376 | 31c4587e31, 2026-11-05 |
| `wire/decode-vs-json-objects` | 2.06 | 84b37a08b0, 2026-11-05 |
| `wire/decode-vs-json-string` | 15.7 | 84b37a08b0, 2026-11-05 |
| `wire/read-vs-json-objects` | 1.56 | 84b37a08b0, 2026-11-05 |
| `wire/read-vs-json-text` | 9.3 | 84b37a08b0, 2026-11-05 |
<!-- /gen -->
