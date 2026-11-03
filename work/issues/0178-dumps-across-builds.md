# 0178: Continuation dumps across builds

status: open
agent: -
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

## Follow-ups

## Result
