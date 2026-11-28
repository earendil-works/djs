# 0305: Remove lazy compilation and the dead code the dump removal left

status: open
agent: pclean
branch: issue/0305-remove-lazy
depends-on: -
touches: crates/djs-core (compiler, interp, vm, heap, snapshot's lazy parts, Cargo.toml features), crates/djs-difftest, crates/djs-test262, crates/djs-bench, crates/djs-prelude (registry ids), crates/djs-wasm (features, exports), js/src/sandbox.ts and abi.ts (the ops that go), scripts/gate.py, scripts/determinism, tests/snapshot/frame-bytecode.txt (regenerated if needed), TESTING.md, docs/decisions.md
guide: docs/guide/compiler.md (Lazy functions), docs/guide/testing.md

## Goal
The owner's "clear wins" after continuation dumps went (0304), from the survey in
work/scratch/simplify/survey.json (primary checkout):
- **The `lazy` feature** (lazy function compilation): no product uses it (the shipped and docs
  modules, djs-cli and djs-embed compile eagerly, D3732); it is in djs-core's default
  features, so every native test runs per-function constant pools the shipped module never
  has (D2991), it doubles the test262, snapshots and full gc-stress legs, and needs a second
  djs-core build for djs-abidrive. Delete it: compiler/lazy.rs, interp/run/lazy.rs,
  vm/lazy_tests.rs, the cfg sites, Op::Lazy, compile_lazy_templates and seal's pass, the
  snapshot module's lazy-template paths (D3721's lazy part), the `--lazy`/DJS_LAZY flags and
  features of difftest/test262/bench, the gate dimension, work/research/0218.
- **Dead leftovers**: the parser's class-span recorder (no caller since 0304); the test-only
  `fast_step`/`fast_more` interpreter tier in interp/run.rs (tests compare run_hot with
  step_once directly); `vm_new`'s `live` argument and `from_base_reserved`'s; the
  `vm_frozen_builtins` export; `Vm::set_math_random_seed` as a separate entry (keep what
  seeded/deterministic runs use); registry ids, BASE_OFFSETS and djs-prelude's `registry`
  feature (keep the committed path list, its check and `reachable_names`); djs-wasm's
  `snapshot` feature, which every module build enables (merge it; keep djs-core's, djs-embed
  builds without it); comments that still give dump reasons for the lowering (D2720,
  D2733, D2620): the lowering stays, its reason is now "one lowering, no return addresses".
- Unused determinism *experiments* in scripts/determinism (probe/, deep_search.py,
  sandbox_interleave.ts and unused js files); keep native-vs-wasm.sh, abidrive.mjs and every
  file the gate or tests include.

**Keep** (owner): deterministic runs, journals, replay and canonical hashes (see 0306); raw
image byte-identity checks (useful during development); frozen builtins; images in-process;
the lowering itself.

## Acceptance criteria
- [ ] Nothing of lazy compilation left; gate legs and TESTING.md updated; gate CPU before and
      after recorded.
- [ ] The leftovers above removed; ABI renumbered as needed (no compatibility code, D3640).
- [ ] Snapshots and the frame bytecode pin still pass (regenerate the pin only if the layout
      of native builds changes, and say why).
- [ ] Decisions recorded (superseding D2990-D2995, D3721's lazy part, D3732's caveat).

## Log

## Follow-ups

## Result
