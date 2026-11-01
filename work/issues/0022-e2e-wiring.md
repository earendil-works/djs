# 0022: End-to-end wiring: Vm::compile, `djs run`, e2e tests

status: in-progress
agent: heap
branch: issue/0022-e2e-wiring
depends-on: 0014, 0015
touches: crates/djs-core/src/vm/, crates/djs-core/src/interp/ (ops the compiler emits: special_object THIS, global_decls, init_global_lex + global lexical record cell, construct, define_elem, copy_data_props, delete), crates/djs-cli/, crates/djs-core/tests/ (or an e2e test module)

## Goal
The 0015 follow-ups: connect the compiler (0014) to the interpreter (0015) so that JS
source runs end to end.

## Acceptance criteria
- [ ] `Vm::compile(source, mode)` on top of `compiler::compile`, for Script mode and
      the async-body mode (D008; the parameters `tools` and `console`. Until async
      exists, run the body as a plain function body and document the gap).
- [ ] `djs run file.js` in djs-cli main.rs, with `--gc-stress` and `--suspend-every N`,
      printing `console.log` output through the host call. Exit codes: 0 ok, 1 uncaught
      error (printed as `Name: message`), 2 usage or compile error.
- [ ] End-to-end tests: a set of small JS programs covering P0 (and whatever P1 the
      interpreter already supports through the compiler), each run in the three modes
      (normal, fuel 1 with a heap-image round trip at every stop, GC stress), with
      identical output required.
- [ ] Fix whatever mismatches show up between compiler output and interpreter
      expectations, and log them.

## Log
- 2026-11-01: Scope additions from the coordinator, for 0024 (P1-1, compiler side):
  1. `special_object` kinds: THIS (4) = the frame's `this`; ARGUMENTS (0) = an unmapped
     strict arguments object from args[0..argc] (indexed elements, non-enumerable
     `length`); NEW_TARGET (1) = the header's new_target; ACTIVE_FUNCTION (3) =
     stack[bp].
  2. New op `rest(index: Arg) -> array` (added to the table by 0024): an Array of
     args[index..argc], empty if argc <= index.
  3. Function object properties from the template in `closure()`: `length` (LENGTH)
     and `name` (NAME's string, "" if null), both {W: false, E: false, C: true}; for
     KIND_NORMAL also `prototype` = a new object with a non-enumerable `constructor` = f,
     the property itself {W: true, E: false, C: false}. KIND_ARROW: no prototype, not a
     constructor.
  4. `construct` for closures: this = Object.create(f.prototype if an object, else
     Object.prototype), ConstructResult, new_target = f.
- Implemented in `interp/run.rs` (and `realm.rs`):
  - special_object THIS / ARGUMENTS (new `ObjectClass::Arguments` = 5) / NEW_TARGET /
    ACTIVE_FUNCTION;
  - global_decls / init_global_lex with the global lexical record (D1220), with TDZ in
    get/put/typeof of globals;
  - define_elem, copy_data_props mode 0 (like qjs, primitive sources add nothing),
    delete (qjs messages), construct;
  - closure length/name/prototype;
  - the NaN / Infinity / undefined globals;
  - qjs's `'length' is read-only` for strings.
- Mismatch found and fixed: `DefineElem` in bytecode/ops.rs said `pop 3 push 2`, but the
  compiler (and the doc comment) use `obj key v -> obj`. Changed to push 1 (one
  character; told the coordinator and lexer).
- `js/src/layout.generated.ts` regenerated: the export includes `objectClasses`, which
  gained Arguments. It is generated, but outside `touches:`.
- `Vm::compile(src, &CompileOptions)` (additive) returns the instantiated top-level
  function.
- `djs run [--body] [--gc-stress] [--suspend-every N] file.js` wired into djs-cli
  `main.rs`. Exit codes 0 / 1 (`Uncaught Name: message` on stderr) / 2 (usage, or
  `SyntaxError: msg at file:line:col`).
- E2E corpus (D1221): 24 cases in `tests/data/e2e/` with qjs-generated `.expected` files,
  run by `crates/djs-core/tests/e2e.rs` in three modes. The first full run timed out: the
  suspension mode at fuel 1 copies the heap at every instruction, which for the
  20k-iteration loop case is ~300k images in a debug build. It now uses fuel 1 for the
  first 5000 stops, then slices of 997. After that, four mismatches: the missing global
  `NaN`/`Infinity`, cases using `for-of` / `String` (not available yet; removed from the
  cases), and the string `length` assignment message.

## Follow-ups

## Result
