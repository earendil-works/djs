# Work tracking

All work is tracked in files in this directory, committed to git.

- `BOARD.md`: index of all issues by status, with dependencies and who works on them.
  Owned by the coordinator.
- `issues/NNNN-slug.md`: one file per issue. The assigned agent updates its status, log
  and result.
- `scratch/`: gitignored scratch space, one subdirectory per issue (`scratch/0002/`).
  Also holds `scratch/merge.lock` while a merge is in progress.

## Issue file format

```markdown
# NNNN: Title

status: open | in-progress | blocked | done
agent: <sub-agent name or ->
branch: issue/NNNN-slug
depends-on: NNNN, NNNN
touches: paths/or/areas, that/this/issue/may/change

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
2. The coordinator creates a worktree, `git worktree add .worktrees/NNNN-slug -b
   issue/NNNN-slug main`, and assigns an agent.
3. The agent works, commits on the branch and merges with `scripts/merge-to-main.sh`
   (rebase, gate, fast-forward).
4. The coordinator reviews the result on `main`, updates the board and removes the
   worktree (`git worktree remove`, `git branch -d`).

## Parallel work

Issues run in parallel only when their `touches:` areas are disjoint and neither depends
on unmerged work of the other. When in doubt, run them one after another.
