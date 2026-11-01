# 0027: P1-2 Objects, arrays and privileged mode (compiler side)

status: in-progress
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
- [ ] Object literals: methods (including `super` placeholders? No: that is classes,
      P1-7; methods only need a home object if `super` is used, so reject `super` in
      object methods for now), getters/setters, computed keys (with runtime function
      naming, a 0024 follow-up), shorthand, `__proto__: x` in literals (the spec's
      non-Annex-B part: a literal `__proto__` key sets the prototype), and spread
      properties.
- [ ] Spread arguments in calls and `new` (`f(...a)`); spread in array literals already
      works. If iteration is needed for non-arrays, emit the iterator path per the note,
      which the interpreter may report as "not supported yet" until P1-5.
- [ ] `in`, `delete` in all forms, and `instanceof` (the compiler side is probably done;
      check it).
- [ ] Privileged mode: a compile option (not reachable from user code) that enables
      `%Name(args)` in primary-expression position, compiling to `call_intrinsic id,
      argc`. Intrinsic ids come from a single table shared with the runtime (bgen's
      list of intrinsic names, from 0017's follow-up: emit it, or a Rust table generated
      from `builtins/intrinsics.d.ts`; coordinate the format in the Log). Compile
      `builtins/gen/prelude.js` successfully in privileged mode as a test.
- [ ] e2e programs (checked against qjs). Cases needing 0028 go into an ignored `_0028`
      test.
- [ ] The test262 compile sweep before/after.

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

## Follow-ups

## Result
