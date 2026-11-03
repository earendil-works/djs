# 0178: Continuation dumps across builds

status: done
agent: portable
branch: issue/0178-dumps-across-builds
depends-on: 0172
touches: tests/dump/v2/, crates/djs-core/src/vm/dump_tests.rs, scripts/ (the two-commit check), TESTING.md
guide: -

## Goal
Hold the format 2 promise (D2560): every later build loads a format 2 dump and continues, or
refuses with a reason.

## Acceptance criteria
- [x] Golden format 2 fixtures written by the exporter (one per role sequence, loop kind,
      re-enactment and class case; small), committed and loaded by every gate run with the
      results delivered and the output compared.
- [x] A script that builds an older commit (git archive), exports at every idle point of the
      harvested cases there, and imports and finishes them on the current build; documented
      in TESTING.md as a periodic check, with its first results in the issue.
- [x] The rule for changing a fixture written down (only with a decision, as for v1).

## Log
- 2026-11-04 (portable): the two-commit check: scripts/dumpx/main.rs (a standalone tool,
  built twice by scripts/dumps-across-builds.sh OLD in target/dumpx: against `git archive
  OLD` to export a dump at every idle point of codemode + codemode-harvest with the
  transcript prefix and the full run; against this checkout to import, finish and
  compare). Needs the dump API of 0172+. First run: OLD=23835fa9 (0172's merge), import
  with 59ca2f38: 1,194 dumps, 1,194 finished the same, 0 refused.
- Waiting for 0180 (marker `"djs-dump 2";`) before writing the golden fixtures.
- Interrupt: two exporter blockers fixed and merged on their own (0172 branch, D2617
  codemode body as `async function (tools, console) {...}`, D2618 script bindings with
  host names); D2617/D2618 are from this issue's ID range.
- After 0180: golden fixtures in tests/dump/v2/golden/: 23 Script programs + 1 codemode
  body (`NAME.body.js`), dumps at every idle point written by the exporter
  (`NAME.K.dump.js`, 59 dumps, 87 KB), K in the file name. Roles covered: every one an
  await can have (call.*, new.*, member.object, super.this/base, object.*, array.*,
  binary.left, assign.*, template.part); super.callee/arg (constructors cannot await),
  import.specifier, intrinsic.arg and for-in (refused) are not. A private method as a
  call callee at an await is refused ("an internal function of a class"), so
  class-fields awaits before calling it. Test golden_dumps_load_and_finish (plain and
  GC stress in 97-fuel slices); writer write_golden_dumps (ignored,
  DJS_GOLDEN_WRITE=1, only programs without dumps). Fixed a latent dangling read in the
  body tests (the run promise is now held by a handle).
- Two-commit check after 0180: dd98ec06 -> 92f0c484: 1,194 dumps, all the same, none
  refused, ~30 s. Pre-0180 commits cannot be OLD any more (their dumps start with a
  comment line).
- TESTING.md "Dumps across builds"; D2619 (the rule for changing a golden dump). Guide:
  none affected.

## Follow-ups

## Result
- Golden format 2 dumps: tests/dump/v2/golden/ (programs + exporter-written dumps at
  every idle point), checked by `vm::dump_resume_tests::golden_dumps_load_and_finish`
  in every gate run. Add a program and run `DJS_GOLDEN_WRITE=1 cargo test -p djs-core
  --lib write_golden_dumps -- --ignored`; existing dumps change only with a decision
  (D2619).
- The two-commit check: `scripts/dumps-across-builds.sh OLD` (scripts/dumpx), by hand;
  OLD must be dd98ec06 or later. First results in the Log (all 1,194 finish the same).
- Gaps: super-call roles are unreachable by await; private-method callees at an await
  are refused by the exporter (not a format question).
