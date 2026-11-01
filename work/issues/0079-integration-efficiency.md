# 0079: Integration and test efficiency (investigation + proposals, then implementation)

status: in-progress
agent: devflow
branch: issue/0079-integration-efficiency
depends-on: 0053
touches: work/issues/0079-integration-efficiency.md (report first); later, after coordinator approval: scripts/gate.sh, scripts/merge-to-main.sh, a gate worktree setup, test runners

## Goal
The owner observes that agents make little progress because the machine is busy with
CPU-bound tasks: too many tests, and integration (rebase → full gate → fast-forward under
one lock, per merge, in each agent's own worktree and target/) may itself be inefficient.
0047 measured the parts; this issue looks at the **whole flow** end to end and proposes
structural changes.

## Questions to answer, with measurements
- Merge throughput: how long does a merge take from `merge-to-main.sh` to main moving
  (lock wait, rebase, build after rebase, each gate step)? How often does a gate fail
  after a rebase (wasted runs), and why? How many merges per hour, and how much machine
  time does each cost? (git log on main has ~160 commits in the last 3 hours.)
- Is building in the agent's worktree after a rebase the main cost? Each agent's
  target/ is behind main, so a rebase onto a moved main recompiles djs-core and
  everything downstream, at gate width. Would a single persistent **gate worktree** with
  a warm target/ (the gate checks out the rebased branch there, so the build is
  incremental against the previous main) be much cheaper? Measure it.
- Gate content: is each step pulling its weight? Today: fmt, clippy native (all
  targets), clippy wasm32 release, the workspace tests (unit, e2e three modes, cross-
  language tests), wasm builds (several), node tests, bgen check, strip-types, difftest
  (plain + suspend 97 + 1009 + gc-stress + determinism), the full test262 active group
  (~38k tests) in debug, native-vs-wasm. Which steps could be (a) skipped by change
  scope (docs-only, builtins-only, js-only changes), (b) run on main after the merge
  instead (a post-merge "nightly"-style check that the coordinator watches, reverting or
  fixing on failure), or (c) sharded or cached (for example test262 results cached per
  test by content hash of the engine build plus the test file, so unchanged results are
  not recomputed)?
- Merge batching: a merge queue or train that gates several ready branches together,
  instead of one full gate per branch?
- Agents' iteration loops: after D012 and 0053, what do agents still run, and what does
  it cost? Are dev builds at opt-level 1 the right trade-off?

## Deliverables
- [ ] Measurements and a ranked list of proposals (expected saving, risk to main's health,
      effort). Send them to the coordinator **before implementing anything that changes
      the gate or the merge script**.
- [ ] Then implement the approved ones.

## Log
- devflow: measured (tools in `work/scratch/0079/`: `lockwatch.sh` samples the merge lock
  every 2 s; `mkgate.py` turns gate.sh into a step-timed copy (wall + children's CPU via
  `times`); `scenario.sh` warms every gate profile at an old commit, then gates a later
  one). Load average 25–245 during all of it (other agents and other projects).
- Throughput: 10 merges/h over the last 1, 3 and 6 h (8.4/h over 12 h). Gaps between merges
  in the last 3 h: min 169 s, p25 224, median 293, p75 406, max 850. Lock watched for 15
  min: held 89 %, three gates of 209–322 s, all merged; usually several agents waiting.
  The lock is saturated: the gate cycle time is the throughput limit.
- Gate on main e1f4d63 with a target 106 commits (2 h) behind ("agent worktree", A) vs a
  target at the previous merge ("gate worktree", B), wall s / CPU s, second run (load
  35–120):

  | step | A | B |
  |---|---|---|
  | clippy | 7.0 / 13.6 | 2.1 / 5.3 |
  | test (incl. build) | 142.5 / 268.8 | 107.6 / 264.3 |
  | clippy wasm32 release | 5.8 / 10.1 | 0.2 / 0.2 |
  | wasm builds | 3.9 / 4.8 | 0.2 / 0.2 |
  | js tests + native-vs-wasm + cross | 7.8 / 11.2 | 22.7 / 16.7 |
  | difftest (stress, determinism) + bytecode | 41.3 / 282.5 | 33.9 / 113.7 |
  | test262 active | 23.8 / 251.3 | 46.9 / 257.6 |
  | total | 233 / 843 | 215 / 659 |

  First run (load 27–245): A 399 s wall, B 187 s + a failure (below). The compile steps
  are ~40 CPU-s and ~20–30 s wall cheaper with a warm gate target; the rest is noise from
  load. The test step itself (~265 CPU-s) is dominated by running tests, not building.
- Spurious gate failure: at load 245, test262 `decodeURI/S15.1.3.1_A2.5_T1` and
  `decodeURIComponent/S15.1.3.2_A2.5_T1` (~3 s each in debug) hit the 20 s wall-clock
  limit. Fixed by D1790 (fuel decides; measured: all active tests pass at 1e9 fuel, only
  those two need more than 2e8; every difftest case needs less than 1e8).
- Conflict hotspots, commits touching the file on main in the last 6 h:
  docs/decisions.md 73, work/BOARD.md 49 (coordinator), builtins/gen/prelude.js 43,
  builtins/gen/resume-points.txt 40, interp/run.rs 34, bytecode/intrinsics.rs 32,
  builtins/intrinsics.d.ts 29, interp/run/intrinsics.rs 28, vm/tests.rs 24, test262
  expected-failures 23, js/src/layout.generated.ts 21, corpus expected-failures 19.
- Ranked proposals sent to the coordinator; 1–4 approved (fuel timeouts, union driver +
  regenerate generated files, persistent gate worktree, nice for agents), 5 (merge train)
  and 6 (post-merge checks) on hold until 1–4 are measured.

## Findings

## Proposals

## Result
