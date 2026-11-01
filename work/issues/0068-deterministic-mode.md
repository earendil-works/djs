# 0068: Deterministic mode for VM and Sandbox

status: done
agent: determinism
branch: issue/0068-deterministic-mode
depends-on: 0067
touches: crates/djs-core/src/vm/, crates/djs-core/src/heap/ (weak refs: strong or absent; no FR scheduling), crates/djs-wasm/, js/src/sandbox.ts, js/pi/, docs/host.md

## Goal
From the determinism investigation (0055): see work/issues/0055-determinism-investigation.md,
"Proposed issues", for the full goal and acceptance criteria. Experiment tools are in
scripts/determinism/.

## Acceptance criteria
- [x] A VM option: weak refs absent or strong, no finalization scheduling, `Vm::collect` doesn't clear weak refs. Sandbox `deterministic: {startedAt, seed}`: the clock advances only with results, time zone 0. A `randomSeed` option (default: entropy).
- [x] The criteria in 0055's report for this item.
- [x] Ratchets unchanged or better, stress modes identical, decisions recorded.

## Log

- Core: `WeakRefs { Collected, Strong, Absent }`, `VmOptions::weak_refs` (applied by
  `Vm::new` and `from_base`), and `Vm::set_weak_refs`. Strong mode is a `Roots::FLAGS`
  bit; heap/weakref.rs allocates WeakRef pairs and registry target tables as ValueArrays
  (same layout as EphemeronArray, checked by a const assert), so they are traced. That
  covers "no finalization scheduling" and "`Vm::collect` doesn't clear weak refs" without
  special cases: nothing dies. Absent deletes the two globals.
- Tests: vm tests (strong mode: deref after explicit collects and under gc-stress, the
  mode survives images, new refs after restore; collected mode for contrast; absent via
  `Vm::new` and `from_base`); djs-wasm `vm_set_weak_refs` for all modes plus a bad mode;
  test262 WeakRef/FinalizationRegistry 74/74 unchanged.
- Sandbox: `randomSeed` (default entropy, a behavior change: every Sandbox used to start
  from the VM's fixed default seed), `weakRefs`, and `deterministic: {startedAt, seed}`
  (seed, tz 0, strong weak refs, the clock moving only with async results via
  `vm_resolve_host_call_at`, the clock in the snapshot meta). Node test: identical
  results at fuel slices 1e6/1000/97, the clock values as specified, seeds repeat, the
  default seeds differ, absent mode.
- pi adapter: `engine.setWeakRefs(2)` before the host globals in deterministic runs,
  replacing 0059's JS `delete` (HOST_GLOBALS lost its parameter, so the fingerprint's host
  hash changes). pi's suite: 221/226 as before, both "without WeakRef" tests pass.
- docs/host.md (options, ABI table, pi rows), D1680.

## Follow-ups

## Result

`VmOptions::weak_refs` / `vm_set_weak_refs`: collected, strong (no collection observable:
targets live in traced tables) or absent (globals deleted). The mode is heap state. The
Sandbox has `randomSeed` (default entropy), `weakRefs` and `deterministic: {startedAt,
seed}`, where the clock advances only with async results (D1670's per-result clock),
the time zone is 0 and weak refs are strong. The pi adapter uses the VM option (D1680).
