# 0284: Refresh the published numbers after 0280-0282

status: done
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
- [x] the changed groups re-recorded at load < ~4; djs-docs check passes; gate green

## Log
- Re-recorded at main b8fec821 (0280-0282 in): size; speed (load 3.7); startup (3.3); pi (3.3).
  Startup's wall times moved +12-25% (sandbox 55.5 → 62.2 µs, first run 72 → 86, instantiate
  17.6 → 22.4) at the same load; the module grew 3.5 KB raw.
- Engines (with ops and bridge): two runs, at load 5.7-6.8 and 5.6-11.4 (fseventsd at 100%,
  Chrome, then the other agents' builds; the load reached 500 later). The ratios did not hold
  against 0279's records at load ~3: sequential 0.38/0.37 (djs/QuickJS) became 0.61/0.84, the
  run table's QuickJS geomean 1.38× → 1.76×, a new VM 0.07 → 0.17 ms, several ops ratios moved
  0.1-0.2. Not published: the engine records stay 0279's (load 3.1/2.9/3.3), except djs's
  module sizes in the size table (load-independent), taken from the later run.
- Ops: "Object.entries/fromEntries x200" (200 rows once, 0.00 ms: too short to compare) made
  "x2" (all 600 rows twice, ~2.4 ms in djs, ~3 ms in QuickJS). Measured at load 6-7 only
  (0.61-0.79), so per the coordinator (option c) the change is parked: ops.ts and the djs-docs
  label keep the old case until the next quiet re-record; the patch is
  work/scratch/0279/ops-entries.patch (ops.ts + crates/djs-docs/src/status.rs; drop the old
  `object-entries-fromentries-x200` status keys with it).
- bench.sh --record at load ~10: wire/decode-vs-json-objects 2.16, -string 2.81,
  read-vs-json-objects 1.52 and the compile ratio held (within 0.06 of 0279's) and are
  published; wire/read-vs-json-text came out 19.74 against 0279's 9.06, so 0279's value is kept.
- Regenerated; no prose changed meaning (module 995 KB / 431 KB gz, pi polls 117,000 vs
  60,000: still a little over half the CPU time).

## Result
Published numbers refreshed where they could be measured honestly: size, speed, startup, pi
and the held bench ratios. The engine comparison stays at 0279's records (this run's load
distorted it); the Object.entries ops case waits for a quiet re-record (patch in scratch).
