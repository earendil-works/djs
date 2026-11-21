# 0273: test262 under GC stress as a full-only gate leg

status: done
agent: pgc
branch: issue/0273-gc-stress-test262
depends-on: -
touches: scripts/gate.py (a full-only leg), crates/djs-test262 (a gc-stress option if the scratch patch needs a home), TESTING.md
guide: testing.md
decisions: D3440-D3449

## Goal
0258's debug verifier, run with test262 under GC stress, found an unlatched prefix write
within minutes that every regular leg missed; the run itself hit a one-hour timeout without
a summary (patch in work/scratch/0258/). With five agents changing the heap, interpreter and
builtins at once, make that run a full-only leg (`scripts/gate.sh --full`, run daily by the
coordinator): debug build with the verifier, GC stress, sharded or filtered so it finishes
in a bounded time (say ≤ 20 min on 6 jobs), a ratchet or an explicit expected list, and a
summary even on timeout. Then run it once on current main and route anything it finds.

## Acceptance criteria
- [x] the leg exists, is documented in TESTING.md, and finishes within its bound
- [x] one run on current main, findings in the Log (routed to the coordinator)
- [x] gate green; decisions recorded


## Log
- djs-test262: `--gc-stress`, `--exclude`/`--exclude-from`, `--shard K/N`, `--deadline SECS`
  (not-started tests leave the tables and the ratchet; `deadline:` line). Under
  `--gc-stress` the fuel slice between wall-clock checks is 100x smaller.
- First full stressed run (all active, wall timeout 20 s): 31 failures, all wall-clock
  timeouts (TypedArray copyWithin/fill/slice, RegExp class escapes, decodeURI A2.5,
  Array 15.4.4.x long loops, Iterator zip); the generated property-escape tests take
  minutes each. These 60 + the property-escapes dir are tests/test262/gc-stress-skip.txt.
- The leg on main e0c804e4 (scripts/gate.sh --full --only "test262 gc stress"): eager and
  lazy, 38,836 tests each, 189 s for both with all cores at load ~45; slowest remaining
  15 s; ratchet OK (the 40 expected). Findings: none (0258's unlatched write is fixed;
  nothing new). Shards of 1/40 were also run while sizing it: same result.

## Follow-ups

## Result
`scripts/gate.sh --full` runs "test262 gc stress" (D3440): test262's active group, eager and
lazy, under GC stress in the debug build, against the regular ratchet, minus
tests/test262/gc-stress-skip.txt; ~3 min, bounded by a 10-min deadline per mode with a
summary. TESTING.md "Full runs" and guide testing.md describe it. No findings on main
e0c804e4.
