# 0092: Compiler code size and compile throughput in wasm

status: done
agent: lexer
branch: issue/0092-compiler-size-speed
depends-on: 0086
touches: crates/djs-core/src/compiler/, crates/djs-core/src/lexer/, crates/djs-bench/ (compile benchmarks)

## Goal
The compiler is the largest code area in the wasm module (0052: ~177–195 KB vs ~60–80 KB
in quickjs-wasi), and every codemode run compiles its script, so compile throughput in wasm
matters too (D016). 0052's root causes for the compiler: 16-byte errors returned through
memory (Q3 boxed them), inlined Vec pushes in the emitters (Q2), and generic code.

## Acceptance criteria
- [x] Measure first: compile throughput (MB/s) and instructions per source byte in the wasm
      build on realistic inputs (pi-like codemode scripts, the prelude, a large library
      such as typescript.js), against quickjs-wasi's parse+compile, plus the compiler's
      wasm code size per module (scripts/wasm-funcs.py, wasm-dwarf-lines.py).
- [x] Reduce code size where it costs nothing in speed (table-driven parts, out-of-line cold
      paths, error construction, duplicated parse paths for TS/JS, monomorphization), and
      speed up the hot paths (lexer next_token in wasm, the resolution pass, line tables,
      allocation in the parser) as the profile shows.
- [x] Numbers before/after (size in KB raw and gzip, compile MB/s in wasm), no behavior
      change (snapshots, the test262 compile sweep, strip-types oracle unchanged).

## Log
- Inputs (work/scratch/0092/in): typescript.js 5.9.3 (9.1 MB), highlight.js core (74 KB), a
  7 KB codemode-like script (the scripts/perf/w bodies as async functions), 146 KB of
  dquickts tests. Tools: scripts/perf/wasm-compile.ts (compile only in node, fresh or one
  Sandbox) and wcompile.py (instructions per compile and per byte vs quickjs-wasi's
  COMPILE_ONLY); `crates/djs-bench/examples/prof_compile.rs` (native loop, fixed count for
  instruction counts); djs-bench `compile/` group.
- Before (wasm, f166a25): 905 instr/byte on typescript.js (qjs 500), 750 on highlight.js
  (qjs 521), ~3,200 on the codemode script (qjs ~2,400, small files are dominated by
  per-function costs). Profile (node --cpu-prof): typescript.js spent 15% in
  `FuncState::find_in_scope` (a linear scan of a scope with thousands of bindings) and 7%
  in `line_col`; a typical file: resolution and templates 46%, of which decode 11%
  (four decodes per instruction), the stack check 8%, IC numbering 5%, line columns 5%,
  the peepholes 7%; allocation ~15%; the lexer 8%.
- Size (wasm-funcs --areas): compiler 159.8 KB, compiler/ts 33.2 KB, lexer 17.8 KB.
  wasm-dwarf-lines: emitter call sites 17.6 KB (ops.rs/imm.rs), `?` 14.8 KB, Vec growth
  24 KB, the keyword decision tree 4.5 KB, syntax-error construction ~8 KB (Cow at each
  site).
- Done (D1920): hash index of bindings; line cursor + ASCII byte columns + word scan in
  LineTable::new; one decode per instruction in the stack check, IC sites from it;
  table-driven decode; one out-of-line `encode` for every emitter (-6 KB); peepholes in
  one backtracking pass, threading from the end until stable; loop rotation by binary
  search; keyword perfect hash (-4.4 KB); syntax/ts errors out of line (-4 KB); buffer
  capacities. Output unchanged: snapshots, the disasm of the four inputs, the test262
  compile sweep (`compile_test262 --ignored`), the strip-types oracle (4316/4802, 0
  differ).
- After (wasm instructions per compile, base → now, qjs): typescript.js 8,273M → 5,174M
  (4,561M, -37%), highlight.js 55.4M → 48.2M (37.5M, -13%), codemode 23.2M → 20.2M
  (17.5M, -13%), dquickts tests 185M → 168M (-9%). Steady-state MB/s in wasm:
  typescript.js 24 → 35 (qjs 40.5), highlight.js 42 → 54 (75), codemode 20 → 27 (35),
  dquickts 18 → 21. Native (prof_compile, instr/byte): typescript.js 455 → 274,
  highlight.js 264 → 225, codemode 681 → 553.
- Size after: compiler 150.1 KB (-9.7), compiler/ts 31.4 KB (-1.8), lexer 14.2 KB (-3.6),
  together 210.7 → 195.8 KB (-7%); shipped module 1,485,742 → 1,472,377 bytes, gzip
  517,652 → 514,990.

## Follow-ups
- Still ~30% behind quickjs-wasi on mid-size files. Left in the profile: the re-emit
  itself (emit_function ~12% self), the stack check (~8%: three per-pc arrays per
  function), decode (5%), allocation (~15%: FuncState's many Vecs, per-function buffers;
  a reused scratch arena for the transient per-function vectors would remove most of
  it), atom interning of identifiers (~5%), the lexer (~9%).
- Line/column at runtime instead of compile time (store source offsets in the line table,
  map them on error from a per-Script lazily built LineTable) would remove the column
  work from every compile (~2-4%), at the cost of a format change.
- Size: the rest is diffuse (`?` plumbing, Vec growth, inlining into the parse_*
  functions); the next step would be structural (a compiler crate at opt-level "z",
  fewer generic helpers), measured against the throughput.

## Result
Compile in wasm is 13-37% fewer instructions (typescript.js 905 → 568 instr/byte, qjs
500; codemode-size scripts -13%), steady-state MB/s +28-45%; compiler+ts+lexer code
210.7 → 195.8 KB. No output change. See D1920 and the Log for the numbers and tools
(scripts/perf/wasm-compile.ts, wcompile.py, djs-bench `compile/`, the prof_compile
example). Not reached: parity with quickjs-wasi (still ~1.3x on mid-size files) and a
large size cut; the follow-ups list what is left.
