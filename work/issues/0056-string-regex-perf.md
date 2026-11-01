# 0056: String and regex hot-path performance (+ the test262 property-escape tests' gate cost)

status: in-progress
agent: num
branch: issue/0056-string-regex-perf
depends-on: 0050
touches: crates/djs-core/src/regexp/, crates/djs-core/src/builtins/ (string/regexp leaves), builtins/ (string.ts, regexp.ts, function.ts apply), crates/djs-core/src/heap/ (string building helpers, additive), crates/djs-core/src/interp/ (only isolated fast paths, coordinated), crates/djs-bench/ (benchmarks), crates/djs-core/src/json/ (only for the optional JSON source-text item)

## Goal
After 0050's promotion, the active test262 run costs ~250 CPU-s more (~30 s wall at
-j 18) because the harness's `buildString` (regExpUtils.js) builds 1M-code-point strings:
a JS loop `codePoints[length++] = cp` plus `String.fromCodePoint.apply(null, chunk)` with
10k arguments, then `+=`, and the property-escape tests match regexes over them. That
cost hits every gate. The same kinds of hot paths matter for codemode (string building,
regex over large tool output).

## Acceptance criteria
- [ ] Profile the slowest test262 property-escape tests natively and record where the
      time goes (the element-store loop, `apply` with a big argument array,
      `fromCodePoint`, rope concatenation/flattening, the regex scan with \p tables, exec
      copying the program out of the heap: 0050 follow-up, 11–136 KB per exec).
- [ ] Fixes for what dominates, for example: `Function.prototype.apply` with a dense
      array (no per-argument slow path), a bulk `fromCodePoint`/`fromCharCode` leaf
      that writes straight into a flat string, exec reading the program in place from
      the heap (zero-copy, or a per-RegExp cached decode), rope-friendly `+=`, and
      regex literal evaluation cost (0050: 8x slower than qjs).
- [ ] Before/after: the gate's test262 step (CPU-s and wall), the slowest 20 tests, and
      djs-bench numbers (add string-building and regex-scan benchmarks).
- [ ] No semantic changes: ratchets unchanged or better, stress modes identical.
- [ ] Optional, if time permits: JSON.parse source text access (0050 follow-up:
      `{source}` in the reviver, `JSON.rawJSON`, `JSON.isRawJSON`, stringify support).

## Notes
- heap (0051) is in builtins/ and realm; infra (0054) in builtins/global functions;
  devflow (0053) changes the test runners. Keep interp changes isolated and tell the
  coordinator before touching shared hot paths in run.rs.

## Log

## Follow-ups

## Result
