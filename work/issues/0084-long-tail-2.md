# 0084: Long tail, part 2 (routed leftovers of 0082)

status: done
agent: lexer
branch: issue/0084-long-tail-2
depends-on: 0082
touches: crates/djs-core/src/interp/ (realm: Function.prototype; the array length set path), builtins/ (atob/btoa, DOMException), crates/djs-core/src/builtins/, js/pi/ (drop the adapter's atob/btoa polyfill), ratchets

## Goal
The 0082 leftovers that nobody else owns:
- `Function.prototype` is a plain object, not a callable function that returns undefined
  (9 test262 tests).
- Setting an array's `length` to a non-number (`a.length = true`, `new Number(6)`, an
  object with valueOf) throws RangeError instead of running ToNumber/ToUint32 per
  ArraySetLength (4 tests; a 0034 follow-up).
- `atob`/`btoa` as native globals (dquickts has them, pi's adapter polyfills them in JS,
  and codemode moves images as base64), with a minimal `DOMException` (name, message,
  code, the `InvalidCharacterError` that atob throws), matching qjs's behavior (the
  difftest cases). Coordinator decision: in scope, since codemode benefits.

## Acceptance criteria
- [x] All three, with qjs messages, tests, and the ratchets updated; the pi adapter's
      polyfill removed and pi's suite rerun.

## Log

- Function.prototype: the realm makes it a BuiltinClosure of the new
  `%FunctionPrototypeBehavior` (returns undefined). 9 test262 tests pass. Messages and
  `String(Function.prototype)` checked against qjs.
- Array `length` [[Set]] with non-Number values: primitives convert inline
  (`array_length_value`), objects go through `%ArraySetLength(a, v, strict)` (two
  conversions, as the spec's hints `number,number`). This covers `set_property` (put)
  and `ordinary_set` (Reflect.set). 4 test262 tests pass.
- DOMException, atob, btoa: builtins/web.ts, crates/djs-core/src/builtins/dom.rs, six
  leaves in intrinsics.d.ts. The constructor's [[Prototype]] special case is in
  `install_constructor`. `error_summary` reads a DOMException's slots, so `djs run` and
  console show `InvalidCharacterError: ...`.
  - qjs's messages ("must be called with new", "DOMException object expected", "1
    argument required", "string is not correctly encoded", "string contains an invalid
    character").
  - dquickts' test_base64 and test_dom_exception pass in difftest, plain and stressed.
- pi adapter: the atob/btoa polyfill is removed from js/pi/vm.ts. The js tests pass
  (45 + 1 skipped). pi's suite is 221/226 as before: the 5 inherent failures (the
  fingerprint regex ×2, strict-mode "ignores patches" ×3, docs/host.md).
- crates/djs-bgen/tests/behavior.js (outside `touches:`): qjs stand-ins for the six new leaves (the @value install function calls DOMExceptionConstant).
- Tests: tests/data/e2e/web_globals.js (expected from qjs, all three e2e modes), the
  builtins::dom unit tests.
- test262 active: 56 → 43 expected failures. difftest: 4 → 2 (the immutable ArrayBuffer
  cases, later).

## Follow-ups

- DOMException.prototype's own property order is constructor first (bgen's install
  order); qjs has name, message, code, constructor. Not observable in the tests.
- `console.log(domException)` prints `Name: message`; qjs prints `DOMException {  }`.
  Deliberate, it is more useful.

## Result

Function.prototype is callable, array `length` sets convert as ArraySetLength, and
`atob`/`btoa`/`DOMException` are native globals as in qjs (the pi polyfill is gone). D1840;
TESTING.md "Known differences" updated: 43 test262 failures left (40 by design, 3
resizable typed arrays for infra), 2 difftest cases (immutable ArrayBuffer, later).
