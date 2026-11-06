# 0236: Performance: the construct path (`new C()`, `new Map()`, `new Promise`)

status: in-progress
agent: pnew
branch: issue/0236-construct-path
depends-on: -
guide: interpreter.md if the call/construct story changes
touches: interp (construct/new, OrdinaryCreateFromConstructor, prototype lookup), builtins constructors (Map, Set, Promise, Error, Array, user classes), compiler if `new` codegen changes

## Goal
pasync (0232b) measured, wasm instructions per op on the shipped module vs QuickJS 2026:
`new C()` of a user class 5.9k vs 2.5k (2.4x), `new Map()` 9.9k vs 2.1k (4.6x),
`new Promise(() => {})` 12.5k vs 5.1k (2.7x, after 0232b's promise fixes): the general construct
path is slow, not the classes. Codemode constructs Map/Set/Date/Error/Promise and small classes
often. Find the root cause (Get(newTarget, "prototype") lookups, OrdinaryCreateFromConstructor in
bytecode, derived-class this binding, builtin constructors' frames, string-key re-interning noted
in 0231) and fix it; target parity with QuickJS. Remember 0230/0231's finding: any edit to
run_hot's own code can cost plain loops 4-6% in V8; keep changes out of its body (out-of-line
helpers) and measure plain calls/loops each time. Exact semantics; dumps unaffected (fuzz regress
both modes). Size: the shipped module has ~5.9 KB to the 430 KB budget after 0232b; keep it small
and report.

## Log

## Follow-ups

## Result
