# 0013: Bytecode table and tools

status: done
agent: lexer
branch: issue/0013-bytecode-table
depends-on: 0005, 0008
touches: crates/djs-core/src/bytecode/, crates/djs-core/src/lib.rs (mod lines only), crates/djs-cli/ (disasm subcommand only)

## Goal
docs/compiler.md section 4 (and 6.1/6.2 where the code layout touches the heap): the
single `ops!` instruction table and everything derived from it. This comes first because
the compiler (0014) and the interpreter (0015) both build on it, and after it lands they
can proceed in parallel.

## Acceptance criteria
- [x] The `ops!` macro defines every P0 and P1 opcode from compiler.md 4.5 (name,
      immediates and their widths, stack effect, fuel cost class, flags such as "may
      throw" and "may call", whether it is scope-generic and rewritten by resolution).
- [x] Derived from it: the opcode enum, an encoder/emitter API (a `CodeBuf` with typed
      `emit_*` helpers, labels and fixups per 4.3), a decoder/iterator, a disassembler
      (readable text with atoms and constants resolved when a heap is given), the stack
      depth checker (4.4), and `OPS_HASH` (fed into the snapshot build hash together with
      the heap's `LAYOUT_HASH`).
- [x] The cell kinds the compiled code needs (function template, script, bytes,
      ref-array per compiler.md 6.1) declared in `heap/cells.rs`, if that can be done
      without the internal-ref tag (D007). Otherwise document what 0015 must add.
      Coordinate: keep changes to `heap/` minimal and additive (0012 is working there).
- [x] The line table encoding (6.2) and the handler table encoding (7.4) as plain data
      structures with encoder and decoder plus unit tests.
- [x] `djs disasm <file.djsbc>` is optional. A test-only disassembler on hand-built code
      is enough.
- [x] Unit tests: round trips (emit → decode → disassemble), label fixups (forward and
      backward), stack depth on hand-built sequences including bad ones, and a stable
      `OPS_HASH` (it changes iff the table changes).

## Notes
- The note is the spec. If something there turns out wrong in code, fix the note in the
  same branch and log it.
- Keep it compact: the table is the single source of truth, so the interpreter's dispatch
  in 0015 should be able to match on the generated enum directly.

## Log

- 2026-11-01: `crates/djs-core/src/bytecode/`:
  - `ops.rs`: the `ops!` table, 147 opcodes (every P0/P1 op of compiler.md 4.5, plus
    `require_object_coercible` and the `define_*_elem` variants, which the note was
    missing). Generated: `Op`, `OPS`/`OpInfo`, typed emitters, `OPS_HASH`. Compile-time
    checks: a `Target` is the last immediate and only on jumps; `pop_imm` is in range.
    Also `special` (special_object kinds) and `capture_desc` (CAPTURE_DESC words), which
    both 0014 and 0015 need.
  - `imm.rs`: immediate kinds, widths, and the marker types that map them to emitter
    argument types (`Loc` → u16, `Atom` → heap `Atom`, `Target` → `Label`, ...).
  - `code.rs`: `CodeBuf<Phase1|Final>`, labels, fixups, `emit_insn` (the re-emit's copy
    path), `last_insn`/`truncate_last` (lvalue conversion; refuses across a label),
    errors for unplaced/duplicate/unknown labels.
  - `decode.rs`: `Insn`, `decode`, `insns`, and raw `read_*` helpers for the dispatch
    loop.
  - `stack.rs`: the checker (linear decode + worklist; gosub/ret, handlers; underflow,
    mismatch, bad targets, falling off the end, phase-1 ops).
  - `lines.rs`, `handlers.rs`: the encodings, with builder/view/lookup.
  - `disasm.rs`, `template.rs`: the table-driven disassembler, `alloc_bytes`,
    `alloc_ref_array`, `alloc_script`, `alloc_template` (runs the stack checker, sets
    `STACK`), and `disasm_template` (a function tree with names, constants and
    handlers).
- `heap/cells.rs`: added kinds 12-16 (FuncTemplate, Script, BoxCell, Bytes, RefArray),
  additive only; nothing else in heap/ changed (walk/GC handle new kinds generically).
  `Box` became `BoxCell` to avoid shadowing `alloc::boxed::Box`. The handler table is a
  `Bytes` cell rather than the U32Array the note proposed (same bytes, read in place
  like the code).
- docs/compiler.md synced: the 4.2 table syntax, the 4.3 emitter modes, the 4.5 additions,
  the 6.1 HANDLERS/BoxCell/RESERVED fields. Decisions D1130 (table and emitter API) and
  D1131 (code cells).
- No `djs disasm` subcommand: there is no bytecode file format, and once 0014 compiles,
  `djs disasm file.js` is more useful. Test-only disassembly covers it (an exact listing
  in `template_cells`).
- Tests (18): table consistency, every opcode emitted/decoded/listed, typed emitters
  and raw readers, decode errors, forward/backward fixups, label errors, a phase-1 →
  final re-emit, truncate_last, the stack checker on good and bad code (including the
  9.3 try/finally), OPS_HASH sensitivity to every field and to order, line and handler
  tables, code cells (verify + byte-copy round trip), capture descriptors.

## Follow-ups

- 0015: add the internal-ref Value tag (D007) before putting templates or Boxes in
  object slots. Then add a GC test that templates survive a collection with their
  parts (they cannot be rooted yet).
- 0014 defines the `FuncTemplate::FLAGS` bits (function kind, async, ...); a good place
  is `bytecode/template.rs`, next to `TemplateParts`.
- Once heap images contain bytecode (snapshots, P4), `Roots::VERSION` should hold
  `bytecode::build_hash(LAYOUT_HASH, OPS_HASH)` instead of `LAYOUT_HASH`.
- `djs disasm <file.js>` once the compiler exists (0014).
- Later optimizations: short opcode/jump forms (with a relaxation pass in `finish`), a
  precomputed fuel table if profiling shows `OPS[op].cost` lookups.

## Result

`djs_core::bytecode` is the single instruction table and everything derived from it.

- Emit: `let mut b = CodeBuf::new();` (final) or `CodeBuf::phase1()`. Then `b.get_loc(0)`,
  `b.call(2)`, `let l = b.new_label(); b.jump_if_false(l); ...; b.place_label(l)`,
  `b.finish()?` → `Code { code, labels }` / `Phase1Code { code, label_count }`. The
  re-emit uses `CodeBuf::with_labels(label_count)`, `emit_insn` and `place_label`.
- Decode: `decode(code, pc)`, `insns(code)`, `Insn::{op, imm, imm_i32, atom, label,
  jump_target, pops, pushes}`. The interpreter matches on `Op::from_u8(code[pc])` and
  reads immediates with `read_u16(code, pc + 1)` and friends; `Op::X.size()` is const.
- Check: `check_stack(code, handlers) -> Result<u16, StackError>`.
- Tables: `LineTableBuilder` / `lookup_line`, `encode_handlers` / `HandlerTable::find`.
- Heap: cells `FuncTemplate`, `Script`, `BoxCell`, `Bytes`, `RefArray`;
  `alloc_template(heap, &TemplateParts { .. })`, `alloc_script`, `bytes_of`,
  `disasm_template`.
- Hash: `OPS_HASH`, `build_hash`.

Known gaps: no CLI disassembler; templates cannot be GC-rooted until the D007 tag exists.
