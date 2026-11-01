# 0014: P0 compiler

status: in-progress
agent: lexer
branch: issue/0014-p0-compiler
depends-on: 0013
touches: crates/djs-core/src/compiler/, docs/compiler.md (corrections only)

## Goal
docs/compiler.md section 10, row 0006b: parser and emitter for expressions and statements
(no functions yet). Literals, all operators, `if`, loops, blocks, `let`/`const`/`var`
at top level and in blocks, phase-1 names and resolution for locals and globals, labels,
the line table, templates in the heap, and SyntaxErrors with positions. Both Script and
async-body mode as far as P0 needs them (D004, D008).

## Acceptance criteria
- [ ] `compile(heap, source, mode) -> Result<ScriptRef, SyntaxError>` producing the
      template cells from 0013.
- [ ] The parser per sections 2 and 3, as far as P0 constructs go. The 0003 follow-up
      checks that apply to P0 syntax are in place.
- [ ] Resolution (section 5) for locals/globals, without captures (those come with
      functions in P1-1). The design must not block captures.
- [ ] Number literals through `num::literal_digits_value`, strings through atoms or the
      heap, regex literals rejected with a clear "not supported yet" SyntaxError (0011 is
      building the engine).
- [ ] Error messages per 8.1 (QuickJS wording) with line/column.
- [ ] Tests: disassembly snapshot tests for representative programs (a stable text
      format, reviewed by eye), and error tests. Run the compiler over test262 files with
      only P0 syntax, if a cheap filter exists (optional, ignored test).

## Log

## Follow-ups

## Result
