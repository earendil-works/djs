# 0010: Differential corpus and difftest runner (oracle side)

status: open
agent: -
branch: issue/0010-difftest-corpus
depends-on: 0002
touches: crates/djs-difftest/, tests/corpus/, scripts/difftest-*, TESTING.md (difftest how-to section only)

## Goal
TESTING.md layer 2: compare djs against dquickts on behavior that test262 does not check
(`console.log` formatting, error messages, uncaught error reporting, return values,
TypeScript, codemode-style scripts). This issue builds the corpus format, the runner and
the qjs side, so that djs can plug in as soon as 0006 lands (the same `Engine` idea as
djs-test262).

## Acceptance criteria
- [ ] The codemode execution model, defined in TESTING.md: a case is the body of an async
      function, run strictly. Observable outputs: the stdout lines (`console.log`, and
      `text()`/`print` if we define them), the return value (formatted the way pi's
      codemode would present it; look at how pi wraps dquickts if that is findable,
      otherwise define it and record a decision), and an uncaught error (type +
      message).
- [ ] Corpus format: `tests/corpus/<area>/<name>.{js,ts}` with `<name>.expected` next to
      it. Optional per-case metadata in a leading comment (for example `// difftest:
      skip-djs reason`, `// difftest: ts`, `// difftest: fuel 1000000`).
- [ ] `crates/djs-difftest` (std, binary `djs-difftest`): runs the corpus with an engine,
      compares the result with `.expected`, prints a summary plus diffs for failures,
      `--update` regenerates `.expected` from the qjs engine (never from djs), runs in
      parallel, and has a ratchet file like test262's
      (`tests/corpus/expected-failures.txt`).
- [ ] A `qjs` engine: wraps the case in the codemode model (an async IIFE or the
      equivalent; `--typescript` for `.ts`), with a wall-clock timeout.
- [ ] An initial corpus:
  - the dquickts tests `test_language.js`, `test_builtin.js`, `test_closure.js`,
    `test_loop.js` and `test_typescript.ts`, split into one case per test function by a
    checked-in script (`scripts/difftest-import-dquickts.*`). Each case includes the
    needed `assert` helpers;
  - hand-written cases for `console.log` formatting of every value kind (primitives,
    -0, NaN, strings with quotes, nested objects and arrays, cycles, functions,
    classes, symbols, errors, Map/Set, dates, sparse arrays, long arrays/strings and the
    truncation rules qjs applies), error messages for common TypeErrors and
    ReferenceErrors, and the uncaught error output.
- [ ] Every case passes against qjs (it is the oracle, so failures there mean the case
      or the runner is wrong).
- [ ] TESTING.md: a "How to run difftest" section.

## Notes
- Do not wire djs in yet (that waits for 0006). Do not add difftest to the gate yet.
- Codemode specifics (the `tools` object, `text()`, `image()`, `store`/`load`): define
  minimal fake versions in a prelude only if cheap. Harvested pi codemode scripts are a
  separate later step (pending a decision on what may be committed).
- Reuse code from djs-test262 by copying or by a small shared module. A shared crate is
  fine if it stays small; record it.

## Log

## Follow-ups

## Result
