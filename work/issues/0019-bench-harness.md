# 0019: Benchmark harness and tracked numbers

status: open
agent: -
branch: issue/0019-bench-harness
depends-on: 0001
touches: crates/djs-bench/, tests/bench/, scripts/bench.sh, tests/scores/

## Goal
ARCH.md "Performance Budget" and "Hill-Climbing Strategy": separate tracks hill-climb on
benchmarks (compile time, startup, run time, wasm size, data passing), each with its own
tracked numbers. So far every agent measured ad hoc in their issue logs. Make it one
reproducible harness with a recorded history, so that regressions are visible and the
perf track has a baseline.

## Acceptance criteria
- [ ] `crates/djs-bench` (std, binary `djs-bench`, `cargo run --release -p djs-bench`):
      a small benchmark runner (warmup, N iterations, median and min, ns/op or MB/s; no
      criterion dependency unless it is clearly worth it). It covers what exists today:
      lexer throughput (typescript.js-sized input, fetched into vendor/ by the script, or
      a generated large JS file), number format/parse, regex compile and scan (the
      patterns from 0011's log), JSON parse/stringify (0012's payloads), GC per MB live
      (0009's scenarios), and the wire decoder once 0018 lands (structure it so adding a
      benchmark is one function).
- [ ] An engine track placeholder: once djs runs JS (0014/0015), the same harness runs
      `tests/bench/*.js` microbenchmarks on djs and on `~/Development/dquickts/qjs` and
      reports the ratio. Prepare the structure and a few benchmark scripts now (taken
      from or modeled on dquickts' `tests/microbench.js`, in the codemode model), and
      run them on qjs only for now.
- [ ] A size track: the size of a release wasm32 build of djs-core's code paths. There is
      no djs-wasm crate yet, so measure a tiny cdylib inside djs-bench's build or under
      tests/bench/ that references the main entry points (lexer, num, regexp, json, heap,
      gc), built with the `wasm` profile. Report raw and (if `wasm-opt` is available,
      currently not) optimized sizes. Document it.
- [ ] `tests/scores/bench.tsv`: one line per run and benchmark (date, commit, machine id,
      benchmark, value, unit), appended by `scripts/bench.sh --record`. Plus
      `scripts/bench.sh --compare [REV]`, which shows the change against the last
      recorded run (or a given commit) with a ±% column and flags changes beyond a noise
      threshold.
- [ ] Record a first baseline run on main.
- [ ] Not in the gate (too slow, too noisy). A `--quick` mode that runs everything once
      as a smoke test can go in the gate if it takes under 5 seconds.

## Notes
- Read the Log sections of 0003, 0004, 0009, 0011 and 0012 for how they measured, and
  keep the numbers comparable.
- 0018 (wire) is in progress in parallel. Add its benchmark after it lands, or leave a
  follow-up.

## Log

## Follow-ups

## Result
