# 0168: ES modules for the Rust pi embedding

status: in-progress
agent: pi-esm
branch: issue/0168-pi-es-modules
depends-on: -
touches: docs/guide/ (tool-generated regions only beyond named prose), tests/test262.toml (comment only), crates/djs-core/src/compiler/, crates/djs-core/src/interp/run.rs, crates/djs-core/src/bytecode/ops.rs, crates/djs-core/src/vm/image.rs, crates/djs-core/compat_inputs.rs, crates/djs-core/src/vm/modules/, crates/djs-core/src/vm/modules.rs, crates/djs-core/src/vm/mod.rs, crates/djs-core/tests/modules.rs, ARCH.md, TESTING.md, docs/decisions.md, docs/modules.md, docs/guide/compiler.md, docs/guide/introduction.md
guide: docs/guide/compiler.md, docs/guide/introduction.md

## Goal
Implement ES modules for the Rust pi embedding. The user explicitly changes the
no-modules non-goal. Do not merge, push or publish; parent reviews and publishes.

## Acceptance criteria
- [x] Execute acyclic JavaScript source graphs through host-controlled resolution/loading.
- [x] Static named/default imports, declarations, export lists and explicit reexports.
- [x] Shared live binding reads, read-only imports, local TDZ and within-module hoisting.
- [x] Synthetic host exports, once-only source execution and cached failures.
- [x] Arena-rooted graph/evaluation state with fuel/host-call/GC/image regression coverage.
- [ ] Graph-wide instantiation and hoisting, cyclic source evaluation and namespaces/star exports.
- [ ] import.meta, dynamic host imports, top-level await and async module graph evaluation.
- [ ] wasm module-loading ABI and pi-rs replacement integration.
- [x] Focused tests and honest documentation of the delivered stages.

## Log
- 2026-11-03: Read architecture, testing, decisions and work process; inspected the
  read-only pi loader and engine. Current module parsing is erasure-only. Full ESM
  requires compiler binding resolution, arena graph/evaluation state and job queue
  integration. First milestone will implement the native arena binding graph used
  by synthetic modules and future source linking, with live aliases and TDZ. It
  will not expose rewritten scripts as executable modules.

- 2026-11-03: Implemented native `vm::modules::ModuleGraph` using null-prototype
  objects and shared cells, rooted by one handle. Added focused integration tests.
  Updated ARCH, TESTING, guide introduction/compiler and docs/modules.md with the
  exact boundary between the binding foundation and executable source modules.
- 2026-11-03: Git commit blocked by sandbox denial creating
  `/Users/jkneen/Documents/GitHub/djs/.git/worktrees/pi-esm/index.lock`.
  Normal Cargo cannot resolve crates.io; offline workspace resolution needs uncached
  pulldown-cmark 0.13.6. Testing an isolated scratch workspace containing unchanged
  core/prelude/lzrc sources and cached libm, without altering the real lockfile.
- 2026-11-03: Minimal additional touch to compat_inputs.rs records the new graph
  property names and binding kind codes in the image dependency descriptor. No
  existing slot changes or compatibility version bump. Added async synthetic call
  snapshot coverage; this tests the existing promise API, not async module evaluation.

- 2026-11-03: Isolated integration tests 7/7, clippy -D warnings and wasm32 core
  check passed; real-worktree formatting and diff whitespace checks passed. The
  initial async test used a bare realm without its promise prelude; fixed the fixture
  to use the shipped prelude and verified pending async call restore/settlement.

- 2026-11-03: Existing dynamic_import e2e passed in plain, suspension/image and GC
  stress modes in the isolated workspace, confirming default no-loader rejection.

- 2026-11-03: Parent authorizes compiler/resolver/interpreter changes for actual
  source graphs. Extending token parsing and lexical resolution into shared arena
  binding cells; first executable stage rejects source cycles until instantiation
  hoisting and cyclic evaluation are implemented. Existing milestone is preserved.

- 2026-11-03: Source stage now executes acyclic graphs. Added source regression tests
  for live imports/reexports, duplicate/missing exports, default binding semantics,
  hoisting/shadowing, caching and cached exception identity, synthetic imports, parked
  host calls and suspension/GC/image restoration. Fixed import ASI so a subsequent
  assert call is not mistaken for an attributes clause.
- 2026-11-03: Actual workspace builds are available now. Used a writable Cargo cache
  under work/scratch/0168/cargo-home to avoid forbidden writes to the shared cache.
  Source integration tests: 20 passed. Compiler unit tests: 41 passed, 1 ignored.
  Scheduler host allocator failure regression: 1 passed. Formatting and diff checks
  pass. Final clippy/wasm/default-import/docs checks passed.
- 2026-11-03: Ran djs-docs regen, which updated only ref-opcodes.md and status.md.
  The guide checker passed with a pre-existing warning that primary vendor/test262
  is not at its pinned revision; its conformance totals were kept, not regenerated.
  tests/test262.toml changes only an obsolete comment, not grouping or test policy.

- 2026-11-03: Final actual-workspace validation passed: 20 integration tests, the
  scheduler host-allocation regression, clippy with warnings denied, wasm32 checks
  with default and no default features, and the existing dynamic_import e2e in all
  three stress modes. Guide check passed, retaining old conformance totals because
  shared vendor/test262 is unpinned. No ratchets changed. Retried staging/commit after
  the writable-root update, but Git still cannot create
  /Users/jkneen/Documents/GitHub/djs/.git/worktrees/pi-esm/index.lock. All changes
  remain uncommitted for the parent, who explicitly handles this fallback. No
  merge/push/publication. Prepared commit message at /tmp/djs-pi-esm-commit.txt.

- 2026-11-03: Parent reviewed the source-stage changes and updated README and guide
  goals to remove stale no-modules claims. Independent actual-workspace validation:
  454 core unit tests passed, 25 existing ignored; 20 module integration tests passed;
  clippy with warnings denied and wasm32 core check passed. The initial full unit run
  lacked the Unicode normalization fixture; fetched the pinned 17.0.0 test data into
  the primary ignored vendor directory and reran successfully. Publication is an
  experimental acyclic-source-module PR, not a complete extension-runtime replacement.

## Follow-ups
- Full module grammar/early-error conformance, graph-wide instantiation and namespace exotic objects.
- Asynchronous loader polling, dynamic import/import.meta and async graph evaluation.
- pi-rs integration, wasm ABI and module test262 runner support.

## Result
Executable source stage implemented on this branch, preserving the native binding
foundation. `ModuleGraph::load_source` uses `ModuleLoader` to resolve/load sources,
then token-parses imports/exports, compiles bodies and links aliases without running
code. Module-local references, including nested functions, resolve into shared arena
binding records via new module_get/module_put bytecode. Default/named imports,
explicit reexports, exported declarations and default expressions/functions/classes
execute with live bindings, read-only import errors, local TDZ and private scopes.

`begin_evaluation` / `poll_evaluation` use an arena-backed dependency queue, cursor,
body functions and execution states. Success and failures are cached by canonical
identity. Fuel slicing, GC, parked host calls and snapshot restoration retain the
same partially executed body. Host allocator failure terminates the scheduler and
prevents restart of a partially executed module. Native host modules supply imports.

The full issue remains in progress. Source cycles are explicitly rejected; namespace
objects/imports, star exports, graph-wide function instantiation before evaluation,
import.meta, dynamic host loading, top-level await, TS module grammar, wasm ABI and
pi-rs integration remain unimplemented. This stage is not a completeness/replacement
PR. `docs/modules.md`, ARCH, TESTING and the guide describe the exact boundary.

Validation and commit details are recorded in `/tmp/djs-pi-esm-source-result.md`.
No merge, push or PR was attempted.
