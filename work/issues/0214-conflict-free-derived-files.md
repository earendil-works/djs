# 0214: Conflict-free derived files: smap summary and the dump-size score

status: done
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
- smap summary: `tests/smap/summary.txt merge=djs-generated`; `pinned_maps` gained
  `DJS_UPDATE_SMAP_SUMMARY=1` (writes only the summary, still checks the pins);
  scripts/regen.sh runs it when the driver marked the file (a failing pin check is
  reported, not fatal: the gate fails on it). Checked by hand: a corrupted summary plus
  the marker, regen.sh, the summary is back and the marker gone.
- dump-size: `merge.djs-scores` (scripts/merge-driver.py `scores`), rows merged three-way
  by (SUITE, CASE) as the issue says; deleted on one side and changed on the other keeps
  the change (a stale line the gate only reports); header ours unless only theirs changed
  it; suites in order of appearance, cases sorted (as dump-size.ts --update writes); a file
  that does not parse falls back to `git merge-file`.
- scripts/merge-driver-test.py: scripted merges in a throwaway repo (both drivers, the
  delete/change case, the unparsable fallback). Without the .gitattributes lines it fails.
  New gate leg "merge drivers" (cls nodocs, ~1 s). Outside the touches: scripts/gate.py
  (the leg) and work/README.md (one sentence next to the other drivers).
- Docs: TESTING.md "Derived files that merge by themselves" and the legs list.

## Follow-ups

## Result
tests/smap/summary.txt and tests/scores/dump-size.tsv no longer conflict on rebases: the
summary is a generated file rewritten by regen.sh (`DJS_UPDATE_SMAP_SUMMARY=1`), the score
merges row by row (`djs-scores`). Tested by scripts/merge-driver-test.py, a gate leg;
documented in TESTING.md and work/README.md.
