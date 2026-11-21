# 0282: @fast entries for builtin getters (Map/Set size and friends)

status: in-progress
agent: pcall
branch: issue/0282-fast-getters
depends-on: 0276, 0280 (budget change)
touches: builtins (Map/Set size, typed-array length, RegExp flags getters: @fast entries), builtins/fast.rs, interp/run getter path only as needed
guide: builtins.md if a fast-path story changes
decisions: D3530-D3539

## Goal
0276's top follow-up: `m.size` is still 4.7x QuickJS because the getter is a bytecode
builtin; 0276's getter path already goes through `call`, so a @fast entry would be picked
up without further changes. Add @fast entries for the builtin getters codemode hits
(Map/Set size first; others only if measured to matter), measured per op and on workloads.
If bytes remain, 0276's follow-up 6 (undo the step operand preload for arms that don't
need it, props +1.5%).

## Acceptance criteria
- [ ] per-op instructions vs QuickJS before/after; workloads; hotcode.sh where run_hot moves
- [ ] size reported, ceiling raised by what's needed (recorded); gate green
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

## Result
