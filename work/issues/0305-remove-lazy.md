# 0305: Remove lazy compilation and the dead code the dump removal left

status: done
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
- [x] Nothing of lazy compilation left; gate legs and TESTING.md updated; gate CPU before and
      after recorded (after: in the merge report).
- [x] The leftovers above removed (except the fast tier, D3741); ABI renumbered (ABI 12).
- [x] Snapshots and the frame bytecode pin still pass (pin regenerated for one function:
      native pools now laid out as the module's, D3740).
- [x] Decisions recorded (D3740-D3744).

## Log
- Gate CPU before (0304's gate, .worktrees/gate/target/gate-steps.tsv, CPU-s): test262 174.0,
  snapshots 14.6, difftest 60.6, build:difftest 50.6, total 791.3.
- Lazy compilation deleted: compiler/lazy.rs, interp/run/lazy.rs, vm/lazy_tests.rs, the cfg
  sites, `resolve::Unit`/`Plan`/`lazy_roots`/`emit_lazy` (two pools now, written inline),
  `Op::Lazy` (super_call_frame/forward move down one: 213, 214), `capture_desc::IMMUTABLE/
  CHECKED`, `Vm::compile_lazy_templates` and seal's pass, `VmOptions::lazy_functions`,
  `Lexer::seek`, the snapshot interface hash's synthetic-number stripping (the `<no name>`
  case stays: privileged templates keep no names), the import's lazy comment, the
  `--lazy`/DJS_LAZY flags and `lazy` features of difftest, test262 and bench (the bench's
  `compile/typescript.js lazy` row and the status key `compile/typescript-lazy-time-pct`).
- Frame bytecode pin: one entry changed (codemode-harvest/h164-load.js, the top level): the
  per-function constant pools of lazily compilable functions are gone, so native builds lay
  out pools as the wasm module always did (its pin entry is now the module's). Regenerated.
- Class-span recorder deleted (parser.rs, class.rs).
- Gate: test262 runs once (no `--lazy` twin), snapshots once, difftest without `--lazy`,
  the full-only gc-stress leg once; build:difftest builds djs-difftest and djs-abidrive in
  one cargo invocation (same features now).
- Guide: compiler.md ("Lazy functions" replaced by a short "Every function compiles at
  load" with its trade-off), interpreter.md (call table row), testing.md, rust-api.md;
  status.md regenerated.
- **fast_step/fast_more kept** (survey item verified false): with the tier deleted and
  `hot_loop: false` going straight to `step_once`, `hot_loop_is_invisible` and
  `hot_loop_paths_are_invisible_at_every_stop` fail on raw images. The general path is not
  the reference the survey assumed: `lookup_ic` never consults the inline cache (it looks up
  and refills every time, so a slot the tier-0 loop hits keeps an older receiver shape there
  while `step_once` rewrites it), and an array's `length` comes back as an f64 from the
  general path, an int32 from the fast paths. The test-only tier is what makes the raw
  image of a run without the loop equal the loop's. Keeping raw-image equality (the 0282
  dead-slot bug was found by it) means keeping the tier; the alternative (compare canonical
  hashes there) weakens the test. Attempt saved in work/scratch/0305/.
- ABI 12: `vm_new(limit, flags)` (no `live`), `vm_frozen_builtins` gone, the ops after it
  renumbered; `Vm::from_base_reserved`, `Heap::start_capacity`/`restore_capacity`,
  `capacity_target`, the `live` of `from_base_vec`/`new_vm_owned`, and
  `Vm::set_math_random_seed` gone (`set_random_seed` is the one entry).
- Registry without ids: registry.tsv is `kind\tpath`; `Registry::name_cells` gives paths;
  djs-prelude's `registry` module/feature, `BASE_OFFSETS` and its build step gone; the CLI's
  heap graph names prefix cells by `canonical_names`. scripts/harvest/members.py follows.
- djs-wasm's `snapshot` feature merged (djs-wasm always enables djs-core's); build.rs always
  links 128 KiB; build-wasm.sh and gate.py without the feature; guide host.md's stack
  paragraph rewritten.
- Lowering comments (completion.rs, temps.rs, parser.rs, Goal::AsyncBody) give the current
  reason; stale doc comments (seal) fixed.
- scripts/determinism: probe/, deep_search.py, sandbox_interleave.ts, js/async_interleave.js,
  js/general.js deleted; README rewritten for what stays.
- Checked: cargo test --workspace, clippy (workspace, combos, wasm32 release with docs),
  js tests on the dev module, native-vs-wasm, DJS_SNAP snapshot legs, djs-docs check.
- Guide pages updated: compiler.md, interpreter.md, testing.md, rust-api.md, builtins.md
  (registry), host.md (stack), snapshots.md (vm_new), arena.md; status.md regenerated.

## Follow-ups
- The elastic stack protocol (`djs_elastic_stack`, NEED_STACK, js/src/trap.ts's wrapper) may
  be unneeded now that every module links 128 KiB, which covers the parser's full limits:
  measure whether any compile still needs the larger block, and drop it if none does.
- The heap analyzer's names come from `canonical_names` of the prefix; if an alias path is
  wanted instead, it can read tests/builtins/registry.tsv again (no current need).

## Result
Landed: lazy compilation deleted everywhere (feature, op, compiler/interpreter/VM/snapshot
code, harness flags, gate dimension, bench row, research/0218); class-span recorder deleted;
ABI 12 (`vm_new(limit, flags)`, no `vm_frozen_builtins`); `Vm::from_base_reserved` and
`set_math_random_seed` gone; registry entries without ids (no base offsets, no `registry`
feature); djs-wasm's `snapshot` feature merged (128 KiB stack always); determinism
experiments deleted; lowering comments updated. Kept on purpose: the test-only fast tier
(D3741: the general path is not an equal reference on raw images).

What pi must change: nothing in packages/codemode/src/run.ts (it uses the Sandbox only, and
`frozenBuiltins` stays); it needs the new module and js package together (ABI 12: a module
and a Sandbox of different ABIs refuse each other). Snapshots made by earlier builds whose
waiting function is h164-like top-level code with many constants may refuse (no
compatibility, D3640).
