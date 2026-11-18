# 0255: Re-record the benchmarks page's operations after 0254

status: done
agent: owner session
branch: issue/0255-bench-rerecord
depends-on: 0254
touches: tests/scores/status.tsv, docs/guide/benchmarks.md, docs/guide/gen/chart-engines-ops.svg
guide: benchmarks.md

## Goal
0253 recorded the page just before 0254 (number formatting, toFixed): its String(i),
toFixed and padStart rows were stale.

## Log
- `scripts/status.py engines` at 17e89acc. The machine was not quiet (a Windows VM and
  other sessions): workloads at load 7.2, ops 9.7, bridge 8.1. Absolute times rose by about
  half for every engine; the djs/QuickJS ratios of the ops moved by at most ±4% except
  the ones 0254 changed (toFixed 1.92 → 1.22, String(i) 1.81 → 1.26, `'' + i` 1.34 →
  0.92, padStart 2.58 → 2.25).
- Kept only the `engines/ops/*` values of this run; the workload, startup, memory and
  bridge tables keep 0253's recording at load 2.3, which nothing since has changed beyond
  noise.
- Prose: String(i), toFixed and template literals now within about a quarter of QuickJS.

## Follow-ups
- On a quiet machine, re-record the operations table (its absolute ms are high from load
  9.7; the ratios hold).

## Result
The ops table and chart from 17e89acc; prose updated.
