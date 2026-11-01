# 0014: P0 compiler

status: done
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
- [x] `compile(heap, source, mode) -> Result<ScriptRef, SyntaxError>` producing the
      template cells from 0013.
- [x] The parser per sections 2 and 3, as far as P0 constructs go. The 0003 follow-up
      checks that apply to P0 syntax are in place.
- [x] Resolution (section 5) for locals/globals, without captures (those come with
      functions in P1-1). The design must not block captures.
- [x] Number literals through `num::literal_digits_value`, strings through atoms or the
      heap, regex literals rejected with a clear "not supported yet" SyntaxError (0011 is
      building the engine).
- [x] Error messages per 8.1 (QuickJS wording) with line/column.
- [x] Tests: disassembly snapshot tests for representative programs (a stable text
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

- Rebased onto 0015 (2373382). `end_to_end` runs 31 async-body programs (operators,
  loops, labels, switch fallthrough, objects, optional chains, logical assignment,
  templates, TDZ and const errors, `console.log` as a host call) on the interpreter. All
  31 outputs were cross-checked with qjs (wrapped as pi does,
  `(async (console) => {...})`).

## Follow-ups

- 0022 (wiring): the interpreter does not run `special_object THIS` (any use of
  `this`), `global_decls` or `init_global_lex` yet, so Script-mode programs with
  top-level declarations and anything using `this` need those ops. The semantics are
  in D1140/D1141 (`get_global_typeof` must throw for an uninitialized global `let`).
  `delete` is also still "not supported yet" in the interpreter.
- Script completion value for `$262.evalScript` (compiler.md 5.8), when 0007 needs it.
- P1-1: functions, closures (the resolver's `classify` walk to enclosing functions,
  Boxes), `rebox` for per-iteration `let` bindings in `for(;;)`.
- A peephole pass: `set_*`/`insert*` followed by `drop` in expression statements,
  `jump` to the next instruction, `&&`/`||` in conditions as direct jumps, constant
  folding of `-1`.
- The destructuring cover check (`{ a = 1 } = x`) only looks at the innermost literal;
  P1-6 replaces it with the memoized pattern scan (compiler.md 3.3).

## Result

`djs_core::compiler::compile(heap, src, &CompileOptions::script(name) |
CompileOptions::async_body(name, &["tools", "console"])) -> Result<Ref, CompileError>`
returns a `FuncTemplate` (and its `Script`, with the source) for the P0 language
(D1142). Call the instantiated closure with `this` = the global object; in body mode,
pass the params. `CompileError` carries the QuickJS-worded message, the byte offset and
line/column. Templates carry the line table (`; line:col` in listings), names of locals,
`func_flags`, and `global_decls` for Script top-level declarations (D1141).

`djs disasm [--body] file.js` prints the bytecode. Tests: 6 disassembly snapshots
(`compiler/snapshots`, update with `DJS_UPDATE_SNAPSHOTS=1`), ~75 error cases, the
depth-limit test, 31 end-to-end programs on the interpreter, and an ignored test262
compile sweep (no panics; 11,443 tests compile).

Known gaps: see Follow-ups; everything with functions is "not supported yet".
