# 0054: pi codemode follow-ups (from 0049)

status: done
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
- [x] Stack overflow throws `RangeError: Maximum call stack size exceeded` (D013), and
      difftest cases that hit it are marked as expected differences.
- [x] `encodeURI`, `decodeURI`, `encodeURIComponent`, `decodeURIComponent`, `escape`,
      `unescape`, with qjs messages (URIError).
- [x] Errors after pi's lockdown print an empty name ("Uncaught : …"): find out why
      (probably name lookups on frozen prototypes, or the error printing reading an own
      property) and fix it.
- [x] Rerun pi's suite (`node js/pi/pi-tests.ts`) and update docs/host.md's gap list.

## Log
- Stack overflow (D013, D1540): the interpreter's stack limit and JSON.stringify's nesting
  limit throw `RangeError: Maximum call stack size exceeded`. Parse depth limits stay
  SyntaxErrors. Outside `touches`: djs-difftest got a `djs-expect` directive (case.rs,
  runner.rs, about 30 lines), and TESTING.md documents it. errors/other-errors.js and
  console/cyclic-array.js use it, keeping qjs's .expected.
- URI functions + escape/unescape (D1541): builtins/uri.ts + builtins/uri.rs leaves,
  `ErrorKind::UriError`. Output and messages are identical to quickjs-wasi and dquickts on
  a probe of 140 inputs (work/scratch/0054/so.js). New difftest case builtins/uri.js
  (recorded from qjs). Function.prototype `length`/`name` were missing (20.2.3); the four
  `*_A5.2` URI tests needed them. test262: 175 more active tests pass (ratchet updated).
  difftest: errors/other-errors.js passes now.
- Empty error name (D1542): `error_summary` read `name` as a data property only. After pi's
  lockdown that is an accessor, so the name came out empty. It now falls back to the
  nearest native error prototype's name. Unit test `vm::tests::error_display_with_accessor_names`.
- pi's suite: 207/226 (was 204), and the deep recursion test passes. Globals still missing vs
  quickjs-wasi: WeakRef, FinalizationRegistry, DisposableStack, AsyncDisposableStack,
  SuppressedError (0051), DOMException, eval, SharedArrayBuffer. docs/host.md updated.

## Follow-ups
- qjs (dquickts) has `queueMicrotask` as a global and djs does not. The pi adapter
  polyfills it (D1494), but a native one would match the oracle.
- The remaining pi failures belong elsewhere: the regexp on 1 MB images (num), WeakRef (0051),
  eval/Function, `import()` (compiler), strict mode (by design), the fingerprint (pi).

## Result
- Stack overflow is `RangeError: Maximum call stack size exceeded` (D013/D1540), including
  JSON.stringify nesting. The difftest directive `djs-expect` marks the deliberate
  differences.
- `encodeURI`, `decodeURI`, `encodeURIComponent`, `decodeURIComponent`, `escape` and
  `unescape` match qjs's results and URIError messages (D1541). Function.prototype has
  `length` and `name`. test262 active: +175.
- Errors display their native name after pi's lockdown (D1542).
- pi's own suite on djs: 207/226 (from 204). The remaining failures are listed in
  docs/host.md "pi codemode".
