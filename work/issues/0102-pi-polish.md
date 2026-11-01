# 0102: pi polish: rerun pi's suite, Error.captureStackTrace, REPL follow-ups

status: done
agent: infra
branch: issue/0102-pi-polish
depends-on: 0097
touches: js/pi/, js/test/, builtins/error.ts + crates/djs-core/src/builtins/ (captureStackTrace), crates/djs-cli/src/repl* (REPL follow-ups), docs/host.md, docs/cli.md

## Goal
Small but user-facing items while perf rewrites the native codegen (0100, which owns
emit_rust.rs/natives for now):
- Rerun pi's own codemode suite (`node js/pi/pi-tests.ts`, read-only use of pi-private) on
  current main after 0093/0097, and fix any regressions; update docs/host.md's table.
- `Error.captureStackTrace` (qjs and V8 have it; code that targets node uses it), per qjs.
- REPL follow-ups (0064): tab completion of globals and properties (from the heap, no
  getters run), a switch for legacy decorators (`.decorators legacy|tc39`), and an
  interactive `.step` that pauses at each slice with a nested prompt showing the frame
  (needs only the existing Vm::backtrace/run_metered).

## Acceptance criteria
- [x] pi's suite results before/after, docs/host.md updated.
- [x] captureStackTrace with tests against qjs.
- [x] The REPL items with the piped-stdin tests from 0064 extended.

## Log
- pi's suite (`node js/pi/pi-tests.ts`, read-only on pi-private, now at c1b68f09da5b) on
  main after 0093/0097 with the `wasm` build: 221 of 226 pass, the same 5 failures as
  before (2x the quickjs-wasi fingerprint test, 3x "ignores patches to built-ins" from
  always-strict). Per file: declarations 17/17, deterministic 28/30, portable 7/7,
  sandbox 150/153, source 5/5, worker-lifetime 14/14. No regressions; docs/host.md updated.
- Error.captureStackTrace (D2010): per qjs (own non-enumerable `stack`, the builtin's own
  frame left out, constructorOpt cuts up to its topmost call or gives "" when absent,
  TypeErrors for non-objects and non-extensible objects with qjs's messages). Corpus case
  builtins/capture-stack-trace.js (frame names only: the harness's frames and the
  call-site columns of `return f()` differ between engines, see follow-ups).
- REPL (D2011): Tab completion from the heap (globals, dotted paths through data
  properties only), `.complete <text>` for tests, `.decorators tc39|legacy|metadata`,
  `.step [n]` with a nested `step>` prompt (Enter/s/n, c, bt, fuel, q; line mode reads the
  next lines). New `Vm::global_lexical()` accessor (read-only, vm/mod.rs). Tests in
  crates/djs-cli/tests/repl.rs (completion, decorators_switch, stepping) and a unit test
  for the common prefix. The editor's Tab key itself is not covered by an automated test
  (no pty in the tests); checked by reading the code path, which is the same function.

## Follow-ups
- Stack positions of calls in `return f(...)`: qjs reports the statement start (column of
  `return`), djs the call; visible in raw `stack` strings (the difftest drops those lines).
- `Error.stackTraceLimit` (qjs has the property, 10, and honors it); djs's limit is a
  constant (D1282).

## Result
pi's own suite: 221/226 on current main, unchanged (no regressions), docs/host.md
updated. `Error.captureStackTrace` as in qjs (D2010). REPL: Tab completion from the heap,
`.complete`, `.decorators`, `.step` (D2011), docs/cli.md updated.
