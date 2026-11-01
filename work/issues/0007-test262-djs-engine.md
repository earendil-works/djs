# 0007: Run test262 on djs and put the ratchet in the gate

status: in-progress
agent: infra
branch: issue/0007-test262-djs-engine
depends-on: 0002, 0022
touches: crates/djs-test262/, tests/test262/, scripts/gate.sh, crates/djs-core/src/vm/ (only additive hooks needed by $262, coordinated in the Log)

## Goal
Plug djs into the test262 runner as an in-process engine (Script mode, D004), check in
`tests/test262/expected-failures.txt` and make the ratchet part of the gate. First
milestone: "the harness loads" (TESTING.md). Like 0016, the deliverables are the
plumbing, the ratchet and a clustered failure report.

## Acceptance criteria
- [ ] A `djs` engine for djs-test262 behind the existing `Engine` trait. A fresh `Vm` per
      test. The harness files and the test compiled as Scripts (the harness can be
      compiled once per distinct include set and reused through the heap image, if that
      saves time: optional). Negative tests by phase: a parse failure must be a
      `CompileError` of the right type; resolution-phase tests are treated as parse;
      runtime negatives check the thrown error's constructor name. Async tests via
      `print` and the `doneprintHandle.js` protocol (`$DONE`): they will mostly fail
      until promises exist; that is fine.
- [ ] `$262`: `global`, `evalScript(src)` (compile and run a Script in the same realm,
      returning the completion value if the compiler provides it, a 0014 follow-up;
      otherwise undefined, logged), `detachArrayBuffer` (throws for now), `gc`
      (calls collect at the next safepoint, or a no-op), and `print`. Implemented as host
      functions; anything needing new VM hooks goes through the coordinator.
- [ ] A fuel limit and a memory limit per test (failures are `timeout` / `oom`).
- [ ] `tests/test262/expected-failures.txt` for the active group, and the ratchet step in
      `scripts/gate.sh`. Gate time: measure it. If the full active set (~20k tests) is
      too slow for the gate in debug, build the runner in release for the gate step, or
      run a fixed deterministic subset in the gate and the full set in a separate
      `scripts/test262.sh`. Record the decision.
- [ ] A Log entry with pass counts (active, later), per top-level directory, and the
      biggest failure clusters (first error line), like 0016. In particular: does the
      harness load (assert.js, sta.js, propertyHelper.js, compareArray.js)? If not, what
      blocks it?
- [ ] AGENTS.md workflow note: agents whose changes make tests pass run the ratchet
      updates (`djs-difftest --engine djs --update-ratchet`, the test262 `--update`) on
      their branch before merging. A stale ratchet hides regressions.

## Log

## Follow-ups

## Result
