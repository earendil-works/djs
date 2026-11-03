# 0175: Sandbox run API and the wasm dumper packaging

status: in-progress
agent: example
branch: issue/0175-sandbox-run-api
depends-on: 0172
touches: js/src/sandbox.ts, js/test/, crates/djs-wasm, scripts/build-wasm.sh, js/proto (remove dump.ts, D2564), js/package.json
guide: -

## Goal
Hosts can stop at an idle point, dump and resume (docs/continuation-dumps.md "Sandbox API").

## Acceptance criteria
- [ ] `sandbox.begin(code, params)` returns a run with `state`, `calls`, `settle(id, outcome)`,
      `advance()` and `dump()` (idle only); `Sandbox.resume(text, opts)` returns the sandbox and
      the run with its open calls. `runAsync` is rebuilt on top, its behavior unchanged
      (journals included).
- [ ] wasm exports for loading and dumping format 2; both packagings build: the main module
      with the `dump` feature, and `djs-dump.wasm` next to a dumper-less `djs.wasm` with the
      image transport (persist: image -> dump module -> dump; resume: dump module -> image ->
      main module). Both are exercised by the tests; sizes recorded.
- [ ] js/test/dumpresume.test.ts: round trip at every idle point for scripts with parallel
      tools, loops, generators, classes, rejections; refusals as SandboxErrors.
- [ ] js/proto/dump.ts and its comparison test removed (D2564).

## Log

## Follow-ups

## Result
