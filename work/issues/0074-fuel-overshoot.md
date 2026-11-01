# 0074: Carry fuel overshoot across runs

status: done
agent: determinism
branch: issue/0074-fuel-overshoot
depends-on: -
touches: crates/djs-core/src/interp/, crates/djs-core/src/vm/

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [x] Total fuel doesn't depend on slice size (low priority).
- [x] The criteria in 0055's report for this item.
- [x] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

- `Interp::FUEL_DEBT` (heap). `Exec::run` subtracts it right before the instruction loop
  (runs that return a status without executing anything don't pay or set it), and after
  the run stores the overshoot for OutOfFuel/HostCall and 0 otherwise. `Vm::call` and
  `abort` reset it. `fuel_left` became `fuel_used` (excluding the debt paid) for
  `run_metered`.
- `suspend_in` refunds the re-dispatch: the call_intrinsic op cost in bytecode frames,
  `CALL_FUEL` in native frames (now `pub(super)`).
- Test (vm/tests.rs): a budget of k·60060 given as one run or as slices 30030 / 4620
  (with restores) / 143 / 13 reaches the same canonical hash and backtrace at 8
  checkpoints. Without the debt it fails.
- While building the test, found two more sources (logged in D1740, not fixed here):
  1. fuel that depends on collection points: string append (D1610) and the JSON/
     Array.from workloads differ by slice only because `run` collects at its start.
     With that line commented out they are slice-independent, so num's 0071 fixes them.
  2. regexp scans: one free character per resumed scan (regexp/exec.rs's "one unit per
     character after the first"). Outside `touches`; a follow-up.
- The debt is 0 whenever the VM is quiescent. A first version carried it everywhere, and
  then e2e's plain/suspend image comparison (0072) failed for cases ending in Threw,
  because the debt is in the image. Journals check hashes at idle points too.
- Checks: djs-core lib, e2e (all modes), djs-wasm, djs-cli, node js tests (journals
  replay at slice 97), clippy; difftest builtins+codemode with --determinism; test262
  RegExp exec / String split.

## Follow-ups

- regexp/exec.rs: make scan accounting additive across suspensions (a stopped scan should
  charge its last character, or the matcher should report an overshoot), so regexp work
  is slice-independent too.

## Result

Fuel overshoot carries to the next run (`Interp::FUEL_DEBT`), and a resumed intrinsic is
not charged twice (D1740). A fuel budget now stops at the same point for any slicing,
except collection-dependent costs (fixed by 0071) and regexp scans (follow-up).
