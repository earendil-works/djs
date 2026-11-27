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

### Design (D3720)
- A host's run is snapshotted in *host mode* (`SnapshotOptions::host: Some(HostNames)`), as
  dumps named host values (D2971): host values (namespaces, `values`, converting functions,
  module namespaces) are written by name, and the objects reachable from them through own
  data properties as members (`ALL_TOOLS.0.schema`), never their content; host functions by
  identity wherever they are (also an open call's function: `HostCall.FN` has type `hostfn`);
  the handle table is not written, only the handles the host names (`named`), plus the host's
  data string (`hostData`: call states, fuelUsed, clock).
- Such a snapshot loads only *into the host's own VM* (`Vm::load_snapshot(base, bytes,
  sources, &HostValues)`): a fresh sandbox made with the same options; its host values and
  handles stay its own, the run's state comes from the snapshot (patches over its global,
  Interp state, host calls, realm per-VM values). A plain VM snapshot (handles by index)
  loads only with `from_snapshot`; each refuses the other.
- Why not the whole heap: pi's ALL_TOOLS (every tool's description) and the namespaces with
  their converting closures would be in every snapshot (KBs to tens of KBs), and the
  sandbox's JS-side tables (handles, fn ids) would have to travel too.
- `needs(bytes)`: the scripts stored by hash only (names) and the host names, so the host can
  find sources and make lazily-made module values before the load.
- The Vm record carries `frozenBuiltins` and `weakRefsStrong`; a load into a host VM with
  another mode refuses; from_snapshot sets them (vm_from_snapshot no longer forces frozen).
- FORMAT 2.

### Steps
- WIP commit 76128a88: export/import host mode, `load_snapshot`, `needs`, host_tests.rs.

## Follow-ups

## Result
