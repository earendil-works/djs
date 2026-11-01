# 0020: Flaky djs-difftest `qjs_comparisons` test under load

status: open
agent: -
branch: issue/0020-flaky-difftest-timeout
depends-on: 0010
touches: crates/djs-difftest/, crates/djs-test262/ (the same pattern, if present)

## Goal
The 0018 merge saw `djs-difftest`'s `qjs_comparisons` runner test fail once, while it
passes alone and in repeated full runs. The suspicion is the 1 s per-case qjs timeout
under machine load (four agents compiling in parallel). The gate must not be flaky.

## Acceptance criteria
- [ ] Root cause confirmed (reproduce under load, for example with parallel `cargo build`
      jobs, or reason it out from the code).
- [ ] Fix: tests that shell out to qjs use generous timeouts (or none, apart from
      genuine hang protection) and bounded parallelism. A timeout is reported as a
      distinct verdict with a clear message. Check djs-test262's fixture suite for the
      same issue.
- [ ] The gate's test steps hold up under load (run the gate while a parallel build
      hammers the CPU, a few times).

## Log

## Follow-ups

## Result
