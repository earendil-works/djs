# 0073: Replay journal and image hash

status: done
agent: determinism
branch: issue/0073-replay-journal
depends-on: 0067
touches: crates/djs-core/src/vm/, crates/djs-wasm/, js/src/sandbox.ts, scripts/ or a tool crate

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [x] `Vm::canonical_hash()` that doesn't modify the VM, a journal format, a record/replay tool that checks the calls issued.
- [x] The criteria in 0055's report for this item.
- [x] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

- `Vm::canonical_hash`: copies the arena (`Heap::from_bytes_trusted`), clears the popped
  stack slots and collects the copy (shared `collect_heap`, without finalization
  scheduling), then FNV-1a 64. vm test: equal before and after `Vm::collect`, the VM bytes
  unchanged, equal at fuel max/1000 and with restores at 997, different for another
  state. `vm_canonical_hash` (+ wasm test).
- Journals in the Sandbox (D1730): `run/runAsync(code, params, { journal })` records
  (deterministic sandboxes only), `replay(journal)` replays. Entries: run, sync, async,
  check, end. Replay answers from the journal and never calls the host functions,
  delivers one result per idle point in the recorded order with its clock, and checks
  fn/args of every call, the issued count and the canonical hash at the start, at every
  recorded idle point and at the end. The new SandboxError kind `diverged` says where.
- Node test: record run 2 of a sandbox (async tools with real timers, a sync function, an
  error result, the clock, Math.random), replay it in a sandbox restored from a snapshot
  taken before run 2, at fuel slice 97 instead of 1e6, with host functions that throw if
  called: same result. A fresh sandbox (different state), a changed result and a changed
  call all diverge, with the message naming the cause. Recording without
  `deterministic` is a misuse error.
- A code change diverges at the first check (the hash covers the compiled code), before
  any call comparison. That is fine: the run entry carries the code anyway.
- No CLI `djs replay`: the acceptance allowed either, and journals hold host values, which
  live in the Sandbox.
- docs/host.md "Journals", the ABI row, D1730.

## Follow-ups

## Result

`Vm::canonical_hash` / `vm_canonical_hash` / `Sandbox.canonicalHash()`: a state hash
independent of collections and slicing, computed on a copy. The Sandbox records a
deterministic run's journal (`{ journal }` option) and replays it (`replay(journal)`)
with call and hash checks, across snapshots and fuel slices (docs/host.md "Journals",
D1730).
