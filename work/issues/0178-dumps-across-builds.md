# 0178: Continuation dumps across builds

status: in-progress
agent: portable
branch: issue/0178-dumps-across-builds
depends-on: 0172
touches: tests/dump/v2/, crates/djs-core/src/vm/dump_tests.rs, scripts/ (the two-commit check), TESTING.md
guide: -

## Goal
Hold the format 2 promise (D2560): every later build loads a format 2 dump and continues, or
refuses with a reason.

## Acceptance criteria
- [ ] Golden format 2 fixtures written by the exporter (one per role sequence, loop kind,
      re-enactment and class case; small), committed and loaded by every gate run with the
      results delivered and the output compared.
- [ ] A script that builds an older commit (git archive), exports at every idle point of the
      harvested cases there, and imports and finishes them on the current build; documented
      in TESTING.md as a periodic check, with its first results in the issue.
- [ ] The rule for changing a fixture written down (only with a decision, as for v1).

## Log
- 2026-11-04 (portable): the two-commit check: scripts/dumpx/main.rs (a standalone tool,
  built twice by scripts/dumps-across-builds.sh OLD in target/dumpx: against `git archive
  OLD` to export a dump at every idle point of codemode + codemode-harvest with the
  transcript prefix and the full run; against this checkout to import, finish and
  compare). Needs the dump API of 0172+. First run: OLD=23835fa9 (0172's merge), import
  with 59ca2f38: 1,194 dumps, 1,194 finished the same, 0 refused.
- Waiting for 0180 (marker `"djs-dump 2";`) before writing the golden fixtures.

## Follow-ups

## Result
