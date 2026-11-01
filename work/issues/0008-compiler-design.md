# 0008: Compiler and bytecode design note

status: in-progress
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
- [ ] Parser structure: the recursive descent over the 0003 lexer, precedence climbing,
      the nesting-depth limit, and how the follow-ups from 0003 (escaped keywords,
      strict reserved words, `#constructor`, …) are enforced.
- [ ] The ambiguities and how each one is resolved: arrow heads, destructuring
      assignment versus a literal, regex versus divide, ASI and restricted productions,
      `async` arrows, `let` as identifier versus declaration, and (sketched for P3) TS
      generics. For each one: a token-level scan, or checkpoint/rewind (ARCH technique 2),
      with the exact checkpoint contents.
- [ ] Bytecode: the instruction encoding (opcode `u8` + immediates, widths), how jumps
      and labels are fixed up, an initial opcode list for P0 and P1 with stack effects, and
      how the instruction table is defined once (a macro) so that the disassembler, the
      encoder, the interpreter and the snapshot build hash all derive from it.
- [ ] Scopes and the resolution pass (ARCH technique 3): the scope-generic opcodes, the
      per-function scope tree that the compiler records, how captures are reported by
      inner functions, the rewrite to local/boxed/global access (in place or re-emit),
      hoisting, TDZ checks (and when they can be elided), and Script versus async-body
      mode (D004).
- [ ] The function template layout, as a heap cell description in terms of the 0005 heap
      (if 0005 has not landed when you get there: in abstract terms, with a note), the
      constant pool, the line table and the exception handler table.
- [ ] Frames and the interpreter contract for P0/P1 calls: what a JS frame record
      contains, how a re-entrant op (getter, `ToPrimitive`) pushes a callee with a
      continuation, and how `HostCall` parks a frame (ARCH "Interpreter").
- [ ] Error reporting: SyntaxError positions and messages (QuickJS wording), and
      early-error coverage strategy (test262 negative tests).
- [ ] A suggested split of P0 and P1 into implementable issues, each with dependencies.

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

## Follow-ups

## Result
