# 0096: Compiler structural speedups (0092 follow-ups)

status: in-progress
agent: lexer
branch: issue/0096-compiler-structural
depends-on: 0092
touches: crates/djs-core/src/compiler/, crates/djs-core/src/lexer/, crates/djs-core/src/bytecode/ (encoder/decoder internals only)

## Goal
After 0092, compiles are ~1.3x quickjs-wasi on mid-size files in wasm (every codemode run
compiles its script). The remaining cost is spread: the re-emit pass ~12%, allocation of
per-function vectors ~15%, the stack check ~8%, the lexer ~9%, atom interning ~5%.

## Acceptance criteria
- [ ] Structural changes chosen by measurement, for example: emitting final code in one
      pass where the resolution allows it (or patching in place instead of re-emitting),
      pooling/reusing per-function vectors across functions in a compile, merging the stack
      check into an existing pass, computing line/column lazily (only for errors and
      stack traces), and faster interning (a pre-sized table, hashing during lexing).
- [ ] wasm compile instructions and MB/s before/after on the 0092 inputs; output unchanged
      (snapshots, disassembly equality, the test262 compile sweep, strip-types).
- [ ] Stop when the remaining gain per change drops below ~3%, and say so.

## Log

## Follow-ups

## Result
