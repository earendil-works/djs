# 0084: Long tail, part 2 (routed leftovers of 0082)

status: in-progress
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
- [ ] All three, with qjs messages, tests, and the ratchets updated; the pi adapter's
      polyfill removed and pi's suite rerun.

## Log

## Follow-ups

## Result
