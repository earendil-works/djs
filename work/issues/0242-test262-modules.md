# 0242: test262's module tests: a fixture loader in the runner, groups and ratchet

status: in-progress
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
- [ ] the runner runs module tests on djs and qjs; negative `resolution` phase tests check
      link errors; async module tests work
- [ ] tests/test262.toml groups updated with reasons; `djs-test262 list` counts in TESTING.md
- [ ] ratchet updated (`--update`), the oracle failures file for qjs updated
- [ ] the slim build keeps `modules` (it is not a slim cut; only djs-wasm leaves it out): the
      slim leg's test262 run passes with its own ratchet
- [ ] the gate's test262 leg green, in both its lazy and eager runs

## Notes
- TESTING.md "test262" describes the runner, the groups and the ratchet.
- Run filtered: `cargo run -q -p djs-test262 -- run --engine djs language/module-code`.

## Log

## Follow-ups

## Result
