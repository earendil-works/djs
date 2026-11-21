# 0279: Re-record the benchmarks page and published numbers after the memory/speed push

status: done
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
- [x] every status record re-taken at load < ~4 (or the load stated)
- [x] `cargo run -q -p djs-docs -- check` passes; prose updated where the numbers changed its meaning
- [x] gate green


## Log
- Each record waited for a 1-minute load below 4 (work/scratch/0279/waitq.sh) and records
  it: D3500 makes status.py write `<group>/load` for speed, pi, startup, dumps, bounds and
  heap (engines, ops, bridge already did). Loads: size (sizes, any load), speed 3.8, bounds
  3.3, heap 3.7, dumps 3.7, startup 3.7, pi 3.8, engines 3.1 (runs at 3.3/3.0/3.1), ops 2.9,
  bridge 3.3, bench.sh --record 3.2 (wasm/ group again at 2.5).
- Fixed on the way: js/bench/sandbox.bench.ts and wasm.bench.ts made several VMs in one
  instance with `vm_new`, which since D3360 needs the base handed over again for each VM
  (they trapped); both now inject the decoded base before each `vm_new`, as a host's fresh
  instance gets it. `startup/vm-new-us` and `wasm/vm_new` therefore include the injection.
- `native/foreach-x`, `native/arrays-x` stay as recorded at 35ed5ab789: the native bench
  lost its bytecode twin in 0251; status.py's bench no longer asks for them.
- bench.sh's size probe (`djs-size-probe`) fails to build for wasm with the shipped build
  environment (duplicate `core` lang items: build-std vs the prebuilt std); its numbers are
  not published (follow-up).
- Regenerated; prose fixed where the numbers changed its meaning: benchmarks.md (in short:
  reads out six to nine times as fast, memory about a tenth above the QuickJS releases;
  idle memory now equal to QuickJS-NG's, the stack sentence dropped; the memory section
  rewritten: below QuickJS where tool results are large, level on small bodies, the
  collector on text/heavy; ops: includes, map, new RegExp and object spread faster,
  Math.max/floor at parity, Math.floor out of the gap list; bridge summary; dump gzip
  ratios), host.md (reading out: six to nine times instead of an order of magnitude),
  pi.md (interrupt budgets: a little over half the CPU time at 108,000 polls/s vs 61,000).

## Result
Every status record re-taken on a quiet machine (loads 2.5-3.8, recorded per group, D3500),
the guide and README regenerated, prose fixed in benchmarks.md, host.md and pi.md. Two
benches repaired for D3360. Follow-ups: bench.sh's wasm size probe does not build with
the shipped build environment.
