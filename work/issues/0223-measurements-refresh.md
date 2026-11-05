# 0223: Refresh every benchmark, table and published measurement

status: done
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

- Inventory: README (conformance, speed, size regions; pi benchmark "at 0095"; VM/Sandbox
  startup), guide prose figures on dumps, pi, limits, heap, arena, compiler, gc,
  builtins, host, js-api, snapshots, testing pages; examples/sleep-dump (688-byte dump,
  current). The status.tsv size record was stale (1,471 KB with the base in data). Full
  list with old and new values: work/research/0223/README.md.
- Killed an orphaned, hung djs-difftest from .worktrees/pjz (15 h at 100% CPU) before
  recording; told the coordinator.
- Measured (load 2.3–4.7 one-minute, 18 cores): status.py size/speed/pi/startup/dumps/
  bounds/heap/bench, `bench.sh --record` (all, then compile/), the dump-size score
  (--update: 34 gzip bytes lower). pi-tests 221/226 in all three modes.
- Fixed dump-costs.ts (no frozenBuiltins: every dump was refused, 0 idle points).
- Tooling: status.py subcommands, pi.bench.ts polls per second, djs-bench lazy compile,
  scripts/size/object-bytes.ts, djs-docs inline numbers and `gen:status recorded` (D3070).
- Pages updated: README.md, status.md, pi.md, dumps.md, limits.md, heap.md, arena.md,
  compiler.md, gc.md (unmeasurable ratios removed), builtins.md, host.md, js-api.md,
  snapshots.md, testing.md; docs/guide/README.md documents inline numbers.
- run.rs module comment: safepoints per the code (not the start of `run`).
- Records say 7167411f0f (the base): no engine source differs on the branch, and branch
  hashes would not survive the rebase (coordinator's instruction).

## Follow-ups
- Native benchmark regressions since 2026-11-01, confirmed twice: regexp/compile +26–83%,
  gc per MiB live +60–66%, wire/stringify 10k-objects +48%; single-run: UTF-16 literal
  regexp scans −14 to −27%, wasm/instantiate 29 → 68 µs (research README).
- status.py records HEAD; on a branch that hash dies in the rebase. Record the newest
  commit that touched the engine's inputs instead, or the merge base when they are equal.
- gc.md's in-place vs copying collection cost and peak, and compiler.md's per-function
  lazy costs, have no measurement now (removed from the prose); a djs-bench case each
  would let them come back as inline numbers.
- pi.md: djs polls pi's interrupt 1.27x as often as QuickJS; if pi's budgets should mean
  the same CPU, the adapter's slice could be about 12,700 fuel.

## Result
Every published number is current and generated: README and guide figures come from
tests/scores/status.tsv (recorded by `scripts/status.py`, now with `size` for all three
modules, `speed`, `pi`, `startup`, `dumps`, `bounds`, `heap`, `bench`, `all`) or are
counted from the repository, through `gen:status` regions and inline `<!--n KEY FMT-->`
numbers that `djs-docs check` keeps fresh (D3070). status.md lists every record
(`gen:status recorded`; pi's on pi.md). bench.tsv has a new full run plus the lazy compile
case. Headlines and regressions: work/research/0223/README.md. Gaps: see Follow-ups.
