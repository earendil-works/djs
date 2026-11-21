# 0274: pi's cached start: image-cache key, restore zero-fill, instantiation

status: open
agent: pstart
branch: issue/0274-pi-start-path
depends-on: -
touches: js/pi (engine.ts, vm.ts: the prelude image cache and its key, instance reuse), js/src (only what pi's start path calls: image restore), crates/djs-wasm (restore's zero-fill only), tests/pi, js/test
guide: pi.md
decisions: D3450-D3459

## Goal
pstart measured pi end to end (0271's log): a cached start is 0.15 ms, of which
instantiation 18%, the image-cache key 16% (`preludeKeyOf` JSON-stringifies the whole
prelude on every start because pi passes a new `data` each run and the WeakMap misses),
the restore's zero-fill 10.6%, the engine's own work under 10%.
1. Key the prelude image cache on a per-prelude hash (or the prelude's identity plus a
   cheap content hash), so a start doesn't stringify the prelude.
2. The restore's zero-fill: zero only what the image doesn't overwrite (or what an earlier
   session dirtied, with D3023's instance reuse), measured.
3. Instantiation in pi's path: check D3023's instance reuse hits on the measured start; if
   not, why not.
Dumps at idle points (dump 46%, load 43% of ~7.5 ms) are out of scope: the owner paused
dump performance work (0215); list findings as follow-ups for the owner.

## Acceptance criteria
- [ ] cached start time and instructions, main vs branch (js/bench/pi.bench.ts, the pi tests)
- [ ] pi's tests pass in all modes; no behavior change for pi
- [ ] gate green; decisions recorded; pi.md updated if the start story changes

## Shared context (all of 0256-0260)
The owner asked to push both memory and speed as far as they go (2026-11-21). Background,
fresh profiles and the ranked idea list: work/scratch/perfreview/ of the primary checkout
(BRIEF.md, IDEAS.md; prof.ts and cat.py profile the codemode workloads in wasm). Read
IDEAS.md's section for your issue first; its numbers are estimates, not measurements.

Five issues run in parallel (0256 per-VM/per-byte costs, 0257 idle memory, 0258 GC,
0259 builtin calls, 0260 byte kernels). Stay in your `touches:`; announced overlaps are in
each issue's Notes.

Measuring (D016: wasm32 under node is the yardstick):
- run time and memory per workload: build the shipped module in your worktree
  (`scripts/build-wasm.sh`, or `--keep-names` for profiles) and run
  `DJS_WASM=$PWD/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm node js/bench/engines/run.ts --engines djs --reps 25`
  before and after, on main's build and yours, at load below ~4 (`uptime`; say the load).
  Memory claims: scripts/perf/mem-perturb.sh (D2048).
- instructions: scripts/perf/wmeasure.py / wmicro.py (TESTING.md "Instruction ratios").
- Report per workload: ms and MiB, main vs branch, and the module's gzip bytes.

Size: the shipped module is ~1.3 KB gzip under the D020 budget (ceilings, D3266). Report
the bytes of every change; find offsets where you can. If a measured win needs more room
than that, stop and tell the coordinator with the numbers: do not raise ceilings yourself.

Never weaken determinism (collection points a function of the program, D1710), suspension
anywhere, images, or the unsafe rules (AGENTS.md).

## Log

## Follow-ups

## Result
