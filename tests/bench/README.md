# Benchmarks

`crates/djs-bench` is the benchmark harness (ARCH.md "Performance Budget" and
"Hill-Climbing Strategy"). Run it through the script, which builds in release mode
and fetches the inputs:

```sh
scripts/bench.sh                    # all benchmarks (about 15 s)
scripts/bench.sh json regexp/scan   # results whose name contains one of these
scripts/bench.sh --record           # append the results to tests/scores/bench.tsv
scripts/bench.sh --compare          # compare with the last recorded run on this machine
scripts/bench.sh --compare abc123   # ... or with the recorded run of a commit
scripts/bench.sh --quick            # every benchmark once with small inputs
```

Each benchmark warms up (at least two calls or 100 ms), then takes samples for about a
second (5 to 50 samples). The median sample is the recorded value; the best sample is
printed for reference. `--compare` shows the change in percent and flags changes beyond
a noise threshold (10% by default, `--threshold`; 0.5% for sizes) as `better` or
`WORSE`, and exits 1 on a regression. The same scenarios run in quick mode as a unit
test (`cargo test -p djs-bench`), so the gate keeps the harness working without timing
anything.

## Tracks

- **Components** (`lex`, `num`, `regexp`, `json`, `gc`): the scenarios the issues that
  built them measured (0003, 0004, 0011, 0012, 0009), so the numbers compare with
  their logs. The lexer input is typescript.js 5.9.3 (9.1 MB, pinned by hash and
  fetched into `vendor/bench` of the primary checkout) plus a generated 2 MB file.
- **Engine** (`engine/<engine>/<script>`): the scripts in this directory, run in the
  codemode model through djs-difftest's `Engine` trait (with `tests/corpus/prelude.js`),
  on dquickts' `qjs` now and on djs once it runs JS, which will add the djs/qjs ratio.
  A sample is one whole run; `empty.js` measures the start-up overhead, which is
  subtracted from the other scripts. Each script starts with `// bench: expect
  <result>`, the value it must return (recorded from qjs), so a wrong result never
  produces a number. Scripts take 20 to 100 ms on qjs.
- **Size** (`size/wasm ...`): `tests/bench/size` is a wasm32 cdylib that calls
  djs-core's main entry points (lexer with rescans, number conversion, regexp
  compile and exec, JSON parse and stringify, heap and GC), built with the `wasm`
  profile (its own workspace, with a copy of the profile). Reported: the file size, the
  size without custom sections (names, producers), gzip -9, and `wasm-opt -Oz` when
  wasm-opt is installed. Until djs-wasm exists this tracks the cost of the code paths,
  not of a shippable module.

## Adding a benchmark

Write a function `fn(r: &mut Runner)` that prepares its input and calls `r.ops(name,
ops_per_call, f)` (ns/op), `r.bytes(name, bytes_per_call, f)` (MB/s), `r.time(...)`
(ms), or `r.value(...)` (sizes) for each result, then add it to `GROUPS` in
`crates/djs-bench/src/lib.rs`. Use `r.scale(full, quick)` for input sizes. For the
engine track, add a script here with its `// bench: expect` line.

## Recorded numbers

`tests/scores/bench.tsv` has one line per run and benchmark: date (UTC), commit
(`-dirty` if the worktree had changes), machine (`$DJS_BENCH_MACHINE` or hardware
model, architecture and CPU count), benchmark, value, unit. Compare runs only on the
same machine; `--compare` picks rows of this machine when there are any.
