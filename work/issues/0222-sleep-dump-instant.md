# 0222: examples/sleep-dump: instant apart from the sleeps, output before each sleep, 1 s sleeps

status: in-progress
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

## Follow-ups

## Result
