# 0223: Refresh every benchmark, table and published measurement

status: in-progress
agent: meas
branch: issue/0223-measurements-refresh
depends-on: -
guide: README.md and docs/guide pages with numbers (status.md, introduction, goals, arena, heap, compiler, gc, builtins, dumps, limits, pi, js-api, testing)
touches: tests/scores/ (bench.tsv, status.tsv), scripts/status.py, scripts/bench.sh, scripts/dump-costs/, scripts/dump-size/, README.md and guide prose with numbers, djs-docs gen:status

## Goal (owner)
Update the benchmarks, the tables and the general measurements, so every number djs publishes
is current and comes from a recorded measurement:

1. **Inventory** every number in README.md, docs/guide/*.md (prose, tables, generated
   `gen:status` regions), examples/*/README.md and docs/*.md that the guide or README quote:
   conformance (test262, difftest, pi suite), speed (engine benchmarks, pi's benchmark vs
   QuickJS), sizes (wasm modules raw/gzip, slim, dump module, base heap, images), dump sizes and
   times (pi sessions, sleep example, corpus), startup, memory. For each: where it comes from
   today and when it was last measured.
2. **Re-measure on a quiet machine** (all other agents are archived; check `uptime` < ~4 before
   recording, wait if not): `scripts/bench.sh --record` (engine track), `scripts/status.py`
   (status.tsv), the dump numbers (scripts/dump-costs/ incl. pi-sessions.ts, scripts/dump-size/),
   pi's benchmark vs QuickJS, module sizes, test262/difftest/pi pass counts.
3. **Make numbers generated where they can be:** prose figures that 0220 left unmeasured (heap,
   arena, compiler, gc, builtins pages) and any other hand-written number become `gen:status`
   values (extend scripts/status.py and status.tsv with new keys) so `djs-docs check` keeps them
   current. Remove numbers that can't be measured or no longer mean anything.
4. **Update the tables** (README, guide status page, dumps page, pi page) and regenerate
   (`djs-docs regen`). Fix the stale code comment 0220 found in crates/djs-core/src/interp/run.rs
   (it lists "the start of `run`" as a safepoint the code doesn't have).
5. Write work/research/0223/README.md: what was measured, how, on which machine, load at the
   time, and notable changes since the last recording (e.g. regressions to look into).

Merge once (or twice: measurements, then generated-number plumbing). Report the headline numbers
to the coordinator.

## Log

## Follow-ups

## Result
