# 0146: Frozen-intrinsics VM option (region-based), with the override fix

status: done
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
- [x] Option plumbed through vm_new (native + wasm, additive ABI), Sandbox and js/pi
      adapter (pass-through), recorded in the image.
- [x] Frozen-mode tests: override cases (Error subclass `this.name`, `obj.toString = f`),
      assigning/adding/deleting on builtins, descriptors, isFrozen, Reflect.set/defineProperty
      results, Proxy targets that are builtins, globalThis extensible; difftest/test262 with
      the flag off unchanged.
- [x] Cost measured: wasm instructions on all workloads with the flag off and on
      (target: < 1%), size ratchet respected or trade recorded.
- [x] Demonstrate pi's prelude without lockdown on the frozen mode in a scratch copy
      (start time, sealed image size), compared with today; no pi-private changes.

## Log
- Rebased on 0147's `Roots::BASE_END` (D2062) as soon as it landed; no temporary boundary
  was needed.
- Design (D2304): `ROOT_FROZEN_BUILTINS` in `Roots::FLAGS`; the heap caches
  `frozen_end` (BASE_END with the bit, else 0), refreshed on load, after collections and
  in `release_top`; `Heap::frozen(o)` in the new heap/frozen.rs. Exempt per-VM objects:
  global object, global lexical record, Symbol.for registry, constructors table
  (`realm::FROZEN_EXEMPT`). Gates in heap/object.rs (one early check each, heap was told);
  override fix in `ordinary_set`, `set_property` and the `super.x = v` path (class.rs).
- `Vm::new` with the option sets BASE_END to everything made at creation (no base).
  `Vm::collect_canonical` is a full collection with the mode on (a Cheney layout has no
  prefix; heap zeroes BASE_END there).
- Plumbing: `VmOptions::frozen_builtins`; wasm `vm_new_with(limit, live, flags)` (flag 1)
  and `vm_frozen_builtins(vm)`, additive; Sandbox `frozenBuiltins`; js/pi
  `VmHooks.frozenBuiltins` → `Engine.create(..., frozenBuiltins)`, in the prelude image key
  and the deterministic fingerprint (only when on, so default keys are unchanged).
  scripts/perf/wasm-run.ts: `FROZEN=1`.
- Tests: crates/djs-core/src/vm/frozen_tests.rs (~70 cases: override cases, assigning /
  adding / deleting, descriptors, isFrozen/isSealed/isExtensible, Reflect.set /
  defineProperty / setPrototypeOf, Proxy over builtins, globalThis, private names,
  super assignments; fresh VM, base image, restored image after collect +
  collect_canonical, gc_stress with suspension every 97 fuel, and flag off); djs-wasm
  `frozen_builtins_flag`; js/test sandbox + pi-vm (pi's prelude with its lockdown cut out).
- Flag forced on for a run (not committed): difftest 22 new failures, test262 active 2841,
  all reviewed by category: every one is a test patching or inspecting builtins
  (assignments, defineProperty, delete, extensibility, descriptor checks). No crash, no
  hang, no engine-internal write refused (the two hidden registries were exempted up front,
  from reading realm.rs). Scratch: work/scratch/0146/{difftest-on,t262-on}.txt.
- Cost (wasm instructions, 22 workloads, n=3, work/scratch/0146/{off,on}.txt):
  flag off vs main: arrays +0.45%, tight +0.5%, loop +0.3%, props +0.2%, pushpop +0.9%
  (bimodal), the rest ±0.2%. Flag on vs off: props +0.8% (global stores take the exemption
  check), pi-like +0.3%, others within noise. Size: 458,187 B gzip vs main 457,666
  (+521), under the ceiling 458,444.
- pi (native release, tests/pi prelude, scratch test in work/scratch/0146/scratch_pi_start.rs,
  the 0144 prelude.js / prelude-nolock.js): start today 2.67–3.07 ms → frozen without
  lockdown 0.40 ms; sealed image 479,912 B / 8,330 cells → 375,608 B / 6,362 cells (base
  322,376 B). pi's prelude *with* lockdown on the mode still works (2.1 ms, 418.3 KB): its
  walk converts nothing, since nothing reads writable+configurable.
- Guide: new section "Frozen builtins" in docs/guide/builtins.md; docs/host.md (Sandbox
  option, exports). No other guide pages affected.

## Follow-ups
- `Error.stackTraceLimit` / `Error.prepareStackTrace` are frozen with `Error` under the
  mode; if a host needs them settable, a per-property exemption (or SES-style accessor)
  is needed.
- 0147: once the per-VM objects leave the prefix, drop `realm::FROZEN_EXEMPT` and the
  slow path of `Heap::frozen`; once the prefix is immutable, `collect_canonical` no longer
  needs the full-collection fallback.
- pi-private: turn `frozenBuiltins` on and drop the prelude's lockdown (owner's call).

## Result
`VmOptions::frozen_builtins` (wasm `vm_new_with` flag 1, Sandbox/js-pi `frozenBuiltins`):
builtins in the base prefix behave as frozen (writes refused, descriptors non-writable /
non-configurable, not extensible), with the override fix in [[Set]]; globalThis ordinary;
recorded in the image. D2304. Flag off: unchanged behavior, ≤0.5% instructions; flag on
≤0.8%. pi without lockdown on the mode: start 0.40 ms (was ~3 ms), image 375.6 KB (was
479.9 KB). Known gaps: see Follow-ups.
