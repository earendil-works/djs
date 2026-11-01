# 0092: Compiler code size and compile throughput in wasm

status: in-progress
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
- [ ] Measure first: compile throughput (MB/s) and instructions per source byte in the wasm
      build on realistic inputs (pi-like codemode scripts, the prelude, a large library
      such as typescript.js), against quickjs-wasi's parse+compile, plus the compiler's
      wasm code size per module (scripts/wasm-funcs.py, wasm-dwarf-lines.py).
- [ ] Reduce code size where it costs nothing in speed (table-driven parts, out-of-line cold
      paths, error construction, duplicated parse paths for TS/JS, monomorphization), and
      speed up the hot paths (lexer next_token in wasm, the resolution pass, line tables,
      allocation in the parser) as the profile shows.
- [ ] Numbers before/after (size in KB raw and gzip, compile MB/s in wasm), no behavior
      change (snapshots, the test262 compile sweep, strip-types oracle unchanged).

## Log

## Follow-ups

## Result
