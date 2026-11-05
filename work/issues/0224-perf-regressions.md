# 0224: Investigate and fix the native performance regressions since 2026-11-01

status: open
agent: -
branch: issue/0224-perf-regressions
depends-on: 0223
guide: pages quoting the affected numbers (generated via status.tsv)
touches: whatever the bisection points at (regexp compiler/matcher, gc, wire/stringify, wasm instantiate); scripts/bench.sh; tests/scores/bench.tsv

## Goal (owner: investigate hard and fix)
0223 re-measured the engine benchmarks (tests/scores/bench.tsv, machine Mac17,7) against the
2026-11-01 recording and found:
- confirmed by a second run: `regexp/compile` +26-83%, `gc` per MiB of live data +60-66%,
  `wire/stringify 10k-objects` +48%;
- one run only: UTF-16 literal regexp scans +14-27% (Latin-1 unchanged), `wasm/instantiate`
  29 -> 68 µs (module ~2x larger), `engine/djs/empty` 0.005 -> 0.131 ms (maybe measures something
  else now).

For each: confirm on a quiet machine (load < ~4), bisect to the commit(s) (git bisect with
`scripts/bench.sh <filter>` in a scratch worktree; many commits since 2026-11-01, so bisect, don't
guess), find the root cause, fix it (or justify a deliberate trade with numbers and a decision),
and add a guard where cheap (a bench threshold or an instruction-count test) so it can't silently
come back. Re-record with `scripts/bench.sh --record` at the end and update status.tsv
(scripts/status.py bench). Also check whether the wasm (shipped module) numbers moved the same
way. Write findings to work/research/0224/README.md.

Constraints: another agent (0225) works on binary size at the same time; tell the coordinator
before restructuring code it may also touch. Batched merges, gate checks locally first.

## Log

## Follow-ups

## Result
