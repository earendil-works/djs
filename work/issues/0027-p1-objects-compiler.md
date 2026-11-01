# 0027: P1-2 Objects, arrays and privileged mode (compiler side)

status: done
agent: lexer
branch: issue/0027-p1-objects-compiler
depends-on: 0025
touches: crates/djs-core/src/compiler/, docs/compiler.md (corrections), crates/djs-core/src/bytecode/ (additive only)

## Goal
docs/compiler.md section 10, row P1-2, compiler half: everything about object and array
literals and property operations that still says "not supported yet", plus the
**privileged mode** from 7.3 (`%Name(args)` intrinsic calls compiled to
`call_intrinsic`), which the hand-written/bgen prelude needs (D1172: the bgen output
format, `builtins/gen/prelude.js`). The runtime half (0028: the `call_intrinsic`
dispatch, prelude installation, ToPrimitive slow paths) follows in interp/ by the heap
agent.

## Acceptance criteria
- [x] Object literals: methods (including `super` placeholders? No: that is classes,
      P1-7; methods only need a home object if `super` is used, so reject `super` in
      object methods for now), getters/setters, computed keys (with runtime function
      naming, a 0024 follow-up), shorthand, `__proto__: x` in literals (the spec's
      non-Annex-B part: a literal `__proto__` key sets the prototype), and spread
      properties.
- [x] Spread arguments in calls and `new` (`f(...a)`); spread in array literals already
      works. If iteration is needed for non-arrays, emit the iterator path per the note,
      which the interpreter may report as "not supported yet" until P1-5.
- [x] `in`, `delete` in all forms, and `instanceof` (the compiler side is probably done;
      check it).
- [x] Privileged mode: a compile option (not reachable from user code) that enables
      `%Name(args)` in primary-expression position, compiling to `call_intrinsic id,
      argc`. Intrinsic ids come from a single table shared with the runtime (bgen's
      list of intrinsic names, from 0017's follow-up: emit it, or a Rust table generated
      from `builtins/intrinsics.d.ts`; coordinate the format in the Log). Compile
      `builtins/gen/prelude.js` successfully in privileged mode as a test.
- [x] e2e programs (checked against qjs). Cases needing 0028 go into an ignored `_0028`
      test.
- [x] The test262 compile sweep before/after.

## Log

- 2026-11-01, **intrinsic id table (the contract with 0028)**:
  `crates/djs-core/src/bytecode/intrinsics.rs`.
  - `INTRINSICS: &[IntrinsicInfo]`; **the id is the index** (u16, the first immediate
    of `call_intrinsic id, argc`). `IntrinsicInfo { name, kind, params, variadic }`.
  - Order: every `declare function` of `builtins/intrinsics.d.ts` in declaration order
    (kind `Leaf`/`Reentrant` from the `@leaf`/`@reentrant` tag), then every builtin
    tagged `@intrinsic` in `builtins/*.ts` (kind `Builtin`), files sorted by name,
    declaration order within a file (the `gen/prelude.js` order). Currently 41 entries:
    ids 0-37 Rust (IsObject = 0 ... InstallIntrinsic = 37), 38 ToPrimitive, 39 ToNumber,
    40 ToString.
  - `intrinsic_id(name)`, `intrinsic(id)`, `INTRINSICS_HASH` (fold it into the build
    hash next to `OPS_HASH`).
  - The test `table_matches_sources` parses the .d.ts and the .ts files and fails, with
    the replacement Rust printed, when the table is stale, so the gate catches additions.
  - Runtime contract: `Leaf`/`Reentrant` ids dispatch to Rust (argc on the stack, as
    for `call`: `a1 .. an -> r`, `pop 0 + imm 1`). `Builtin` ids call the JS function
    that `%InstallIntrinsic(name, fn)` registered for that name (a realm table indexed
    by id); calling one before it is installed is an internal error. `params`/`variadic`
    allow an arity check in debug builds; the compiler checks the arity at compile time
    (wrong argc is a SyntaxError in privileged code).

- Part 1, merged early so that 0028 builds against main:
  - the intrinsic table;
  - `CompileOptions { privileged: bool }` (constructor `CompileOptions::privileged_script`),
    which enables `%Name(args)` → `call_intrinsic id argc` with compile-time name and
    arity checks; the disassembler prints `%Name`;
  - `builtins/gen/prelude.js` compiles in privileged mode (`prelude_compiles`);
  - spread arguments (`f(...a)`, `o.m(...a)`, `f?.(...a)`, `new F(...a)` →
    `array_from k; array_spread/array_push; call_spread/construct_spread`);
  - new ops `set_proto` and `set_function_name prefix`, whose emitters come in part 2.
  Touched `crates/djs-wasm/src/lib.rs` (outside `touches:`) by one line: its
  `CompileOptions` literal needs `privileged: false`.

- Part 2:
  - Object literal methods, getters/setters, and computed keys with runtime naming
    (`set_function_name`), D1271.
  - `__proto__: v` → `set_proto`. Duplicates are an early error except in patterns,
    which the test262 sweep caught.
  - `delete` of optional chains; `super` in methods rejected as not supported yet.
  - `in`/`instanceof` were already right. `delete` now works in all forms.
  - Snapshot `objects.js`, about 12 new error cases (qjs positions), `end_to_end_objects`
    (11, passing now) and the ignored `end_to_end_objects_0028` (14, needs 0028). All
    are checked against qjs (`work/scratch/0027/qjs_check.py <test fn>`).
- test262 compile sweep: 24,742 → 25,999 compile; "not supported yet" 25,786 → 24,330.
  No new missing early errors.

## Follow-ups

- 0028 (heap agent): implement `call_intrinsic` (D1270), `define_method[_elem]`,
  `define_getter[_elem]`, `define_setter[_elem]`, `set_proto`, `set_function_name`,
  `array_spread` (iterables), `call_spread`, `construct_spread`; then un-ignore
  `compiler::tests::end_to_end_objects_0028`.
- `super` in object methods (home objects) with classes, P1-7.
- `delete (a?.b)` (parenthesized chain) yields true without deleting; the spec deletes.
  Rare.

## Result

Part 1 (merged early, ac51f50): the intrinsic id table (D1270), privileged mode
(`CompileOptions { privileged: true }`: `%Name(args)` → `call_intrinsic`; the
generated prelude compiles), and spread arguments.

Part 2: object literals are complete except for `super`: methods, getters/setters,
computed keys with runtime function naming, shorthand, `__proto__:`, spread; `delete`
in all forms (D1271).

**Un-ignore `compiler::tests::end_to_end_objects_0028` when 0028 lands.**
test262 compile sweep: 24,742 → 25,999 compile; "not supported yet" 25,786 → 24,330.
