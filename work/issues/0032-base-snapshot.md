# 0032: Base snapshot (prelude precompiled at build time)

status: done
agent: num
branch: issue/0032-base-snapshot
depends-on: 0028
touches: crates/djs-prelude/ (new), crates/djs-core/src/vm/ (additive: create from a base image, build hash checks), crates/djs-wasm/, crates/djs-cli/ (use the base), crates/djs-bench/ (startup benchmarks), Cargo.toml (workspace member), docs/host.md

## Goal
ARCH.md "G2" and "Snapshots" → "Base snapshot": the initialized realm with the installed
prelude is produced at build time and embedded, and `vm_new` copies it into a fresh
arena. That copy *is* startup. Today `Vm::new` compiles `builtins/gen/prelude.js` on
every call: 92 µs native and 143 µs in wasm (num, 0026 profile), already over the
100 µs target, and growing with every builtin wave (0031 is adding many).

## Acceptance criteria
- [x] `crates/djs-prelude`: a build.rs that uses djs-core (as a build-dependency) to
      create a VM with the prelude, run a collect, and write the heap image; embedded
      with `include_bytes!`. The image is deterministic: the same inputs give identical
      bytes (check it in a test, because the GC is deterministic, D1090).
- [x] The build hash: the image header records `build_hash(LAYOUT_HASH, OPS_HASH,
      INTRINSICS_HASH)` (the 0013/0027 follow-ups: put it in `Roots::VERSION` or
      similar), and `Vm::from_bytes` rejects a mismatch with a clear error.
- [x] `Vm::new_from_base(bytes)` or `VmOptions::base`: no compile at startup. The
      default embedders (djs-wasm, djs-cli, difftest, test262, e2e tests) use the base.
      Keep the compile-at-startup path (`VmOptions::prelude`) for tests and for
      bootstrapping.
- [x] Per-VM state that must not be shared or baked in (`Math.random` seed (0031),
      host function bindings, handles) is initialized after the copy. Document what
      lives in the base.
- [x] Startup numbers: `vm_new` native and in wasm, before and after, in
      tests/scores/bench.tsv (add a `startup` benchmark if 0026 hasn't). Target: well
      under 100 µs in wasm; report the image size too (it ships in the wasm binary:
      check the size track).
- [x] The gate stays green, and the ratchets are unchanged (the same behavior from base
      and from compile).

## Notes
- 0031 (infra) is growing the prelude right now, so the build must regenerate
  automatically when `builtins/gen/prelude.js` or djs-core changes (cargo's
  `rerun-if-changed` plus the dependency on djs-core).
- Incremental (delta) snapshots against this base are P4. Keep the format compatible
  (ARCH "Deltas": the base region as an old generation later).

## Log
- **Build hash** (D1320): a new `Roots::BUILD` field (layout.generated.ts regenerated);
  `vm::BUILD_HASH` combines the layout, ops and intrinsics hashes. `Vm::new` writes it;
  `Vm::from_bytes` and `Vm::from_base` check it first and fail with `BuildMismatch`.
  `Roots::VERSION` stays the layout hash (the JS reader checks it; putting the build hash
  there would churn layout.generated.ts with every new op or intrinsic). `LoadError` has
  `Display`.
- **`Vm::from_base(bytes, opts)`** (vm/, additive): a copy plus header and build-hash
  checks through a new `Heap::from_bytes_trusted` (heap/mod.rs, small and additive;
  `from_bytes` = trusted load + verify). It requires an idle VM with no handles, and calls
  an `after_copy` hook for per-VM state (none yet). Measured: from_bytes with verify 8 µs,
  plain copy 0.6 µs (36 KB image).
- **`crates/djs-prelude`**: build.rs (host djs-core: `Vm::new`, collect, full verification,
  `$OUT_DIR/base.img`), `BASE` and `new_vm(opts)`. Tests: deterministic (two builds are
  identical and equal the embedded image), full verification, limits. Touching
  prelude.js reruns the build script (checked with -vv).
- **Embedders**: djs-wasm `vm_new`, djs-cli, difftest, test262 and the e2e tests use
  `djs_prelude::new_vm`. djs-core dev-depends on djs-prelude, a dev-dependency cycle that
  cargo allows for integration tests. `Vm::new` stays for unit tests and for the image.
- **djs-wasm**: `vm_new_compiled` behind the `compiled-prelude` feature (the gate's debug
  build enables it). `js/test/base.test.ts` checks that the host-built base equals the
  realm compiled inside wasm32: byte-identical. Without the feature, release modules
  drop the realm-building code: 564 KB vs 578 KB on main, with the base included.
- **Startup**:
  - native: 74 µs → 0.48 µs (`startup/` group, which also records the base size);
  - wasm: 144 µs → 0.40 µs (`wasm/vm_new`, now timed in batches, plus `wasm/vm_new
    compiling the prelude`).

  Recorded in tests/scores/bench.tsv. After rebasing onto 0031 part 1: base 72 KB,
  compile 216 µs, copy 0.77 µs natively.
- **Outside touches**:
  - `heap/cells.rs` (`Roots::BUILD`), `heap/mod.rs` (`from_bytes_trusted`, `LoadError`);
  - `js/src/layout.generated.ts` (regenerated);
  - difftest/test262 (one line each, as the criteria ask);
  - `scripts/gate.sh` (the feature flag);
  - `js/test/base.test.ts`, `js/bench/*.ts`.

## Follow-ups
- Instantiating the wasm module (~35 µs) is now most of a sandbox's startup. Options:
  several VMs per instance in the Sandbox (instances are per sandbox only for trap
  isolation), or a smaller initial memory.
- 0031: put the `Math.random` seed (and any other per-VM state) in `Vm::after_copy`, so it
  is not baked into the base.
- Wasm builds compile djs-core twice (host for build.rs, wasm32 for the module). Fine
  today; watch build times.
- P4 deltas: the base is a plain image, so ARCH's "base region as an old generation" can
  build on it. Nothing done here.

## Result
VMs start from a base snapshot made at build time:
- `djs_prelude::new_vm(opts)`, or `Vm::from_base(djs_prelude::BASE, opts)`;
- djs-wasm `vm_new`, djs-cli, difftest, test262 and the e2e tests all use it;
- `Vm::new` remains the compile path, for unit tests and for making the base.

Images carry a build hash (`Roots::BUILD`) and are rejected with a clear error on a
mismatch.

Startup: 0.5 µs native and 0.4 µs in wasm (from 74 and 144 µs). The base is 36 KB (72 KB
with 0031 part 1). The release module got smaller (564 KB vs 578 KB).

Decisions D1320-D1321. docs/host.md updated.
