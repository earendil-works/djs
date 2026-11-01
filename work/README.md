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
   (rebase, regenerate, gate, fast-forward).
4. The coordinator reviews the result on `main`, updates the board and removes the
   worktree (`git worktree remove`, `git branch -d`).

## Parallel work

Issues run in parallel only when their `touches:` areas are disjoint and neither depends
on unmerged work of the other. When in doubt, run them one after another.

## Merging and generated files

`scripts/merge-to-main.sh` takes the merge lock, rebases the branch onto `main`,
regenerates the generated files, runs the gate and fast-forwards `main` (D1791):

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
- The drivers are registered in the repository's git config by
  `scripts/setup-merge-drivers.sh` (merge-to-main runs it). Run it once before rebasing
  by hand; after a manual rebase, run `scripts/regen.sh` too.
- Conflicts in sources (Rust, `builtins/*.ts`, docs other than decisions.md) still need
  resolving by hand: resolve, `git rebase --continue`, run merge-to-main again.
- merge-to-main fails right away if the primary checkout has uncommitted changes (the
  coordinator edits there only under the lock).
