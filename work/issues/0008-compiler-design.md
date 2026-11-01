# 0008: Compiler and bytecode design note

status: done
agent: lexer
branch: issue/0008-compiler-design
depends-on: 0003
touches: docs/compiler.md

## Goal
A design note (not code) for the single-pass compiler and the bytecode, concrete enough
that 0006 (P0) and the P1 issues can be implemented from it without re-deciding the
fundamentals. It also becomes the basis for 0006's acceptance criteria. Written by the
lexer author, so it builds directly on the lexer API that landed.

## Acceptance criteria
`docs/compiler.md` covers:
- [x] Parser structure: the recursive descent over the 0003 lexer, precedence climbing,
      the nesting-depth limit, and how the follow-ups from 0003 (escaped keywords,
      strict reserved words, `#constructor`, …) are enforced.
- [x] The ambiguities and how each one is resolved: arrow heads, destructuring
      assignment versus a literal, regex versus divide, ASI and restricted productions,
      `async` arrows, `let` as identifier versus declaration, and (sketched for P3) TS
      generics. For each one: a token-level scan, or checkpoint/rewind (ARCH technique 2),
      with the exact checkpoint contents.
- [x] Bytecode: the instruction encoding (opcode `u8` + immediates, widths), how jumps
      and labels are fixed up, an initial opcode list for P0 and P1 with stack effects, and
      how the instruction table is defined once (a macro) so that the disassembler, the
      encoder, the interpreter and the snapshot build hash all derive from it.
- [x] Scopes and the resolution pass (ARCH technique 3): the scope-generic opcodes, the
      per-function scope tree that the compiler records, how captures are reported by
      inner functions, the rewrite to local/boxed/global access (in place or re-emit),
      hoisting, TDZ checks (and when they can be elided), and Script versus async-body
      mode (D004).
- [x] The function template layout, as a heap cell description in terms of the 0005 heap
      (if 0005 has not landed when you get there: in abstract terms, with a note), the
      constant pool, the line table and the exception handler table.
- [x] Frames and the interpreter contract for P0/P1 calls: what a JS frame record
      contains, how a re-entrant op (getter, `ToPrimitive`) pushes a callee with a
      continuation, and how `HostCall` parks a frame (ARCH "Interpreter").
- [x] Error reporting: SyntaxError positions and messages (QuickJS wording), and
      early-error coverage strategy (test262 negative tests).
- [x] A suggested split of P0 and P1 into implementable issues, each with dependencies.

## Notes
- This is a design note, no code. Keep it as long as needed but no longer. Prefer
  tables and short examples over prose. Mark any proposal that deviates from ARCH.md
  clearly, so the coordinator can update ARCH.
- QuickJS (`quickjs.c`: `js_parse_*`, `resolve_variables`, `resolve_labels`, the opcode
  list in `quickjs-opcode.h`) is the main reference. Say where we deliberately differ.
- 0005 (heap) is being built in parallel. Read its worktree
  (`.worktrees/0005-values-heap`, read-only) for its current direction if useful, but
  do not depend on unmerged details.

## Log

- 2026-11-01: Read ARCH/TESTING/decisions, the 0005 worktree (cells! layout macro,
  NaN-boxed Value with an uninitialized tag, stable u32 atom ids, Roots::STACK), and
  QuickJS: `js_parse_skip_parens_token` and `is_regexp_allowed`, the try/finally
  emission (dummy value slot + gosub), `js_create_function` (children first, then
  `resolve_variables`, `resolve_labels`, `compute_stack_size`), and the opcode list.
  Probed qjs for about 40 early-error and runtime messages; they are in the note's
  tables.
- Wrote `docs/compiler.md` (11 sections). Main choices, recorded as D1080-D1086:
  speculation never emits (lexer-only checkpoints; the only code rewrite is
  last-instruction lvalue conversion); resolution once per unit, children first;
  `u8` + LE immediates, fixed `i32` jumps, one `ops!` table generating everything;
  captured bindings in Box cells (no `close_loc`); handler tables + gosub/ret;
  re-entrant ops finish immediately via fixed continuations, with slow paths in
  privileged JS (`prelude.js` in P1); frame layout in the index-addressed value stack.
- Differences from QuickJS called out in the note: the bailing arrow-head scan, the
  memoized destructuring scan (QuickJS rescans every nested literal), the name-ref side
  table, Boxes instead of stack var_refs, handler tables instead of catch offsets on the
  stack, and no per-op recursion.
- Also touched `docs/decisions.md` (not in `touches:`) to append D1080-D1086, as the
  coordinator asked.

## Follow-ups

- Coordinator: apply the proposed ARCH edits (note §11): technique 2 and 3 wording,
  re-entrant ops (no resume states), privileged prelude in P1, exceptions paragraph.
- 0005 / coordinator: an internal-reference Value tag (for example 13, "cell") so
  function objects can hold their template and Boxes in `SLOTS` (§6.3).
- New cell kinds for 0006: `FuncTemplate`, `Script`, `Box`, `Bytes`, `RefArray`, plus
  the `Interp` state cell and the realm's global lexical record (§6, §7).
- Confirm the codemode top level against pi's real wrapper: dquickts' own async eval
  uses global-eval semantics (top-level `var` becomes a global), while ARCH, TESTING and
  this note use a function body (§11).
- Lexer: `rescan_less` for TS `f<<T>(...)` when P3 starts (§3.9).
- Later optimizations, deliberately not in P0/P1: short jump forms with relaxation,
  short opcode forms, dead-code removal, slot sharing between sibling blocks, a
  peephole pass for `&&`/`||` in conditions, and for-of array superinstructions.

## Result

`docs/compiler.md`: the compiler and bytecode design for P0/P1. It covers parser
structure (precedence table, depth limit, the 0003 follow-up checks), every
ambiguity with its resolution and the checkpoint contents, the bytecode encoding and the
single `ops!` table, labels and fixups, the stack-depth pass, the initial P0/P1 opcode
list with stack effects, scopes and the resolution pass (storage, captures, hoisting,
TDZ with elision rules, Script vs async body), template/Box/Script cell sketches, the
constant pool, line table and handler table, the frame layout and interpreter contract
(calls, continuations, HostCall parking, exceptions, fuel), error reporting with
verified QuickJS messages, the early-error strategy, worked examples, and a P0/P1 issue
split with dependencies (§10, usable as 0006's acceptance criteria). Decisions
D1080-D1086 are in `docs/decisions.md`.

Known gaps: cell layouts are sketches until 0005 lands; TS is only sketched (P3);
generators/async (P2/P5) are only placed, not designed.
