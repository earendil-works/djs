# 0020: Flaky djs-difftest `qjs_comparisons` test under load

status: done
agent: infra
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

2026-11-01 (infra)
- Code: djs-difftest's `qjs_comparisons` and djs-test262's `fixture_suite_with_qjs` ran
  every qjs case with a 1 s limit (4 and 8 workers), including the deliberately
  hanging `for (;;) {}` case. Any normal case that went past 1 s became a Fail.
- Reproduction attempts (no failure in any of them):
  - 15 rounds of both tests with 54 busy loops (3 per core);
  - 20 rounds with four concurrent clean `cargo build --release` plus 36 busy loops
    (load average 33);
  - qjs latency under that load: median 3.9 ms, p99 5.2 ms, max 7.3 ms.
  So CPU contention alone cannot explain it.
- Second hypothesis checked: on macOS, std creates pipes with `pipe()` and only then sets
  close-on-exec, and `posix_spawn` runs without `POSIX_SPAWN_CLOEXEC_DEFAULT`. So a child
  spawned by another thread in that window can inherit a pipe's write end. Then a normal
  case's stdout EOF would wait for the hanging qjs, until its 1 s kill. A targeted
  stress test (16 threads of short processes with piped stdout against 8 threads
  spawning `sleep 1` with piped stdout, 16,000 runs, with and without load) never hit it:
  the window is too narrow in practice.
- Root cause, then: a run stalled for more than 1 s while four agents built and tested
  at once (most likely memory pressure). The margin was 1 s against a
  few-millisecond run, and the hanging case shared the same short limit. I could not
  reproduce that stall on demand; the fix covers both explanations anyway.
- Fix:
  - Normal cases get 60 s and 4 workers. Only the hanging case is short (0.5 s): a
    `// difftest: timeout 0.5` directive in difftest, and a separate run after the
    others in the test262 fixture, so it can never delay another case.
  - Timeouts now read `timeout: no result within Ns` in both runners; in difftest a
    timeout or crash is no longer shown as a transcript line difference.
  - The test262 fixture's bound on the kill delay went from 3 s to 30 s.
  - Both tests got faster (1 s to 0.5 s).
  - The convention is recorded as D1200.
- Other subprocess tests: djs-bench's quick smoke test already uses 60 s, and djs-bgen's
  behavior test has no timeout. No other short timeouts in the workspace.
- Gate under load: three full `scripts/gate.sh` runs (now including 0018's node tests)
  while two endless clean release builds and 36 busy loops ran (load average 37-50):
  all OK, 27-42 s.

## Follow-ups
- Optional hardening: the engines detect completion by stdout EOF. A leaked pipe end (the
  macOS race above) would delay a result by at most the other child's lifetime, which
  only matters close to a case's limit. If it ever shows up, wait for the child's exit
  and drain the pipe non-blockingly instead.

## Result
- djs-difftest and djs-test262 runner tests: 60 s limits and 4 workers for normal cases.
  The deliberately hanging case runs with its own 0.5 s limit and cannot affect others.
- Both runners report timeouts as `timeout: no result within Ns` (difftest no longer
  shows them as a diff line).
- D1200 records the convention for subprocess-based tests.
- Verified with three gate runs under heavy load.
