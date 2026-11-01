# 0085: Leaf fast paths for everyday builtins (0081 proposal 1)

status: open
agent: -
branch: issue/0085-everyday-builtin-fast-paths
depends-on: 0081
touches: builtins/ (string.ts, math.ts, number.ts, object.ts: the fast-path dispatch only), crates/djs-core/src/builtins/ (new leaf intrinsics), crates/djs-core/src/bytecode/intrinsics.rs (regenerated), crates/djs-bench/ + scripts/perf/ (measurements)

## Goal
0081 F1: in the pi-like script 95% of executed bytecodes are prelude builtins, and
everyday builtins are 4–17x slower per call than quickjs-wasi (`Math.max` 17x, `padEnd` 14x,
`String(i)` 12.5x, `includes` 7.4x, `split` 6.7x, `hasOwnProperty` 6.6x). Give them a
leaf fast path in Rust when the receiver and arguments are primitives (or plain objects for
hasOwnProperty), falling back to the spec path otherwise, so no user code can observe the
difference (the D1351 rule: check whatever the spec would look up).

## Acceptance criteria
- [ ] Fast paths for the hot set measured by 0081 (String: includes, indexOf, split with a
      string separator, slice, substring, trim*, pad*, startsWith/endsWith, repeat,
      toLowerCase/UpperCase, replace/replaceAll with string patterns; `String(x)`/
      ToString of numbers; Math.max/min/floor/ceil/round/abs; Number.prototype.toString
      /toFixed; Object.prototype.hasOwnProperty / Object.hasOwn; Array.isArray, …),
      chosen by instruction counts on the pi-like script and wmicro.py.
- [ ] Coordinate with infra (0080: for-of, native builtins quick fixes, natives wave 2
      including Object.keys/entries and JSON) via the coordinator: this issue owns
      String/Math/Number and hasOwnProperty/hasOwn leaf fast paths.
- [ ] Per-call instruction counts before/after (wmicro.py, wasm primary per D016), and the
      pi-like script; difftest/test262 identical, both backends.

## Log

## Follow-ups

## Result
