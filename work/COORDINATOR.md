# Coordinator checklist

Notes for the coordinator (the main agent), learned the hard way.

## Assigning an issue
1. Write the issue file, update BOARD.md, and commit with
   `scripts/with-merge-lock.sh git commit ...`.
2. Per D012 each agent has one permanent worktree, `.worktrees/<agent>`. Switch it to the
   new branch (`git -C .worktrees/<agent> switch -c issue/NNNN-slug main`, with the
   worktree clean and the old branch merged), or create it once with `git worktree add`.
   Do this **in its own tool call** and wait for it to finish.
3. `subagent configure` with the **absolute** cwd. Never in the same parallel batch as
   step 2: the configure call races the worktree creation (this happened three times).
4. Only then `subagent send` the assignment.

## When an agent reports done
- Check `git log main`, `git worktree remove`, `git branch -d`, update the board (scores
  table too), and commit.
- Relay follow-ups that belong to other agents' areas.

## Health checks (do this whenever an agent has been quiet for a while)
- `subagent status`: an agent that is **idle** but hasn't reported may have lost its
  final message (it happened with 0049). Read it with `subagent answer <name>`.
- `ps -Ao pid,etime,command | grep -v grep | grep -i "qjs\|djs\|cargo"`: anything
  running for more than ~15 minutes is probably a hang (it happened twice: a bgen
  behavior test hung for 1h45m, a qjs probe for 4h15m). Kill it and tell the agent.
- `git -C .worktrees/<x> log --oneline main..HEAD` plus `status --short`: an agent with
  no commits for a long time needs a nudge ("commit often").

## Parallelism
- Parallel only when `touches:` are disjoint or the overlap is small, additive and
  announced to both agents.
- Shared hot files: `bytecode/intrinsics.rs` (regenerate, never hand-merge),
  `heap/cells.rs` and `heap/object.rs` (class/kind numbers: coordinate), `interp/run.rs`,
  `docs/decisions.md` (append-only; conflicts are trivial).
