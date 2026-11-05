# 0230: Math.max / Math.min fast enough for codemode (2 numeric args)

status: done
agent: mathfast
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
- Measured in wasm instructions (scripts/perf/wmicro.py on the names-kept wasm profile
  build; the 0229 ops.ts runs were made at load 75-146 and their wall-clock ratios were
  noise, so the numbers below are instruction counts). A user call costs 0.8x quickjs-wasi
  and djs bytecodes are cheaper than qjs's: the frames are not the problem.
- Found two systemic costs. (1) A leaf intrinsic that the hot loop does not know leaves
  run_hot for step_once and call_intrinsic, ~550 instructions per call. Math.floor makes 2
  such calls, substring 6. Running step_once in place without exiting gained nothing; the
  cost is in the dispatch. (2) Builtins are chains of tiny helper frames: substring is 1
  builtin frame plus 8 helper frames and ~100 bytecodes.
- Fix (1), D3120: `Exec::pure_leaf`, about 75 pure leaves moved out of call_intrinsic. The
  hot loop runs them in place, filtered by a bitset (`hot::PURE_LEAVES`, checked by a
  test); call_intrinsic tries them last. First version (pure_leaf before every intrinsic)
  cost hasOwn and Object.keys 3-8%; fixed.
- Fix (2), D3121: djs-bgen's JS emitter inlines small helpers as expressions (temporaries
  `$n`, `?:`, the new `%Throw` leaf), at most 160 characters each (that limit measured
  against 260 and unlimited: +1 KB / +2.4 KB / +4.5 KB gz). `@frame` keeps
  CallPrepareStackTrace's frame (stack.rs looks for it; the corpus case
  errors/prepare-stack-trace caught it). 73 helpers vanish from the prelude (registry
  tombstones). Removing the dead helper functions from prelude.js saved nothing: the base
  snapshot's collection already drops them.
- D3122: @fast Math.max/min (dropped in 0159 for want of calls; heavy and json call them now).
- Tried and dropped: @fast calls inside run_hot's call arm (parr's suggestion): +4-9% on
  @fast calls, but -7-10% on plain closure calls (patch in work/scratch/0230/fasthot-v2.patch).
- Size (D3123): +1,803 B gz shipped, +1,388 slim; ceilings raised by that (approved).
- Fixed scripts/perf/instr.py's anchors and its prof.rs patch (it no longer built).
- Guide: builtins.md (inlining, with a generated excerpt of substring), interpreter.md
  (pure leaves in the tier-0 loop).
- Note: my old stash entry ("WIP on issue/0230 ... aa88ab07") is still in the shared stash
  list; its content is applied and committed. pmem's stash was popped into this tree by
  mistake and reverted (pmem restored it).

## Follow-ups
- Per-builtin @fast entries for the ops that are still >2x qjs after the general fix: substring
  (~8.2k instr), padStart/padEnd (~12.5k), at (~5.9k), String(i) (~6k: a rest array; could use
  %ArgumentCount like reduce), Number.isInteger, repeat. Each ~150-300 B gz.
- Raising INLINE_TEXT selectively (e.g. ClampPosition, RelativeIndex) gets substring/at more,
  at ~1.4 KB gz for 260 chars across the board; a per-helper opt-in tag would be cheaper.
- ops.ts ratios need a quiet machine; redo the before/after table there.

## Result
Merged: pure leaf intrinsics in the tier-0 loop (D3120), bgen inlining of small helpers in
the bytecode prelude with `%Throw` and `@frame` (D3121), @fast Math.max/min (D3122), and
ceilings +1.8 KB / +1.4 KB gz (D3123). Wasm instructions per op: Math.max 8.8k → 1.2k,
Math.floor -40%, substring/at/repeat/toFixed/lastIndexOf -35-40%, padStart/startsWith
-30%, trim/toLowerCase/replace/parseInt -20-25%. Gaps that remain over 2x QuickJS
(substring, padStart/End, at, String(i), Number.isInteger) need per-builtin paths or more
inlining; see Follow-ups.
