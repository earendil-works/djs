# 0054: pi codemode follow-ups (from 0049)

status: in-progress
agent: infra
branch: issue/0054-pi-followups
depends-on: 0049
touches: builtins/ (global functions, error), crates/djs-core/src/builtins/, crates/djs-core/src/interp/ (stack overflow error type only), js/pi/, tests/corpus/ (expected-difference markers), docs/host.md

## Goal
Close the engine gaps that pi's codemode suite found (0049 Result), apart from the ones
that belong to other issues (the regexp memory issue on large images → num; WeakRef,
FinalizationRegistry and the disposables → heap's 0051; eval/Function → a later
decision; `import()` → compiler).

## Acceptance criteria
- [ ] Stack overflow throws `RangeError: Maximum call stack size exceeded` (D013), and
      difftest cases that hit it are marked as expected differences.
- [ ] `encodeURI`, `decodeURI`, `encodeURIComponent`, `decodeURIComponent`, `escape`,
      `unescape`, with qjs messages (URIError).
- [ ] Errors after pi's lockdown print an empty name ("Uncaught : …"): find out why
      (probably name lookups on frozen prototypes, or the error printing reading an own
      property) and fix it.
- [ ] Rerun pi's suite (`node js/pi/pi-tests.ts`) and update docs/host.md's gap list.

## Log

## Follow-ups

## Result
