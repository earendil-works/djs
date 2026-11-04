# Work tracking

All work is tracked in files in this directory, committed to git.

- `BOARD.md`: index of all issues by status, with dependencies and who works on them.
  Owned by the coordinator.
- `issues/NNNN-slug.md`: one file per issue. The assigned agent updates its status, log
  and result.
- `scratch/`: gitignored scratch space, one subdirectory per issue (`scratch/0002/`).
  Also holds `scratch/merge.lock` while main moves and `scratch/gate.lock` while a gate
  runs.

## Issue file format

```markdown
# NNNN: Title

status: open | in-progress | blocked | done
agent: <sub-agent name or ->
branch: issue/NNNN-slug
depends-on: NNNN, NNNN
touches: paths/or/areas, that/this/issue/may/change
guide: docs/guide pages this issue may affect, or -

## Goal
What and why, with links into ARCH.md / TESTING.md.

## Acceptance criteria
- [ ] concrete, checkable items

## Notes
Hints, pointers, constraints from the coordinator.

## Log
Appended by the agent while working (dated entries).

## Follow-ups
Work discovered but not done here.

## Result
Filled in when done: what landed, how to use it, known gaps.
```

## Lifecycle

1. The coordinator writes the issue (`status: open`) and adds it to the board.
2. The coordinator creates the branch `issue/NNNN-slug` from `main` and assigns an agent.
   Each agent has one permanent worktree, `.worktrees/<agent>`, which the coordinator
   switches to the issue's branch, so its warm `target/` is reused across issues (D012).
   `.worktrees/gate` belongs to merge-to-main (D1792).
3. The agent works, commits on the branch and merges with `scripts/merge-to-main.sh`
   (rebase, regenerate, gate, fast-forward).
4. The coordinator reviews the result on `main`, updates the board and deletes the
   merged branch (`git branch -d`).

## Parallel work

Issues run in parallel only when their `touches:` areas are disjoint and neither depends
on unmerged work of the other. When in doubt, run them one after another.

## Merging and generated files

`scripts/merge-to-main.sh` rebases the branch onto `main` and regenerates the generated
files (D1791), runs the gate on the resulting commit in the shared gate worktree
`.worktrees/gate` (D1792) under the gate lock (`work/scratch/gate.lock`), then takes the
merge lock (`work/scratch/merge.lock`) only to fast-forward `main` to that commit (D2666).
If `main` moved while it gated, it rebases and gates again; legs the move did not touch
come from the gate's cache (D2662), so a moved-by-docs `main` costs seconds. The third
attempt holds the merge lock throughout, so no branch starves. It says what it is doing:
"waiting for the gate worktree", "gating", "main moved while gating; rebasing and gating
again", "fast-forwarding". The gate worktree is detached, belongs to merge-to-main (never
work in it, never remove it: its warm `target/` is the point), and is created on first
use.

- `docs/decisions.md`, `builtins/intrinsics.d.ts` and the two ratchet files
  (`tests/test262/expected-failures.txt`, `tests/corpus/expected-failures.txt`) merge
  with git's `union` driver (`.gitattributes`): both sides' lines are kept, so appends by
  different issues never conflict. A stale or duplicate ratchet line is harmless (the gate
  reports stale entries; the next `--update` rewrites the file).
- Generated files (`builtins/gen/prelude.js`, `builtins/gen/resume-points.txt`,
  `js/src/layout.generated.ts`, and the `INTRINSICS` table in
  `crates/djs-core/src/bytecode/intrinsics.rs`) merge through `scripts/merge-driver.py`,
  which keeps one side; `scripts/regen.sh` then rewrites them from the merged sources
  (bgen, `scripts/gen-intrinsics.py`, and the layout test with `DJS_UPDATE_LAYOUT=1`), and
  merge-to-main commits the result as `NNNN: regenerate generated files after the rebase
  onto main`. A conflict in intrinsics.rs outside the table is left for you, as before.
  `tests/smap/summary.txt` is such a generated file too (regen.sh rewrites it through
  `pinned_maps`), and `tests/scores/dump-size.tsv` merges row by row (TESTING.md
  "Derived files that merge by themselves", 0214).
- The drivers are registered in the repository's git config by
  `scripts/setup-merge-drivers.sh` (merge-to-main runs it). Run it once before rebasing
  by hand; after a manual rebase, run `scripts/regen.sh` too.
- Always run `scripts/regen.sh` after a manual `git rebase main` (or a merge), before
  you build, test or commit: the driver kept one side of each generated file, so until
  then the prelude, the INTRINSICS table or the layout can lack your branch's changes
  (or main's) while everything still compiles. Commit what it changes. (0090: a branch
  commit carried a stale prelude without `%TypedArraySetElementSlow` until the next
  regen.)
- Conflicts in sources (Rust, `builtins/*.ts`, docs other than decisions.md) still need
  resolving by hand: resolve, `git rebase --continue`, run merge-to-main again.
- merge-to-main fails right away if the primary checkout has uncommitted changes (the
  coordinator edits there only under the lock).
- Every merge is logged in `work/scratch/` of the primary checkout: `merge-log.tsv`
  (lock waits, rebase, gate, total, outcome, merge lock held, attempts), `gate-steps.tsv` (each gate step's wall and CPU
  seconds) and `slow-tests.tsv` (the gate's 10 slowest unit tests and test262 tests).
  `scripts/merge-stats.sh [HOURS]` summarizes them per hour. The gate prints its slowest
  tests too (via `scripts/test-runner.sh`), so a slow new test shows in its own merge.
