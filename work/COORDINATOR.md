# Coordinator checklist

Notes for the coordinator (the main agent), learned the hard way.

## Editing main
- Make edits **inside** the merge lock, not before taking it:
  `scripts/with-merge-lock.sh bash -c '<edit>; git add -A; git commit -qm ...'`. Editing
  first and then waiting for the lock leaves the primary checkout dirty while an agent's
  merge holds the lock, and its fast-forward fails after a full gate (happened with D016).

## Assigning an issue
0. Every issue gets a `guide:` line (pages under docs/guide it may affect, or `-`) and
   **a decision ID range for the agent** (forgetting this made docs and infra both use
   D2030).
1. Write the issue file, update BOARD.md, and commit with
   `scripts/with-merge-lock.sh git commit ...`.
2. Per D012 each agent has one permanent worktree, `.worktrees/<agent>`. Switch it to the
   new branch (`git -C .worktrees/<agent> switch -c issue/NNNN-slug main`, with the
   worktree clean and the old branch merged), or create it once with `git worktree add`.
   Do this **in its own tool call** and wait for it to finish. Check `git status --short`
   first: `git switch` carries uncommitted edits into the new branch.
3. `subagent configure` with the **absolute** cwd. Never in the same parallel batch as
   step 2: the configure call races the worktree creation (this happened three times).
4. Only then `subagent send` the assignment, **in a separate tool call, never in the same
   parallel batch as the branch switch**: the agent reacts immediately and creates the
   branch itself (happened twice with infra).

## When an agent reports done
- **Never remove `.worktrees/gate`**: merge-to-main's persistent gate worktree with a warm
  target/ (D1792).
- Before removing a worktree: `subagent status <name>` must say **idle** (a working agent
  may already have started the next task in it), `git status --short` must be clean, and
  the branch must be merged (`git branch -d`, never `-D`; recover with the commit hash if
  needed).
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

## Pushing
- The owner added `origin` (github.com/earendil-works/djs). The coordinator may push
  `main` (optional; e.g. after a batch of merges): `git push origin main`, under the merge
  lock. Sub-agents never push (AGENTS.md).

## Docs
- Owner rule (0115): the guide is self-contained. No references to ARCH.md, TESTING.md,
  docs/*.md or work/ as explanations, no bare goal numbers ("G2"); decisions are
  explained in words and linked only to a page inside the site; source links go to the
  published GitHub source. `djs-docs check` enforces it once 0115 lands.
- The owner chose djs-docs (0103, option A). When writing an issue, name the guide pages
  (docs/guide/) it affects, and check at merge time that the agent updated them and that
  `djs-docs check` passes in the gate.
