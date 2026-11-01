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

- 2026-11-01: `crates/djs-core/src/compiler/`: `mod.rs` (API), `parser.rs` (state,
  tokens, constants, declarations), `stmt.rs`, `expr.rs`, `scan.rs` (arrow-head
  scan, pattern-closer scan), `func.rs` (scopes, bindings, name refs), `resolve.rs`
  (classification, TDZ, storage, re-emit, line table, template), `tests.rs` plus
  `snapshots/*.js` → `*.disasm`.
- Scope of P0 (D1142): everything without functions. Object/array literals and
  untagged templates are in (the ops existed), while methods, accessors, spread
  arguments, tagged templates, destructuring, for-in/of, try, regex literals, BigInt,
  classes and functions give "... not supported yet". Arrow heads are detected with
  the bailing scan from the note, so `(a, b) => 1` gets the right message.
- Shared surface, additive only (mentioned to 0015): `bytecode::special::THIS` (4),
  `bytecode::global_decls` (encode/decode of the declaration list),
  `bytecode::func_flags`, a `lines` field in `DisasmContext` (listings show
  `; line:col`) and a decoded `global_decls` operand in the disassembler. No changes
  to `ops!` or the cell layouts.
- Found by the snapshots: `for (;;)` without an update jumped to itself (`continue`
  label placed after the test). Fixed and the layout documented in `parse_for`.
- Messages/positions checked against qjs (`errors` test, about 70 cases). qjs
  reports declaration conflicts at the token after the name, member errors at the `.`
  or `[`, call errors at the `(`; the line table follows that.
- Depth limit raised from 500 to 1000: depth 1000 needs under 512 KiB natively
  (measured with `DJS_DEPTH_STACK`), and the test runs it on a 1 MiB thread.
- test262 sweep (`compile_test262`, ignored, release): 53,891 files, no panics; 11,443
  compile, 39,903 hit "not supported yet", the remaining errors are negative tests,
  sloppy/module tests and module fixtures (`import`/`export`, top-level await,
  `using`). Negative parse tests that compile: 5, all module-only (`await` as an
  identifier is valid in Scripts).
- docs/compiler.md corrections: depth limit, `init` name refs, `this` prologue, global
  TDZ in `get_global_typeof`, the `global_decls` format.
- `djs disasm [--body] file.js` in djs-cli (`disasm.rs` plus a one-arm dispatch in
  `main.rs`).

## Follow-ups

## Result
