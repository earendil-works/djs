# 0032: Base snapshot (prelude precompiled at build time)

status: open
agent: -
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
- [ ] `crates/djs-prelude`: a build.rs that uses djs-core (as a build-dependency) to
      create a VM with the prelude, run a collect, and write the heap image; embedded
      with `include_bytes!`. The image is deterministic: the same inputs give identical
      bytes (check it in a test, because the GC is deterministic, D1090).
- [ ] The build hash: the image header records `build_hash(LAYOUT_HASH, OPS_HASH,
      INTRINSICS_HASH)` (the 0013/0027 follow-ups: put it in `Roots::VERSION` or
      similar), and `Vm::from_bytes` rejects a mismatch with a clear error.
- [ ] `Vm::new_from_base(bytes)` or `VmOptions::base`: no compile at startup. The
      default embedders (djs-wasm, djs-cli, difftest, test262, e2e tests) use the base.
      Keep the compile-at-startup path (`VmOptions::prelude`) for tests and for
      bootstrapping.
- [ ] Per-VM state that must not be shared or baked in (`Math.random` seed (0031),
      host function bindings, handles) is initialized after the copy. Document what
      lives in the base.
- [ ] Startup numbers: `vm_new` native and in wasm, before and after, in
      tests/scores/bench.tsv (add a `startup` benchmark if 0026 hasn't). Target: well
      under 100 µs in wasm; report the image size too (it ships in the wasm binary:
      check the size track).
- [ ] The gate stays green, and the ratchets are unchanged (the same behavior from base
      and from compile).

## Notes
- 0031 (infra) is growing the prelude right now, so the build must regenerate
  automatically when `builtins/gen/prelude.js` or djs-core changes (cargo's
  `rerun-if-changed` plus the dependency on djs-core).
- Incremental (delta) snapshots against this base are P4. Keep the format compatible
  (ARCH "Deltas": the base region as an old generation later).

## Log

## Follow-ups

## Result
