# 0139: Decode/inject the base image straight into the VM's arena

status: done
agent: heap
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
- [x] Per-instance wasm memory measured before/after (pi warm/cold, an empty Sandbox,
      mem-perturb for the workloads); vm_new and first-instance times not worse.
- [x] Multiple VMs per instance still work (they decode/copy from wherever the base
      lives; if the instance must keep a copy for a second VM, keep it lazily).
- [x] Verification of injected bytes (length + hash) kept; tests for decode-into-arena,
      inject-into-arena and corrupted injection.
- [x] ABI additive; size ratchet respected (or trade recorded).

## Log
- djs-wasm:
  - the instance's base is an owned `Vec`; the first `vm_new` takes it as its arena,
    with no copy, growing in place (`Vm::from_base_vec`, a small vm/ addition;
    `djs_prelude::new_vm_owned`);
  - without a base, it decodes straight into a buffer with the arena's capacity
    (`djs_lzrc::decode_into`, a small additive change outside touches);
  - after that, a later VM or `djs_base_ptr` decodes once more and the instance keeps it
    for further VMs.
- Hosts: `rememberBase` before `vm_new`, and `provideBase` only where a VM is created
  (no longer in pi's restore instances).
- Tests:
  - native: inject into the arena, the corrupted path, second and third VMs, the kept
    copy;
  - wasm (js/test/base.test.ts): the arena's address equals the base's, then a second
    VM and the base read again.
- Measured (D2060):
  - wasm memory −0.31 MiB everywhere: an empty Sandbox, pi warm/cold, all 22 workloads
    in their 24-run means;
  - first `Sandbox.create` unchanged, later ones ~20 µs faster, `vm_new` (multi-VM)
    unchanged;
  - +157 B gzip, under the ceiling.
- Docs: docs/host.md (export table, the coded-base section), guide host.md.

## Follow-ups

## Result
- The base is decoded or injected straight into the first VM's arena (D2060): −0.31 MiB
  of wasm memory per Sandbox and pi session.
  - Multi-VM instances decode twice, then copy.
  - Verification, ABI and timings are kept; pi warm starts are faster (0.52 → 0.43 ms).
- `djs_base_ptr`'s pointer now lasts until the next `vm_new`. Hosts read the base before
  `vm_new`; old hosts still work, at the old memory cost.
