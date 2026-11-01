# 0013: Bytecode table and tools

status: open
agent: -
branch: issue/0013-bytecode-table
depends-on: 0005, 0008
touches: crates/djs-core/src/bytecode/, crates/djs-core/src/lib.rs (mod lines only), crates/djs-cli/ (disasm subcommand only)

## Goal
docs/compiler.md section 4 (and 6.1/6.2 where the code layout touches the heap): the
single `ops!` instruction table and everything derived from it. This comes first because
the compiler (0014) and the interpreter (0015) both build on it, and after it lands they
can proceed in parallel.

## Acceptance criteria
- [ ] The `ops!` macro defines every P0 and P1 opcode from compiler.md 4.5 (name,
      immediates and their widths, stack effect, fuel cost class, flags such as "may
      throw" and "may call", whether it is scope-generic and rewritten by resolution).
- [ ] Derived from it: the opcode enum, an encoder/emitter API (a `CodeBuf` with typed
      `emit_*` helpers, labels and fixups per 4.3), a decoder/iterator, a disassembler
      (readable text with atoms and constants resolved when a heap is given), the stack
      depth checker (4.4), and `OPS_HASH` (fed into the snapshot build hash together with
      the heap's `LAYOUT_HASH`).
- [ ] The cell kinds the compiled code needs (function template, script, bytes,
      ref-array per compiler.md 6.1) declared in `heap/cells.rs`, if that can be done
      without the internal-ref tag (D007). Otherwise document what 0015 must add.
      Coordinate: keep changes to `heap/` minimal and additive (0012 is working there).
- [ ] The line table encoding (6.2) and the handler table encoding (7.4) as plain data
      structures with encoder and decoder plus unit tests.
- [ ] `djs disasm <file.djsbc>` is optional. A test-only disassembler on hand-built code
      is enough.
- [ ] Unit tests: round trips (emit → decode → disassemble), label fixups (forward and
      backward), stack depth on hand-built sequences including bad ones, and a stable
      `OPS_HASH` (it changes iff the table changes).

## Notes
- The note is the spec. If something there turns out wrong in code, fix the note in the
  same branch and log it.
- Keep it compact: the table is the single source of truth, so the interpreter's dispatch
  in 0015 should be able to match on the generated enum directly.

## Log

## Follow-ups

## Result
