# 0019: Benchmark harness and tracked numbers

status: done
agent: infra
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

2026-11-01 (infra)
- Read how 0003/0004/0009/0011/0012 measured (ignored `*_throughput`/`bench_*` tests in
  djs-core). djs-bench ports their scenarios unchanged (same inputs, patterns, payloads
  and result names), so numbers compare with their logs. One deliberate difference: JSON
  benchmarks collect (untimed) before each sample instead of letting the heap grow
  across iterations. The lexer uses the parser-like token driver from djs-core's tests
  (copied: it is test code there).
- Harness (D1190): no dependencies; warmup, ~1 s of samples, median recorded, best
  shown; `Runner::{ops, bytes, time, value}`; a benchmark is one function added to
  `GROUPS`. Quick mode runs every scenario once on small inputs as a unit test (0.05 s),
  so the gate covers the harness without timing anything. No gate.sh change.
- Engine track (D1191): `tests/bench/*.js` (10 scripts modeled on dquickts'
  microbench.js plus codemode-style map/filter/JSON work) run through djs-difftest's
  `Engine` trait and prelude, so 0016's djs engine plugs in unchanged. `empty.js` (qjs
  start-up, ~1.8 ms) is subtracted. Every script declares its expected result, recorded
  from qjs; my first guesses were wrong, which the check caught.
- Size track (D1192): `tests/bench/size` (own workspace, copy of `[profile.wasm]`):
  222,858 bytes, 180,610 without custom sections, 85,190 gzip -9. No wasm-opt installed.
- Lexer input: typescript 5.9.3 from the npm registry, pinned by sha256 in
  `scripts/bench.sh` (the tarball's sha512 matches npm's published integrity), into
  `vendor/bench` of the primary checkout.
- Full run: about 15 s. Numbers (M-series, while other agents were building): lexer
  ~650 MB/s, to_string 6/29/31 ns, string_to_number 5/6/22 ns, regexp scans
  34-840 MB/s, JSON parse 490 MB/s (records) and 1290 MB/s (string), stringify 890 and
  2200 MB/s, GC 0.22 and 0.29 ms/MiB: all in line with the earlier logs.
- Noise: back-to-back runs varied by 5-10%, and by up to 20% while other agents
  compiled. Hence a 10% default threshold, and `--compare` re-runs anything that looks
  worse once and keeps the better value before flagging it. Compare on an idle machine.

## Follow-ups
- 0018 (wire) landed in parallel or is still in progress: add `wire/decode ...` as one
  more group function once its API is on main.
- Once djs runs JS: add a djs `Engine` (from 0016) to `engine::engine` and report the
  djs/qjs ratio per script; also a compile-throughput benchmark (MB/s of source) and
  `vm_new` start-up once those APIs exist (ARCH performance budget).
- When djs-wasm exists, point the size track at it (and install wasm-opt on bench
  machines).

## Result
- `crates/djs-bench` and `scripts/bench.sh [--record] [--compare [REV]] [--quick]
  [filter...]`; documentation in `tests/bench/README.md`.
- Tracks: components (lex, num, regexp, json, gc), engine (`tests/bench/*.js` on qjs),
  size (`tests/bench/size` wasm probe).
- `tests/scores/bench.tsv` with the first baseline, recorded on main after the merge.
- Decisions D1190 to D1192.
- Known gaps: no wire benchmark yet (0018); the engine track has only qjs; no wasm-opt
  numbers.
