# 0048: P3 TypeScript: type skipper, runtime TS constructs, strip-types oracle

status: in-progress
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

## Follow-ups

## Result
