# 0069: Running out of host memory is not a JS out-of-memory error

status: done
agent: heap
branch: issue/0069-host-oom
depends-on: -
touches: crates/djs-core/src/heap/ (arena growth, GC to-space), crates/djs-core/src/vm/, crates/djs-wasm/, js/src/sandbox.ts

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [ ] `HeapError::HostOutOfMemory` for arena growth and GC to-space failures. The run stops with its own uncatchable status, never a catchable error or a silently skipped collection.
- [ ] The criteria in 0055's report for this item.
- [ ] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

- D1690: `HeapError::HostOutOfMemory` from arena growth (`Arena::reserve`) and the GC
  to-space (`Arena::with_capacity`, `fail_next_gc_reserve`). In the interpreter a host
  OOM fault, or a failed safepoint collection (which used to be ignored), stops the run at
  the next instruction boundary with `Status::HostOutOfMemory`; `Vm::run`/`run_metered`
  abort the call. The compiler and `eval` keep the distinction (`CompileErrorKind::
  HostOutOfMemory`). djs-wasm status 5; Sandbox kind `memory`; pi: crash. docs/host.md:
  the status, the error kind and the cap guidance for wasm32 (≤ ~1 GiB). Tests (vm/tests.rs):
  `host_memory_failure_is_not_a_js_error` (growth refused inside a try: the catch does
  not run, the VM is reusable) and `a_refused_to_space_stops_the_run` (gc stress; also
  `Vm::collect`). No JS test: forcing a wasm memory.grow failure needs a module with a
  small maximum memory.
- Rebased onto 0063: one match arm added in interp/run/native.rs's test (infra's file,
  outside `touches`); native frames' faults already go through `fault_value`.

## Follow-ups

## Result

A catchable out-of-memory now always means the cap was reached. Host allocator failures
end the run with `Status::HostOutOfMemory` / wasm status 5 / Sandbox `memory`, never a JS
error and never a skipped collection (D1690). Callers of `Status` gained one arm.
