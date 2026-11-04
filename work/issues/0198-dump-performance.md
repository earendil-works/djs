# 0198: Make dumping and loading fast

status: open
agent: -
branch: issue/0198-dump-perf
depends-on: 0196
guide: docs/guide/snapshots.md (dump costs) if numbers change
touches: crates/djs-core/src/vm/dump*.rs, crates/djs-core/src/compiler/ (recording, maps), crates/djs-wasm (dump paths), js/src/sandbox.ts (dump module use), scripts/dump-costs, docs/continuation-dumps.md

## Owner
"After the architecture change, get an agent to optimize the dump performance. I'm sure
there is a lot of opportunity left to make that really fast."

## Where we are (main after 0196, D2735)
- Format 2 dumps are named-slot state machines (D2720, D2731–D2735); a load compiles the
  dump text once with recording on (load_dump ~450 ms native over the 1,164 harvested
  dumps, 23 MB; was ~800 ms).
- Last wasm measurements (0184, before SM): corpus p50/p90 dump 1.84/3.65 ms (packaging B),
  resume ~0.9 ms; data-heavy cases tens of ms (records 21/38 ms dump/resume, 5 MB log
  68/81 ms). scripts/dump-costs/dump-costs.ts measures it.
- Known opportunities (sm, critic):
  - the exporter recompiles each script per dump (script_maps/body_site): a per-VM map
    cache;
  - big data dumps compile at ~100 ns/byte (12 MB → 1.2 s native): data sections could
    skip the compiler (a fast literal/JSON path for data-only statements, or a separate
    data encoding the loader parses directly);
  - drop executed body text (critic's report §6; needs a decision, changes toString of a
    body only the run holds);
  - skip re-dumping an unchanged run (dump-after-load is byte-identical);
  - the dumper's graph walk, string escaping, number formatting, allocation;
  - packaging B's image transport between modules.

## Goal
Profile first (native and wasm, the corpus distribution and the data-heavy cases), rank
the costs, then land the wins in small merges with numbers before/after. Keep the format a
plain Script unless a decision says otherwise; keep every gate check green (goldens,
dump-every-idle, idle-point counts at 100%).

## Log

## Result
