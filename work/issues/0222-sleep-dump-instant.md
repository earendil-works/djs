# 0222: examples/sleep-dump: instant apart from the sleeps, output before each sleep, 1 s sleeps

status: done
agent: sleepy
branch: issue/0222-sleep-dump-instant
depends-on: -
guide: - (the sleep README; the guide's dumps/JS-interface pages may quote it: tell the coordinator)
touches: examples/sleep-dump/, js/test/sleep-dump.test.ts, scripts/ (only if a shared build helper is needed)

## Goal (owner)
- `examples/sleep-dump/run.sh` (all modes: default, `--restart`, `--node`) must feel instant:
  apart from the sleeps themselves, no noticeable wait. Today every run calls `cargo build` of
  the debug dump module and node compiles a large debug wasm. Measure where the time goes
  (cargo no-op build, node startup, wasm compile/instantiate, prelude, dump/load), then fix it:
  e.g. build the release dump module once and skip cargo when it is up to date (a cheap staleness
  check instead of invoking cargo), cache compiled wasm, avoid repeated work between the two
  processes. Target: the time from start to the first output, and from the end of a sleep to the
  next output, under ~100–200 ms on a warm machine.
- Log something before each sleep starts (the reader must see output immediately, not after the
  first sleep): e.g. the run starting, and each step announcing the sleep before it begins, and the
  host announcing the suspend/resume as it happens.
- The default sleep is 1 s instead of 5 s (README and test updated).
Keep the example small and readable (it is the showcase); update its README (transcript
regenerated) and js/test/sleep-dump.test.ts.

## Log
- Measured (load 60–135 on the machine, so wall times are 2–5x CPU; medians of 5 runs,
  300 ms sleeps). Before, per process: `cargo build` no-op 30–60 ms (more when another
  build holds the target lock), node 40–80 ms, the TS imports ~80 ms, compiling the 63 MB
  debug module 100–130 ms, then create/start/suspend in debug code ~200 ms more. First
  output 773 ms (`run.sh`), first `[vm]` line 618 ms (`--node`). `--restart` was broken:
  process 2 refused the dump (`$djs.source("sleep.js")`: the host had no `sources`).
- With the release dump module (`scripts/build-wasm.sh --dump`, target/djs-dump.wasm,
  2 MB) the first run in a process costs, in CPU: compile ~5 ms, create ~15, start ~15,
  suspend ~22, resume ~35 (warm repeats: ~5 ms all together). Node's compile cache
  (NODE_COMPILE_CACHE) cuts the imports' ~35 ms.
- run.sh: builds target/djs-dump.wasm only when missing or older than a source (`find
  -newer` over the crates it is built from, builtins/, tests/builtins, Cargo.toml/lock:
  15 ms), sets NODE_COMPILE_CACHE=target/node-compile-cache. sleep-dump.ts: default 1000 ms,
  `sources` in `options()` (fixes --restart), host lines "starting", "suspended … and dropped
  the sandbox", "resuming …"/"continuing … in node" as they happen.
- After (same load): first output 142 ms (`run.sh`, "host: starting"; 12–17 ms in the two
  process modes, run.sh's echo), first `[vm]` line 287/153/160 ms (default/--restart/--node),
  0–6 ms from a sleep's end to the next output (in-process and --restart); total minus the
  sleeps 296/161/254 ms, was 792/–/905.
- README: run.sh paragraph, `sources` in the host snippet, transcripts regenerated at 1000 ms.
  js/test/sleep-dump.test.ts: a third test runs the CLI's `start` and `resume` as two
  processes (the --restart path). Guide: docs/guide/host.md said "sleeps five seconds" (one
  phrase changed, outside touches:); no other page quotes the example.
- The Rust copy of the script (vm/dump_size_tests.rs, the_sleep_example_dumps) is unchanged:
  SCRIPT did not change, and it keeps its own 5000.

## Follow-ups

## Result
`examples/sleep-dump/run.sh` (all three modes) runs on target/djs-dump.wasm, built once and
rebuilt only when its sources are newer (no cargo call otherwise), with node's compile cache;
apart from the sleeps a run takes ~150–300 ms on a heavily loaded machine (was ~800–900), and
the host logs each step before the sleep starts. Default sleep 1000 ms. `--restart` works
again (`sources` option). Gap: the first run after a source change waits for a release build
(a minute or two; run.sh says so).
