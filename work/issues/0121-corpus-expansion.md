# 0121: Codemode corpus expansion; shorter ratchet reasons

status: in-progress
agent: infra
branch: issue/0121-corpus-expansion
depends-on: 0118
guide: docs/guide/testing.md (if the corpus description changes)
touches: scripts/harvest/, tests/corpus/codemode-harvest/, crates/djs-difftest/ (ratchet reason format, structured fake tools), TESTING.md

## Goal
0118's follow-ups:
1. Harvest more of the ~1,800 codemode runs that 0118 had to skip, by giving the fake tools
   structured synthetic results for MCP tools, `models`, edit/write and `load` (stored
   values), under exactly the same privacy rules as 0118 (D2281; redaction report;
   manual review; the coordinator spot-checks before merge).
2. Shorter ratchet reasons in difftest: a short reason (the first differing line, truncated)
   instead of whole output diffs (the 5 harvest entries total ~28 KB).

## Acceptance criteria
- [ ] More harvested cases (target: double the coverage of language features and tool
      shapes, not just the count), djs vs qjs results clustered and reported.
- [ ] The ratchet reason format shortened, with the full diff still available via the
      runner's normal output.

## Log

## Follow-ups

## Result
