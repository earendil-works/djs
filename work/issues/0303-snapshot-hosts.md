# 0303: Snapshots for hosts: Sandbox API, CLI, REPL, docs-site pauses, pi's setup

status: in-progress
agent: psnap
branch: issue/0303-snapshot-hosts
depends-on: 0301
touches: js/src/sandbox.ts, js/test/, crates/djs-cli, crates/djs-docs (live examples' pauses), docs/guide (js-api, cli, host), examples/
guide: docs/guide/js-api.md, docs/guide/cli.md, docs/guide/host.md, docs/guide/pi.md

## Goal
Hosts use snapshots instead of dumps.

## Acceptance criteria
- [ ] `Run.snapshot({ sources?: false })` -> bytes, `Sandbox.loadSnapshot(bytes, { sources })`
      -> Run with its open calls (same ids, states as dumps carried them), tasks'
      `suspend()`/`checkpoint()` and `Sandbox.resume` on snapshots; sources by hash with a
      host lookup; `describe(bytes)` for people.
- [ ] **pi's setup through snapshots**: the Sandbox's namespaces, aliases, missingMember
      proxies, jsonBoundary converting functions, `values`, module namespaces (0292), RunExit,
      fuelUsed across a load, run as in pi's code mode; harvest cases with that setup.
- [ ] CLI: `djs snapshot` (or `run --snapshot-after K`), `djs resume`, `djs describe`;
      the REPL's `.snapshot`/`.restore` (no `.dump` once 0304 lands).
- [ ] The docs site's `debugger;` pauses continue from a snapshot; `debugger` no longer
      depends on the `dump` feature.
- [ ] examples/sleep-dump becomes a snapshot example (one process suspends, another resumes).
- [ ] Guide pages updated (a new persistence page may wait for 0304).

## Log

## Follow-ups

## Result
