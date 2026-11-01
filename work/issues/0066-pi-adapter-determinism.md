# 0066: pi adapter determinism fixes

status: done
agent: determinism
branch: issue/0066-pi-adapter-determinism
depends-on: -
touches: js/pi/, js/test/, docs/host.md

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [x] Cold, cache-filling and warm starts give the same interrupt poll count, and the same timeout outcome at a budget between the old counts (a js/test case). The fingerprint includes a hash of the adapter's injected HOST_GLOBALS code. docs/host.md: the stale WeakRef row and the interrupt-budget notes fixed.
- [x] The criteria in 0055's report for this item.
- [x] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

- The WeakRef part (deleting WeakRef/FinalizationRegistry in deterministic runs, images
  keyed by the mode) landed with 0059 already.
- js/pi/vm.ts: `Session.polling`; `prepare` turns it off while the host globals and
  pi's prelude run, so OUT_OF_FUEL there does not call `shouldInterrupt`.
  `scripts/determinism/pi_polls.ts`: before, 676 cold / 676 cache-filling / 540 warm;
  after, 540 / 540 / 540.
- Fingerprint: `host <fnv1a(HOST_GLOBALS)>` after the prelude hash (the js test's regex
  updated).
- js/test/pi-vm.test.ts "a budget means the same on cold and warm starts": the same poll
  count for all three kinds of start, and at budget = polls all three finish while at
  polls - 1 all three are interrupted.
- docs/host.md: the polling and fingerprint rows, the stale "no WeakRef at all" row, the
  interrupt-budget note, and the pi test numbers.
- Coordinator asked about pi's eval test: with 0058 it passes, and so does "rejects
  dynamic import". `node js/pi/pi-tests.ts`: 221/226. The 5 failures are the known
  inherent ones: the fingerprint regex (×2) and strict-mode "ignores patches to
  built-ins" (×3). docs/host.md's table updated.
- D1660 recorded. No engine changes, so ratchets and stress modes are unaffected.

## Follow-ups

## Result

Deterministic pi runs now use the same interrupt polls whether the prelude ran or its
image was reused (D1660), so a budget gives the same outcome for an original warm run
and a cold replay. The deterministic fingerprint includes a hash of the adapter's
`HOST_GLOBALS`. docs/host.md is current (pi's suite on djs: 221/226; eval and dynamic
import pass since 0058). Known gap: a stop request that arrives while the prelude runs
(about 10 ms) waits until the prelude has finished.
