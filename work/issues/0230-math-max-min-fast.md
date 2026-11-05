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

## Scope extended (coordinator, from 0229's per-op table)
The Math.max cost is one case of a systemic problem: builtins written in the TS subset have
~200-370 ns call overhead vs ~35-50 ns in QuickJS: s.substring 9.6x, padStart/padEnd 10-12x,
s.at 7.5x, s.repeat 5.9x, String(i) 5.8x (`''+i` and `${i}` are fine), Number.isInteger 4.9x,
toFixed 3.8x, trim 3.4x, Math.floor 3.1x, Math.max 19x, toLowerCase 1.9x, slice (@fast) 2.0x,
charAt 2.0x. Find and fix the systemic cause in how bytecode builtins are called (frame setup,
argument handling, rest/arguments objects, prologue checks), so all of them get faster, rather
than a fast path per builtin; add per-builtin paths only where the general fix leaves a gap. The
interpreter's call path is yours for this; 0232 (async) also touches call/resume: coordinate via
the coordinator.

## Log

## Follow-ups

## Result
