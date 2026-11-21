# 0284: Refresh the published numbers after 0280-0282

status: in-progress
agent: pstart
branch: issue/0284-rerecord
depends-on: 0279, 0280, 0281, 0282
touches: tests/scores/status.tsv and the other score records, docs/guide (regen; prose only where the new numbers change its meaning)
guide: benchmarks.md, status.md and any page with generated numbers that moved
decisions: -

## Goal
0279 re-recorded everything at f8dae01d. Since then 0280 (construct path, object rest, the
owner's +2 KB budget), 0281 (regexp lastIndex) and 0282 (fast builtin getters) merged:
module size (~+1.5 KB gz), the engine track and ops, speed and startup moved. Re-record the
groups that changed (at least size, engines incl. ops and bridge, speed, startup, bench.sh
records that cover them), at load < ~4 with the load stated, regenerate, check the prose
(the module-size sentences in benchmarks.md, the budget wording if the D020 allowance shows),
and merge. Other agents are idle.

## Acceptance criteria
- [ ] the changed groups re-recorded at load < ~4; djs-docs check passes; gate green

## Log

## Result
