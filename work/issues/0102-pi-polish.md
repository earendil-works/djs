# 0102: pi polish: rerun pi's suite, Error.captureStackTrace, REPL follow-ups

status: in-progress
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
- [ ] pi's suite results before/after, docs/host.md updated.
- [ ] captureStackTrace with tests against qjs.
- [ ] The REPL items with the piped-stdin tests from 0064 extended.

## Log

## Follow-ups

## Result
