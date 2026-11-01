# 0067: Async host results as macrotasks (the host inbox)

status: done
agent: determinism
branch: issue/0067-host-inbox
depends-on: -
touches: crates/djs-core/src/vm/, crates/djs-core/src/interp/ (job queue / run loop), crates/djs-core/src/heap/ (an inbox root, additive), crates/djs-wasm/, js/src/sandbox.ts, docs/host.md, docs/compiler.md 7.5

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [x] `resolve_host_call` appends to a heap inbox. `run` applies one result only when the job queue is empty, drains it, then takes the next. Unhandled rejections are checked after each result. Optional `resolve_host_call_at` carries a clock value. `async_interleave.js` and `sandbox_interleave.ts` (scripts/determinism) give the same output for any slice size and delivery timing, batched delivery behaves like separate deliveries, and an image with queued results restores. Amends D1374/D1370 (record a decision).
- [x] The criteria in 0055's report for this item.
- [x] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

- Kept inside the agreed areas: `Interp` gets three fields (REPORTED, INBOX,
  INBOX_HEAD; layout and base image regenerate, js/src/layout.generated.ts updated). The
  inbox lives in interp/run/promise.rs (next to the job queue). In interp/run.rs only
  `next_job` changed: a checkpoint plus `inbox_apply` when the queue is empty. vm/mod.rs:
  `resolve_host_call` → inbox, `resolve_host_call_at`, `abort` clears the inbox. The
  call/frame and property paths are untouched.
- `enqueue_host_settle` is gone; `inbox_apply` reserves the job slot before it takes the
  entry, so OOM keeps the result.
- Rejections: `Interp::REPORTED` marks the prefix of `REJECTIONS` that was unhandled at a
  checkpoint. JS handlers no longer withdraw those; the embedder's `mark_handled` still
  does (D1423 unchanged). `take_unhandled_rejections` still returns everything tracked.
- Tests (vm/tests.rs): every case under eager / one-at-idle / batch delivery × fuel
  max/1000/37 × with/without restore, which must match: async_interleave (b1 b2 a),
  macrotask order with microtasks between results, rejection checkpoints, per-result
  clock; results waiting in an image; abort drops the inbox. djs-wasm:
  `vm_resolve_host_call_at`. Node (sandbox.test.ts): the Sandbox prints the same at
  `fuelSlice` 1e6/1000/50 for sync- and async-answering tools (before: differed at 1000).
- The Sandbox itself needed no change. The existing async tests (vm, wasm, sandbox,
  pi-vm) pass unchanged, since they deliver at IDLE one at a time, where the semantics
  are identical.
- Checks: djs-core all tests (e2e in three modes), djs-wasm, node js/test (42), clippy;
  test262 Promise/await/async-function filtered (no change), difftest ratchet OK.
- ARCH "Host Interface" still says "Resolving one enqueues the reaction jobs". Proposed
  edit for the coordinator: "Resolving one puts the result into the VM's inbox; it is
  applied as a macrotask once the job queue is empty (D1670)."

## Follow-ups

## Result

Async host results are macrotasks (D1670): `resolve_host_call` puts them into a heap
inbox, and `run` applies them one at a time whenever the job queue runs empty, draining
it between results, with a rejection checkpoint at every empty queue.
`resolve_host_call_at` / `vm_resolve_host_call_at` give each result its own clock. What
a script does now depends only on the order of results, not on fuel slices, delivery
timing, batching or snapshots in between (tested in core, wasm and the Sandbox).
