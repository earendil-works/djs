# 0053: Dev-flow test fixes (from 0047)

status: open
agent: -
branch: issue/0053-devflow-test-fixes
depends-on: 0047
touches: crates/djs-difftest/, crates/djs-test262/ (job count from DJS_JOBS, the qjs result cache), crates/djs-core/tests/e2e.rs, the two slow djs-core unit tests, tests/corpus/ (no-stress headers), scripts/gate.sh (only if needed for the above)

## Goal
Implement devflow's proposals 2, 3, 4 and 6 from work/issues/0047-devflow.md (1, 5 and 7
are adopted as D012). Measure the gate before and after.

## Acceptance criteria
- [ ] difftest: a `// difftest: no-stress` directive. Apply it to the six cases that only
      time out under the stress modes (array-methods, base64, proxy, test_map, test_rope,
      other-errors), so the compared set is the same on every run. The stress runs of
      all other cases stay hard failures on divergence.
- [ ] e2e: parallel cases, a per-case fuel cap (a hung case fails in seconds), and a
      smaller `iteration_runtime.js`.
- [ ] The two slow djs-core unit tests: limit fuel-1 suspension to the first N stops, as
      e2e does.
- [ ] The test262/difftest runners default `-j` to `DJS_JOBS` when set (the repo's
      `.cargo/config.toml` sets 6; the gate sets full width).
- [ ] A qjs result cache keyed by the qjs binary's hash and the test262 pin, so qjs runs
      don't repeat.
- [ ] Gate wall/CPU time before and after on a quiet machine, recorded in the Log.

## Log

## Follow-ups

## Result
