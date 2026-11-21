# 0279: Re-record the benchmarks page and published numbers after the memory/speed push

status: open
agent: pstart
branch: issue/0279-bench-rerecord
depends-on: 0256-0278
touches: tests/scores/status.tsv, tests/scores/*.tsv records, docs/guide (generated values and charts via regen; prose that the new numbers make wrong), README.md status lines if any
guide: benchmarks.md, and every page with generated numbers that moved (status.md, arena.md, gc.md, host.md, pi.md, limits.md, ...)
decisions: D3500-D3509

## Goal
Issues 0256-0278 changed nearly every published number: startup, idle and per-workload
memory, run times, the ops table, compile speed, pi's numbers, module sizes. All other
agents are idle now, so the machine is quiet: record everything `scripts/status.py`
records (engines, ops, bridge, startup, heap, pi, size, speed, dumps), regenerate the
guide, and fix the prose that no longer reads true (e.g. benchmarks.md's paragraphs on
memory, the per-op gaps and what is left of them; the idle anatomy; pi.md). Check load
before every recording (below ~4) and say it in the records.

## Acceptance criteria
- [ ] every status record re-taken at load < ~4 (or the load stated)
- [ ] `cargo run -q -p djs-docs -- check` passes; prose updated where the numbers changed its meaning
- [ ] gate green


## Log

## Result
