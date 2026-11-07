# 0242: test262's module tests: a fixture loader in the runner, groups and ratchet

status: done
agent: t262
branch: issue/0242-test262-modules
depends-on: 0241
touches: crates/djs-test262 (module flag, fixture loader, dynamic import once 0243 lands), tests/test262.toml, tests/test262/expected-failures*.txt, tests/test262-slim.toml, crates/djs-core (fixes for failures found), TESTING.md
guide: docs/guide/testing.md (test262 groups), docs/guide/status.md if it states the skip set

## Goal

D3171: test262 is the conformance check for modules. Make djs-test262 run `flags: [module]`
tests as modules (the test file is the root module, its key its path; specifiers resolve
relative to the referrer's directory and load the `*_FIXTURE.js` files from disk; the harness
files run as Scripts first, as now), with qjs too (`qjs` runs modules with `-m`; check how
dquickts' run-test262 does it, and keep the qjs cache key right). Move `flags: module`,
`language/module-code`, `language/import` and `language/expressions/dynamic-import` (static
parts) out of the skip group as far as djs supports them; features that stay skipped
(`top-level-await` until 0248, `json-modules`, `import-defer`, `source-phase-imports`, ...)
stay skipped by feature. Fix what fails in 0241's implementation, or ratchet it with a reason.

## Acceptance criteria
- [x] the runner runs module tests on djs and qjs; negative `resolution` phase tests check
      link errors; async module tests work
- [x] tests/test262.toml groups updated with reasons; `djs-test262 list` counts in TESTING.md
- [x] ratchet updated (`--update`), the oracle failures file for qjs updated
- [x] the slim build keeps `modules` (it is not a slim cut; only djs-wasm leaves it out): the
      slim leg's test262 run passes with its own ratchet (amended: slim has no `embed`
      since D3193, so its `no-modules` group skips the module tests; D3230)
- [x] the gate's test262 leg green, in both its lazy and eager runs

## Notes
- TESTING.md "test262" describes the runner, the groups and the ratchet.
- Run filtered: `cargo run -q -p djs-test262 -- run --engine djs language/module-code`.

## Log

### t262, 2026-11-08
- Runner (crates/djs-test262): `Job::module`, `Status::ResolutionError`; djs.rs `Fixtures`
  loader (keys = paths relative to test/, specifiers joined to the referrer's dir), parse
  phase = `compile_module` of the test alone, resolution = `Vm::load_module`, runtime = the
  Evaluate promise. `drive()` answers `import()` calls (0243 landed meanwhile:
  `set_dynamic_import(true)` + `answer_import`), holding the call's result in a Handle;
  the clock (`set_now`) moves between fuel slices (dynamic-import/await-import-evaluation's
  fixture busy-waits on Date.now()). qjs: driver runs module tests via `std.evalScript`
  `compile_module`/`eval_module` of `import "<abs path>"`, cwd = the test's dir for every
  job; qjs loads requests at compile time, so a compile error whose stack starts in another
  file (or "could not load module") is resolution phase. Cache key mode `module`.
- Group paths accept globs (config.rs), used briefly for a dynamic-import group that is gone
  again since import() works.
- tests/test262.toml: dropped flag `module`, paths module-code/import, features
  dynamic-import/import.meta. 908 tests newly active: djs fails 4 (top-level await without
  the feature tag: 3 `await using` at module top level, dynamic-import-of-waiting-module),
  ratcheted; qjs fails 5 (4 string export names after `export * as`, the TLA one), in
  oracle-failures.txt. Lazy run: same 4.
- djs-core fix (D3231): `import * as ns; export {ns}` is now an indirect export (namespace),
  fixing a linker panic (instn-star-equality.js: "a Box", an unfilled import Box read while
  making a namespace) and two spurious ambiguity errors. Unit test
  `re_exported_namespace_imports`, snapshot modules.disasm updated.
- Slim: no `embed` = no modules (D3193 postdates the issue's slim criterion), so
  tests/test262-slim.toml `no-modules` group (flag module, feature dynamic-import), applied
  when `!djs_core::MODULES`; slim ratchet unchanged.
- Runner fixture suites (tests/fixture/{djs,test262}): module, module-async, module-parse,
  module-resolution, module-fixture-syntax, dynamic-import passes; module-throws,
  module-missing-export failures.
- Checked: djs-test262 tests, djs-core module_tests/compiler, test262 language + Date +
  Promise against the ratchet (OK), slim runner on the newly active set.
- Docs: TESTING.md (skip set, module execution model, counts 39,017/529/14,051, status,
  known differences, slim), guide testing.md (skip set, module tests, ratchet bullet for
  top-level await); status.md/README/index regenerated. D3230, D3231.

## Follow-ups
- 0248 (top-level await): 4 ratchet entries use TLA without the `top-level-await` tag
  (3 `await using` at module top level, module-code/top-level-await/
  dynamic-import-of-waiting-module.js); un-skip the `top-level-await` feature there.
- `import()` inside `$262.evalScript` code resolves against `<evalScript>` (test/ itself),
  not the calling test; no active test needs it.

## Result
test262's module tests run (D3230): `flags: [module]` tests are the root of a module graph
loaded from the checkout's `_FIXTURE` files (djs: `Vm::load_module` over a path loader;
qjs: `std.evalScript` with `compile_module`/`eval_module`), with the parse, resolution and
runtime phases told apart, and `import()` answered through the same loader in modules and
Scripts. Active now: language/module-code, language/import, language/expressions/
dynamic-import, import.meta and every `module` test elsewhere, 908 more tests (39,017
active, 529 direct-eval, 14,051 skipped). djs passes all but 4 of them (top-level await
without its feature tag; ratcheted, 44 entries in all); qjs fails 5 (oracle-failures.txt,
9 in all). Skipped by feature: top-level-await, import attributes/defer/text/bytes, JSON
modules, source-phase imports. Slim runner: `no-modules` group (no `embed`). djs-core fix:
`import * as ns; export { ns }` is an indirect export (D3231: a linker panic and a spurious
ambiguity). Group paths in the toml may be globs.

Run: `cargo run -q -p djs-test262 -- run --engine djs language/module-code`.
