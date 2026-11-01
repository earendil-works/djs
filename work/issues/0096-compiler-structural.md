# 0096: Compiler structural speedups (0092 follow-ups)

status: done
agent: lexer
branch: issue/0096-compiler-structural
depends-on: 0092
touches: crates/djs-core/src/compiler/, crates/djs-core/src/lexer/, crates/djs-core/src/bytecode/ (encoder/decoder internals only)

## Goal
After 0092, compiles are ~1.3x quickjs-wasi on mid-size files in wasm (every codemode run
compiles its script). The remaining cost is spread: the re-emit pass ~12%, allocation of
per-function vectors ~15%, the stack check ~8%, the lexer ~9%, atom interning ~5%.

## Acceptance criteria
- [x] Structural changes chosen by measurement, for example: emitting final code in one
      pass where the resolution allows it (or patching in place instead of re-emitting),
      pooling/reusing per-function vectors across functions in a compile, merging the stack
      check into an existing pass, computing line/column lazily (only for errors and
      stack traces), and faster interning (a pre-sized table, hashing during lexing).
- [x] wasm compile instructions and MB/s before/after on the 0092 inputs; output unchanged
      (snapshots, disassembly equality, the test262 compile sweep, strip-types).
- [x] Stop when the remaining gain per change drops below ~3%, and say so.

## Log
- Metric: `scripts/perf/wcompile.py --same` (steady state, all compiles in one Sandbox:
  stable to ~1%, the CPU work of the compiler) as the primary A/B; the fresh-Sandbox
  numbers (Liftoff, memory growth) as the second view. Inputs: 0092's (typescript.js,
  highlight.js core, the 7 KB codemode script, the dquickts tests). Every change checked
  for identical disassembly of all four inputs (work/scratch/0096/same.sh).
- Profile at the start (steady, codemode + highlight.js): lexer 11%, re-emit 9.5% self,
  phase-1 `encode` 6.9%, decode 5.9%, stack check 4%, allocation ~12%, identifier
  interning ~5%, line table 4.5%, peepholes ~6%.
- Kept, in order (steady-state deltas on highlight.js / codemode):
  1. identifier atom cache: -5% / -4% (qtests -4%);
  2. reused stack-check and re-emit buffers: -1% / -1%;
  3. phase-1 code as instructions, not bytes: -5.6% / -6.2%; packed to 9 bytes after the
     fresh typescript.js compile came out +17% with 16-byte instructions (memory growth);
     packed: +8% vs before the change on that one case, -3.6% overall vs base.
- Tried and reverted: recording the final instructions for the stack check (+2%), a
  table-driven byte `encode` (neutral). Checked: next_token runs about once per token
  (highlight.js: 12k calls), so the lexer has no re-lexing to remove.
- Totals (base 33ecb01 → now; qjs): steady highlight.js 29.1M → 26.1M (-10%; 21.3M),
  codemode 8.13M → 7.43M (-9%; 6.24M), dquickts 140M → 129M (-8%), typescript.js
  4,479M → 4,058M (-9%; qjs 4,595M: djs is now ahead there). Fresh Sandbox: highlight.js
  50.4M → 47.5M (-6%), codemode 20.8M → 20.0M (-4%), typescript.js 5,234M → 5,044M
  (-4%). MB/s (steady, wall clock): highlight.js 44 → 49 (qjs 60), codemode 22 → 24
  (qjs 26).
- Stopped here: every remaining candidate is below ~3% per change, or (lazy line/column,
  ~3.6%) needs a line-table format change and a per-Script cache in the heap.

## Follow-ups
- Lazy line/column (~3.6% of a compile): line tables with source offsets, line/column
  computed at error time from the Script source with a lazily built per-Script line
  index. Worth it only together with other work on the line-table or Script format.
- Phase-1 memory: 9 bytes per instruction vs ~2.5 as bytes (typescript.js native peak
  106 → 126 MB). If big compiles in fresh instances matter, a variable-length packing
  of `P1` would win the memory back at some decode cost.

## Result
Compile cost in wasm -8 to -10% in steady state (highlight.js, codemode, dquickts,
typescript.js), -4 to -6% per fresh Sandbox, output identical. Kept: the identifier atom
cache, phase-1 code as packed instructions (no encode/decode round trip), reused
per-function buffers (D1960). Stopped at the ~3% line: the remaining candidates are
listed in D1960 and the Follow-ups. On typescript.js djs's steady-state compile is now
below quickjs-wasi's; on small and mid-size files it is ~1.15-1.2x.
