# 0214: Conflict-free derived files: smap summary and the dump-size score

status: in-progress
agent: pjd
branch: issue/0214-conflict-free-derived-files
depends-on: -
guide: -
touches: .gitattributes, scripts/merge-driver.py, scripts/setup-merge-drivers.sh, scripts/regen.sh, the pinned_maps test, the dump-size score test

## Goal
Two files conflict on almost every rebase while several agents add corpus cases (0212 log):
1. tests/smap/summary.txt (derived totals over the pinned .smap files): mark it
   `merge=djs-generated` and have scripts/regen.sh rewrite it through a summary-only mode of
   `pinned_maps` that still checks the per-case pins.
2. tests/scores/dump-size.tsv (per-case `SUITE CASE RAW GZIP` rows, D2906): a merge driver
   that merges rows by key (SUITE CASE) three-way: a row changed or added on one side takes
   that side's value; a row changed on both sides takes the smaller value per column (it is a
   ratchet; the gate then checks the merged result); a row deleted on one side and unchanged on
   the other is deleted. Keep the file sorted.
Test the drivers with a scripted three-way merge (no real branches needed), document them in
TESTING.md next to the other drivers (D1791).

## Log

## Follow-ups

## Result
