# 0048: P3 TypeScript: type skipper, runtime TS constructs, strip-types oracle

status: done
agent: lexer
branch: issue/0048-typescript
depends-on: 0040
touches: crates/djs-core/src/compiler/ (the TS grammar behind the `ts` flag), crates/djs-core/src/lexer/ (`rescan_less`, the 0003 follow-up), crates/djs-cli/ (`djs strip-types`), crates/djs-difftest/ (.ts cases on djs), crates/djs-wasm + js/src/sandbox.ts (a `typescript` compile flag), tests/corpus/ts/, TESTING.md (the strip-types oracle how-to), vendor/ (ts-blank-space + a TS corpus, gitignored, fetched by a script)

## Goal
ARCH.md "TypeScript" and phase P3. djs replaces dquickts, which runs TypeScript (tsc
semantics with `isolatedModules` and `useDefineForClassFields`), and codemode scripts are
typically written against TypeScript declarations of the tools. The type skipper erases
types during parsing, token by token. Constructs with runtime meaning (enums, namespaces,
parameter properties) compile to bytecode. There is no separate TS mode in the core: a
flag enables the TS grammar.

## Acceptance criteria
- [ ] The type skipper: annotations on bindings, parameters, return types and class
      members (incl. `!` definite assignment, `?` optional), type parameter lists on
      functions/classes/methods/arrows (the generic arrow `<T>(x: T) => x` ambiguity in
      .ts), type argument lists on calls and `new` (`f<T>(x)` vs comparisons, using tsc's
      rules from docs/compiler.md 3.9 and a checkpoint/rewind), `as` / `satisfies` /
      `!` (non-null), `interface`, `type` aliases, `declare` (vars, functions, classes,
      modules, global), overload signatures, `abstract` classes/members, accessibility
      modifiers, `readonly`, `override`, index signatures, `implements`, `import type` /
      `export type`, `this` parameters, assertion functions, `enum` declarations with
      `declare`/`const` (`const enum` under isolatedModules: treat like a regular enum,
      as tsc does), and `unique symbol`. Angle-bracket assertions `<T>x` are not allowed
      in .ts with JSX off? Check dquickts/tsc behavior and match it.
- [ ] Runtime constructs: `enum` (numeric with auto-increment and reverse mapping, string,
      computed initializers referencing earlier members, merging of multiple
      declarations), `namespace`/`module` (nested, merged with functions/classes/enums,
      exported members), and parameter properties (`constructor(private x: number)`,
      with `useDefineForClassFields` ordering, matching tsc).
- [ ] Legacy decorators (`test_decorators_legacy.ts` in dquickts, under a flag like qjs's
      `--experimental-decorators`): decide whether they are in scope now (codemode
      rarely uses them); a follow-up is fine.
- [ ] `djs strip-types file.ts`: prints the source with type ranges replaced by
      whitespace (ts-blank-space style), which is the parse-only oracle from TESTING.md.
      A script fetches `ts-blank-space` (npm, pinned) and a TS corpus (for example
      TypeScript's own `src/` at a pinned tag) into vendor/. A runner compares
      byte-for-byte on every file that ts-blank-space accepts. Report the match rate and
      the failure clusters. The gate gets a fast subset; the full run is a script.
- [ ] Early errors that tsc would report as syntax errors in isolatedModules mode (for
      example `enum` in a `declare` context initialized with non-constants) as far as
      dquickts reports them. Type errors are not checked (erasure only).
- [ ] difftest: the .ts cases run on djs (tests/corpus/ts/, the dquickts
      test_typescript.ts import), and the ratchet updated. The wasm host and Sandbox get
      a `typescript: true` compile option.
- [ ] Error positions in TS sources stay correct (the line table follows the original
      source).

## Notes
- Land in parts: the skipper for annotations, `as`, interfaces and type aliases first
  (most codemode scripts), then generics/ambiguities, then enums/namespaces/parameter
  properties, then the strip-types oracle.

## Log
- Part 1 (type skipper): compiler/ts.rs ports dquickts' `ts_*` skipper and
  disambiguation (D1480): annotations everywhere, type params/args, generic calls vs
  comparisons, arrows with return types (incl. the `a ? (b) : c => d` rule), generic
  arrows, `as`/`satisfies` precedence, `<T>x`, `x!`, `type`, `interface`, `declare`,
  overload signatures, `abstract`, class member modifiers, index signatures, `this`
  parameters. `Lexer::rescan_less`. Erased ranges recorded for `compiler::strip_types`
  / `djs strip-types` (D1481, simple blanking for now). `djs run --ts` (default for
  .ts files); the difftest djs engine passes `job.ts`. Tests: compiler/ts_tests.rs (37
  e2e cases checked with dquickts' qjs, errors, strip). difftest: 10 TS cases pass now.
  JS fix on the way: `function f(a b)` says `expecting ','` like qjs.
- ts-blank-space's details (read from 0.9.0's source) that strip-types must emulate
  later: `;` in place of the first blanked char of a type-only statement when the last
  visited JS node did not end with `;` (reset at function bodies), `;` for an `as`
  that ends its statement without `;`, `(`/`)` moves for generics / arrow return types
  that span lines, `this` params blanked with their trailing comma. The corpus files
  are modules, so strip mode must also parse `import`/`export` declarations.

- Part 2 (runtime constructs, compiler/ts_decl.rs): enums (constant evaluation,
  auto-increment, reverse mapping, string members, merging, block scoping), const enums
  (constant-only, folded at resolution, D1483), namespaces (nested, `A.B.C`, merged,
  merged with functions/classes/enums, exported vars as members, type-only namespaces
  emit nothing, `export declare`), `import X = A.B` (and `export import`), parameter
  properties (D1484). D1482: members resolve at resolution time. All dquickts TS
  difftest cases pass except decorators (test_rejected_syntax_5/6) and `using`
  (test_using*), which djs does not have in JS either. JS fixes: `function f(private)`
  says `missing formal parameter` like qjs.

- Part 3 (strip-types oracle): `djs strip-types [--compare]`, scripts/strip-types/
  (fetch.sh: ts-blank-space 0.9.0 + TypeScript 5.9.3, the compiler's src/ and
  tests/cases/conformance; oracle.sh; update-cases.sh), strip mode parses module syntax
  (compiler/module.rs) and emulates ts-blank-space's `;`/`(`/`)` insertions. Result:
  4053/4802 compared files match byte for byte, 0 differ, 749 are djs syntax errors
  (decorators, `using`, `accessor`, and the conformance tests' deliberate JS errors).
  Gate: tests/strip-types/ (compiler test `ts_strip_corpus`).
  Found through the corpus and fixed: `class implements I {}`, `declare class X extends
  Mixin(B)`, tsc's `allowReturnTypeInArrowFunction` propagation (`x ? y => ({ y }) : z
  => ({ z })` was rejected, also by dquickts), `import type from from`, `{ type as }`.
- Part 4: djs-wasm `vm_compile` goal bit 2 = TypeScript, `SandboxOptions.typescript`,
  difftest case ts/codemode-tools.ts.

## Follow-ups
- Decorators (legacy, `experimentalDecorators`, test_decorators_legacy.ts, and TC39
  ones, test_rejected_syntax_5/6): not in scope here; djs has no decorators in JS
  either (ARCH lists them later). They are the largest cluster of strip-types errors.
- `using` / `await using` (test_using*.ts) and `accessor` fields: missing in djs's JS too.
- js/pi/engine.ts could compile codemode with the TypeScript bit (`goal | 2`) if pi's
  codemode scripts are TypeScript, as with dquickts; that is the pi adapter's call.
- tsc parses the body of an arrow in a conditional's `b` and then checks for `:`;
  djs (like dquickts) skips tokens, which can cross a statement boundary
  (`a ? (b) : c => (d) : e => f` followed by another conditional). Rare.
- Module syntax exists only in strip mode (djs has no modules); `import type` in a
  codemode script is a SyntaxError on both engines.

## Result
TypeScript behind `CompileOptions::typescript` (`CompileOptions::ts()`), `djs run --ts`
(default for .ts), the difftest .ts cases, djs-wasm goal bit 2 and
`SandboxOptions.typescript`:
- compiler/ts.rs: the type skipper and all disambiguation, ported from dquickts (D1480):
  annotations everywhere, type parameters/arguments, generic calls vs comparisons,
  arrows with return types and generic arrows, `as`/`satisfies`/`!`/`<T>x`, `type`,
  `interface`, `declare`, overloads, `abstract`, member modifiers, index signatures,
  `this` parameters, assertion functions, `unique symbol`; dquickts' messages and
  positions.
- compiler/ts_decl.rs: enums (constant evaluation, reverse mapping, merging, const enums
  checked and folded, D1483), namespaces (nested, merged, merged with functions/classes,
  exported vars as members resolved after parsing, D1482), `import X = A.B`; parameter
  properties in class.rs (D1484).
- strip-types: compiler::strip_types, `djs strip-types [--compare]`, the oracle scripts
  in scripts/strip-types/ (D1481, TESTING.md "How to run the strip-types oracle"):
  4053/4802 files of TypeScript's src and conformance tests match ts-blank-space, none
  differ.
- difftest: every dquickts TypeScript case passes except decorators and `using`;
  tests/corpus/ts/ all pass.
Known gaps: see Follow-ups.
