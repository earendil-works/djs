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

## Findings

## Proposals

## Result
