# 0002: test262 infrastructure and oracle baseline

status: done
agent: infra
branch: issue/0002-test262-infra
depends-on: 0001
touches: crates/djs-test262/, scripts/fetch-test262.sh, tests/test262.toml, tests/test262/

## Goal
Build the test262 runner described in TESTING.md ("test262" section) and use it to
produce the oracle baseline with dquickts' `qjs`, before djs can run anything. Once djs
exists (issue 0007), the same runner, filters and harness handling run djs in-process.

## Acceptance criteria
- [ ] `scripts/fetch-test262.sh` clones test262 at the pinned commit (D003) into
      `vendor/test262`. It is idempotent, verifies the commit, and uses a shallow or
      partial clone where possible.
- [ ] `tests/test262.toml` holds the skip / later / active grouping from TESTING.md:
      flags, dir prefixes, feature globs, and a source scan for direct `eval(` and the
      `Function(` constructor (group `scan-eval`, skipped). Every group entry has a
      short comment saying why.
- [ ] New std crate `crates/djs-test262` (binary `djs-test262`):
  - [ ] a frontmatter parser for the YAML subset test262 uses (`description`,
        `includes`, `flags`, `features`, `negative`), unit tested on real samples,
  - [ ] test selection: a path/glob filter, groups, `list` (counts per group, per dir,
        per feature),
  - [ ] the execution model from TESTING.md: strict-only, the harness put in front,
        `includes`, `raw`, `onlyStrict`, negative tests by phase and type, `async` with
        `doneprintHandle.js`,
  - [ ] an `Engine` trait so the in-process djs engine plugs in later without changes to
        the selection or reporting code,
  - [ ] a `qjs` engine: `~/Development/dquickts/qjs` as a subprocess, with a wall-clock
        timeout and a small JS `$262` shim (`global`, `evalScript` via indirect eval,
        `detachArrayBuffer` via `ArrayBuffer.prototype.transfer`, `gc` as a no-op),
  - [ ] parallel execution on std threads (`-j`, defaulting to the number of CPUs),
  - [ ] a summary report (pass/fail/total for active and later; per top-level dir and
        per feature; and `--failures` to list failing tests with a one-line reason),
  - [ ] the ratchet: `--expected <file>` compares the run against a failures file
        (`path: reason` lines, sorted). It exits non-zero on new failures, reports stale
        entries, and `--update` rewrites the file.
- [ ] `tests/test262/oracle-failures.txt`: qjs under our runner, over active + later.
      Compare it with dquickts' `test262_errors.txt` and `run-test262` (read-only use)
      and explain the differences in the Log. Fix the runner where it is wrong. Leave a
      difference in place only when it is a real semantic difference of our execution
      model.
- [ ] A short "how to run" section added to TESTING.md (only that section).
- [ ] The full active + later run with qjs finishes in a reasonable time (record it).

## Notes
- Keep dependencies minimal. Hand-roll the frontmatter parser and the small TOML subset
  (or use a tiny crate if it really saves work, and note why). Use std threads, no rayon,
  unless you have a good reason.
- dquickts is read-only (D002). Reading `run-test262.c` for how it handles the harness,
  `$262`, negative tests and async tests is encouraged.
- Do not add test262 to `scripts/gate.sh` yet. That happens in 0007, when djs runs it.
- Design the reporting output so the coordinator can paste it into the board: a compact
  table.

## Log

2026-11-01 (infra)
- `scripts/fetch-test262.sh`: `git init` + `git fetch --depth 1 origin <sha>` + checkout
  into `vendor/test262` of the primary checkout (found via `git rev-parse
  --git-common-dir`, or `$DJS_TEST262`). Idempotent, verifies HEAD, refuses a modified
  checkout. A fresh fetch takes about 7 s.
- `crates/djs-test262` (no dependencies): frontmatter parser, TOML-subset parser, grouping,
  glob filters, source scan, `Engine` trait + shared `judge`, qjs engine, parallel runner
  (std threads), Markdown reports, ratchet.
- Frontmatter parser checked against PyYAML on all 53,597 test files: no differences in
  description, includes, flags, features or negative. Two odd files needed support: CR line
  terminators (`Function/prototype/toString/line-terminator-normalisation-CR.js`) and a
  frontmatter indented as a whole (`language/statements/function/13.2-30-s.js`).
  `tests/frontmatter_all.rs` (ignored test) keeps this check around.
- qjs engine: one process per test, `qjs -m -e <driver> <spec>`. The driver uses
  `std.evalScript(..., {compile_only: true})` and then runs the compiled script, so the
  phase of an error is exact (D1021). `$262.evalScript` is `std.evalScript` (real Script
  semantics) rather than indirect eval. The driver captures everything it needs before the
  test runs, because tests tamper with builtins (`tests/fixture` has a case for that).
- Because qjs passes nearly everything, a lenient runner would go unnoticed. The fixture
  suite (`crates/djs-test262/tests/fixture/test262`, run by `tests/qjs_fixture.rs`) has 12
  must-pass and 17 must-fail tests (parse vs runtime phase, wrong negative type, strict-only
  early errors, async never/twice/failure, harness errors, missing includes, timeouts,
  thrown primitives, tampered builtins, bad frontmatter) and checks each reason.
  `tests/engine_trait.rs` checks the prepared jobs (harness order, raw, async) and that an
  engine panic fails only that test.
- Baseline: qjs over active + later = 37,569 tests in 9.2 s (10.4 s wall including
  loading) with -j 18; 4 failures (`tests/test262/oracle-failures.txt`). A 256M memory
  limit changes nothing; the default is 512M.
- Comparison with dquickts: `test262_errors.txt` lists 41 failures, all in `intl402` and
  `staging` (both skipped here). I also ran `run-test262 -s` (strict mode) with a copy of
  `test262.conf` pointing at our checkout (in `work/scratch/0002/dq`, nothing written to
  dquickts): 30 failures, again all intl402/staging. Per test, against its report:
  - fixed in our runner: `ArrayBuffer/prototype/immutable/return-immutable.js` and
    `TypedArray/prototype/sort/sort-tonumber.js` failed because the first
    `$262.detachArrayBuffer` shim called `transfer()` on an already detached buffer.
    DetachArrayBuffer is a no-op there; fixed.
  - `harness/assert-throws-same-realm.js`, `harness/asyncHelpers-throwsAsync-same-realm.js`
    (and two TypedArray tests) use `$262.createRealm` without the `cross-realm` tag. Now
    skipped by a scan rule.
  - remaining differences (real, kept in oracle-failures.txt):
    - `Array/prototype/toLocaleString/invoke-element-tolocalestring.js`: without Intl the
      spec passes no arguments to element `toLocaleString`; qjs passes
      `(undefined, undefined)`. `run-test262` is built with Intl, so it takes the other
      branch of the test. djs has no Intl, so this is our model.
    - `language/identifier-resolution/assign-to-global-undefined.js`: excluded in
      dquickts' test262.conf (deliberately non-compliant).
    - `language/statements/class/{elements,subclass}/private-class-field-on-nonextensible-*.js`:
      feature `nonextensible-applies-to-private`, which dquickts' conf skips (not
      implemented in qjs). Kept active: it is current spec.
  - tests we run that `run-test262 -s` does not: the 26 active `raw` tests (it skips raw in
    strict mode; we run them unmodified, which is sloppy for qjs and strict for djs; the one
    raw test that needs sloppy mode, `hashbang/use-strict.js`, is skipped), and 4 tests that
    `run-test262` never runs because its test list dedups names that compare equal
    numerically (`escaped-bang-u0021.js` vs `escaped-bang-u21.js`,
    `non-octal-like-invalid-08n.js` vs `-0008n.js`, ...). qjs passes all of them.
- Group sizes (test262 @7ab7faf, 53,597 tests): skip 14,963, scan-eval 1,065, later
  17,445, active 20,124. The largest later reasons: async-iteration 4,192, generators
  3,852, RegExp 1,859, TypedArray 1,339, Iterator 651, Date 583, Reflect.construct 225
  (mostly via the isConstructor.js include in "not a constructor" tests).

## Follow-ups
- 0007: implement `Engine` for djs in-process (print collection, fuel → `Status::Timeout`,
  memory cap → `Status::OutOfMemory`; probably a fuel field on `Job`), create
  `tests/test262/expected-failures.txt` with `--update`, and add the ratchet run to
  `scripts/gate.sh`.
- Coordinator: review the later/active split in `tests/test262.toml` (D1022), in
  particular `built-ins/Date`, `Symbol` / `Symbol.iterator` (active) and the 225 tests that
  are later only because isConstructor.js implies Reflect.construct.
- `--suspend-every N` and `--backend` once the stress modes exist (TESTING.md).

## Result
- `scripts/fetch-test262.sh`: shallow fetch of the pinned commit into the shared
  `vendor/test262`.
- `tests/test262.toml`: skip / scan-eval / later groups, each entry commented (D1022).
- `crates/djs-test262` (binary `djs-test262`, no dependencies): `list` and `run`, path and
  glob filters, `--group`, Markdown summary per group / dir (`--depth`) / feature
  (`--features`), `--failures`, `--slowest`, `-j`, `--timeout`, `--memory-limit`, the
  ratchet (`--expected FILE [--update]`, D1023), and the `Engine` trait with a `qjs`
  engine (D1020, D1021). Usage is in TESTING.md, "How to run".
- `tests/test262/oracle-failures.txt`: 4 qjs failures over active + later, each explained
  in the Log.
- Full active + later run with qjs: 37,569 tests in about 10 s (-j 18).
- Known gaps: no djs engine yet (0007); the source scan is a heuristic (it can match
  inside strings or miss a pattern after a regex literal containing `//`); stale ratchet
  entries are reported but do not fail the run.
