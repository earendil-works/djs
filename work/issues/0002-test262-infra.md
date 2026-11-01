# 0002: test262 infrastructure and oracle baseline

status: in-progress
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

## Follow-ups

## Result
