# 0010: Differential corpus and difftest runner (oracle side)

status: done
agent: infra
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

2026-11-01 (infra)
- How pi presents a script: pi codemode (pi-private `packages/codemode`) runs
  quickjs-wasi with a JS prelude that replaces `console` (strings raw, Errors as
  name/message/stack, everything else JSON.stringify) and serializes the return value in
  the VM; `packages/durable/src/codemode/output.ts` shows strings raw and other values as
  compact JSON. qjs's native console.log formatting is never shown to the model. After a
  note to the coordinator, decided: difftest compares exactly what pi shows (D1100).
- `tests/corpus/prelude.js`: shared prelude with pi's formatting functions (copied),
  `text`, `exit`, `store`/`load`, fake `tools.echo`/`tools.fail`, and the pi stall error.
  No lockdown, no tools Proxy, no `image()` (D1100).
- `crates/djs-difftest`: case discovery and directives, `Engine` trait, transcript
  rendering and normalization (stack frame lines dropped), qjs engine (one process per
  case, driver module with `std.evalScript`, `typescript` option for .ts, wall-clock
  timeout, qjs-only globals such as `print` deleted), LCS line diffs, `--update`
  (qjs only; refuses timeouts and crashes), the ratchet (`--ratchet`,
  `--update-ratchet`, `tests/corpus/expected-failures.txt`, currently empty). It reuses
  djs-test262's library for par/ratchet/glob/table (D1101).
- Hand-written corpus (69 cases): `console/` (the JSON-style output of every value kind:
  primitives, -0/NaN, strings, nesting, key order, dropped values, toJSON, dates, cycles
  and the String() fallback, functions/classes, collections, sparse arrays, errors,
  `text()`), `result/` (return values, unserializable values, exit()), `errors/`
  (uncaught errors of every kind, syntax errors, the stall, ~70 common TypeError /
  ReferenceError / RangeError / JSON.parse messages), `codemode/` (tools, store, typical
  scripts, the visible globals, `this`/`arguments`), `ts/`.
- Finding: `String([cyclic array])` overflows the stack in qjs (its join has no cycle
  detection; V8 prints ""). It is its own case, `console/cyclic-array.js`, so djs can
  decide whether to match.
- `scripts/difftest-import-dquickts.py` (Python stdlib only): splits the five dquickts
  files into 94 cases: 76 test functions plus `closure/toplevel.js`, and 18 parse-error
  cases from `assertSyntaxError` lines. 17 test functions are excluded with reasons
  (sloppy mode 8, qjs std/os 5, eval'd stack positions 1, modules 1, evalTS-only 1, and
  one assertSyntaxError that is valid in a function body). 11 cases use eval/Function and
  carry `skip-djs`. Every imported test function completes and every syntax case throws a
  SyntaxError with qjs (checked).
- All 163 cases pass with qjs, also under other time zones and locales; the full run
  takes about 0.3 s. Runner tests: `crates/djs-difftest/tests/runner.rs` (scratch corpus
  with qjs: pass, diff, missing .expected, timeout, ignored stack frames, TS; fake engine:
  skip-djs, panics, filters, .expected collisions).
- TESTING.md: besides the "How to run difftest" section I added a short "The codemode
  model" subsection and changed `case.js.expected` to `case.expected` in the intro of
  "Differential tests", since the issue asks for the model to be defined there.

## Follow-ups
- 0006 / djs: implement `Engine` for djs: evaluate `tests/corpus/prelude.js` as a strict
  Script, call it with a host object (`output`, `done`), compile the case as a strict
  async arrow body, call `api.run(fn)`, and call `api.stalled()` when the VM goes idle.
  Then create the ratchet with `--engine djs --update-ratchet` and add
  `djs-difftest --engine djs --ratchet` to the gate.
- Harvested pi codemode scripts with recorded tool results (pending the decision on what
  may be committed). The fake `tools` in the prelude would then replay recordings.
- Stack traces are not compared (D1100); revisit when djs has stack traces.
- Open question for later: whether djs's own codemode console should be the pi prelude
  (JS) or native code with the same formatting.

## Result
- `crates/djs-difftest` (binary `djs-difftest`): runs `tests/corpus` in the codemode
  model with an engine (qjs now), compares transcripts with `.expected`, prints a summary
  table and diffs, `--update` records from qjs only, `--ratchet` / `--update-ratchet`
  with `tests/corpus/expected-failures.txt`, parallel. Usage: TESTING.md, "How to run
  difftest".
- `tests/corpus/prelude.js`: the shared codemode prelude (pi's formatting).
- Corpus: 163 cases (69 hand-written, 94 imported from dquickts), all passing with qjs.
- `scripts/difftest-import-dquickts.py`: regenerates `tests/corpus/dquickts/`.
- Decisions D1100 to D1102.
- Known gaps: no djs engine yet; stack frames are not compared; no harvested pi scripts;
  the prelude leaves out pi's lockdown, tools Proxy and `image()`.
