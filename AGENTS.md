# AGENTS.md

Instructions for every agent working in this repository: the coordinator and the
sub-agents that implement issues.

## What this is

djs is a small JavaScript + TypeScript engine in `no_std` Rust, built mainly for wasm and
meant to replace QuickJS in pi's codemode. Read these before you start any work:

1. `ARCH.md`: the architecture. It is the design reference. Do not silently deviate from
   it. If the design needs to change, say so in your issue log and propose an edit.
2. `TESTING.md`: how we test: test262, the differential oracle and the stress modes.
3. `docs/decisions.md`: decisions made during implementation. Append to it, never
   rewrite it.
4. `work/README.md`: the work tracking process (issues, worktrees, merging).
5. Your issue file in `work/issues/`.

## Oracle

- `~/Development/dquickts` is the reference engine and is **read-only**. Never build,
  modify or `git` anything in it. Use the existing binaries:
  `~/Development/dquickts/qjs` and `~/Development/dquickts/run-test262`.
- Its tests (`tests/test_*.js`, `tests/test_typescript.ts`) and its `test262.conf` /
  `test262_errors.txt` are useful reference material.

## Code conventions

- Rust edition 2024, stable toolchain. `cargo fmt` and clippy with `-D warnings` must be
  clean.
- `djs-core` is `#![no_std]` + `alloc`. It must always build for
  `wasm32-unknown-unknown`. No `std`, no platform dependencies.
- Dependencies: none in `djs-core` without a recorded decision in `docs/decisions.md`.
  Std tool crates (`djs-cli`, `djs-test262`, `djs-bgen`) may use small, well-known crates
  where they save real work, but still keep them minimal. Dev-dependencies for tests are
  fine.
- No `unsafe` in `djs-core` unless a decision records why. Heap access is bounds checked.
- Prefer plain, direct code. Comments explain *why*. Keep modules focused; no speculative
  abstraction.
- Every feature comes with tests. Unit tests sit next to the code; JS behavior tests go in
  `tests/` (see TESTING.md).

## Workflow (sub-agents)

You work on exactly one issue at a time, in your own git worktree under `.worktrees/`, on
the branch `issue/NNNN-slug`. The coordinator creates the worktree for you.

1. Read the issue. Set `status: in-progress` in its header and commit that.
2. Work in small steps and **commit often** (`git commit` on your branch). Message format:
   `NNNN: short imperative summary`, plus a body when the reason is not obvious.
3. Keep the issue file's `## Log` section current: what you did, what you found, open
   questions. This file is how context survives between agents and sessions.
4. If you make a design decision that others must know about, append it to
   `docs/decisions.md` on your branch (format given in that file).
5. If you discover follow-up work, list it under `## Follow-ups` in your issue. Do not
   create new issue files; the coordinator does that.
6. When the acceptance criteria are met:
   - set `status: done` in the issue header and fill in `## Result` (what landed, how to
     use it, known gaps),
   - commit,
   - run `scripts/merge-to-main.sh` from your worktree. It takes the merge lock, rebases
     onto `main`, runs `scripts/gate.sh` and fast-forwards `main`. If the rebase conflicts,
     resolve it and run the script again. If the gate fails, fix the failure; never weaken
     the gate to get green.
7. Report back to the coordinator with a short summary: what landed, the commit range,
   follow-ups, anything surprising.

Rules:

- Never commit directly on `main` and never push anywhere. `main` must always pass the
  gate.
- Do not edit `work/BOARD.md` (the coordinator owns it) or other agents' issue files.
- Put scratch files, logs and large outputs in `work/scratch/<issue-id>/`, which is
  gitignored. External checkouts such as test262 go in `vendor/` **of the primary checkout** (the
  repo root, not your worktree), so that all worktrees share one copy; it is gitignored.
- If you are blocked or an issue turns out to be much bigger than described, stop and tell
  the coordinator instead of expanding the scope on your own.
- Stay inside the areas listed in your issue's `touches:` field. If you need to change
  something outside them, say so in the log and keep the change minimal: other agents may
  be working there in parallel.

## Coordinator

The coordinator (the main agent) owns `work/BOARD.md`, creates issues and worktrees,
assigns agents, reviews merged work and keeps ARCH/TESTING/decisions coherent. It works
directly on `main` only for coordination files (issues, board, docs) and commits right
away, so `main` is never left dirty. It commits through `scripts/with-merge-lock.sh`, so
that main never moves while an agent is between rebase and fast-forward.
