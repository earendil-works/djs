# 0146: Frozen-intrinsics VM option (region-based), with the override fix

status: in-progress
agent: perf
branch: issue/0146-frozen-intrinsics
depends-on: 0144, 0147 (BASE_END root, landed first by heap)
guide: docs/guide/ (a new section on the frozen mode in builtins.md or determinism.md)
touches: crates/djs-core/src/interp/ and builtins/ property write paths ([[Set]], [[DefineOwnProperty]], [[Delete]], [[SetPrototypeOf]], [[PreventExtensions]], descriptor reporting, Object.isFrozen/isSealed/isExtensible, Reflect.*), crates/djs-core/src/vm/ (the option), crates/djs-wasm/ (vm_new option, additive ABI), js/src/sandbox.ts and js/pi/ (pass-through option only; NO pi-private changes), tests/ (a frozen-mode suite); heap (0147) is in heap/ and vm/ image code

## Goal
Owner decision (after docs/frozen-builtins.md): frozen builtins as a **runtime flag**, not a
build profile. A VM option at creation (like deterministic mode, stored in the image):
every builtin object in the base region (the immutable prefix, `Roots::BASE_END` from
0147; NOT OLD_END, which covers pi's sealed prelude too) behaves as frozen:
- JS-visible writes to such objects are refused (TypeError in strict code, silent in sloppy);
- **override fix**: an assignment whose target property is found on a frozen builtin holder
  that is not the receiver creates an own data property on the receiver (SES /
  Node `--frozen-intrinsics` semantics), as pi's lockdown emulates today;
- descriptors report writable:false / configurable:false, isFrozen/isSealed true,
  isExtensible false for those objects;
- globalThis stays extensible and its non-builtin properties writable (explicit exception;
  until 0147 moves per-VM state out, any other per-VM objects in the base region too).
No per-object freezing, no dictionary conversion, one base image for both modes.

## Acceptance criteria
- [ ] Option plumbed through vm_new (native + wasm, additive ABI), Sandbox and js/pi
      adapter (pass-through), recorded in the image.
- [ ] Frozen-mode tests: override cases (Error subclass `this.name`, `obj.toString = f`),
      assigning/adding/deleting on builtins, descriptors, isFrozen, Reflect.set/defineProperty
      results, Proxy targets that are builtins, globalThis extensible; difftest/test262 with
      the flag off unchanged.
- [ ] Cost measured: wasm instructions on all workloads with the flag off and on
      (target: < 1%), size ratchet respected or trade recorded.
- [ ] Demonstrate pi's prelude without lockdown on the frozen mode in a scratch copy
      (start time, sealed image size), compared with today; no pi-private changes.

## Log

## Follow-ups

## Result
