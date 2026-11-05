# 0230: Math.max / Math.min fast enough for codemode (2 numeric args)

status: open
agent: -
branch: issue/0230-math-max-min-fast
depends-on: -
guide: builtins.md if the fast-path story changes
touches: builtins/math.ts, the @fast / native builtin machinery, tests

## Goal
0229's benchmarks found Math.max/Math.min at ~200 ns per call in djs-wasm vs ~20 ns in QuickJS
(builtins/math.ts: a rest-args array, CoerceNumbers and a Get loop in bytecode). In the codemode
"heavy" workload, `lines.reduce((m, l) => Math.max(m, l.length), 0)` over 30k lines is 6.5 ms of
djs's 20 ms (QuickJS: 15.8 ms total); with a ternary djs beats QuickJS. Make the common calls
fast (0, 1 and 2 arguments, all numbers, the most common being 2) without changing semantics
(coercion order, NaN, -0 vs +0, valueOf side effects for non-numbers), within the size budget
(shipped module headroom ~15 KB; record the cost). Check the other small Math functions and
similarly hot tiny builtins used in codemode (Math.floor/round/abs, String.prototype.slice/
includes, Array.prototype.push/join, Number/parseInt) for the same rest-args/bytecode overhead
and fix the ones that show up in the 0229 workload set. Measure in wasm instructions (shipped
module) before/after; test262 Math stays green.

## Log

## Follow-ups

## Result
