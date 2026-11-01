# 0139: Decode/inject the base image straight into the VM's arena

status: open
agent: -
branch: issue/0139-base-into-arena
depends-on: 0122, 0138
guide: docs/guide/host.md / arena.md if they describe base loading
touches: crates/djs-wasm/ (base decode/inject exports, vm_new path), js/src/snapshot.ts (provideBase/rememberBase), js/src/sandbox.ts, js/pi/engine.ts (only the call sites), crates/djs-core/src/vm/ (vm_new from a caller-provided arena, if needed)

## Goal
heap's 0138 §8: each wasm instance keeps a decoded copy of the base image for its whole
life, but with one VM per instance (pi, Sandbox) it is used once, to copy into the arena.
Decode (first instance) or inject (later instances, D2252) directly into the new VM's
arena instead, and drop the extra copy: ~0.31 MiB less wasm memory per instance, no
interpreter/collector/image-format change.

## Acceptance criteria
- [ ] Per-instance wasm memory measured before/after (pi warm/cold, an empty Sandbox,
      mem-perturb for the workloads); vm_new and first-instance times not worse.
- [ ] Multiple VMs per instance still work (they decode/copy from wherever the base
      lives; if the instance must keep a copy for a second VM, keep it lazily).
- [ ] Verification of injected bytes (length + hash) kept; tests for decode-into-arena,
      inject-into-arena and corrupted injection.
- [ ] ABI additive; size ratchet respected (or trade recorded).

## Log

## Follow-ups

## Result
