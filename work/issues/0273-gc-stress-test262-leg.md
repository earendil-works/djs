# 0273: test262 under GC stress as a full-only gate leg

status: in-progress
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
- [ ] the leg exists, is documented in TESTING.md, and finishes within its bound
- [ ] one run on current main, findings in the Log (routed to the coordinator)
- [ ] gate green; decisions recorded


## Log

## Follow-ups

## Result
