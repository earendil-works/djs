# 0280: D020 budget +2 KB (owner); land the parked construct path and object rest

status: in-progress
agent: pidle
branch: issue/0280-budget-parked
depends-on: 0270, 0278
touches: scripts/size/budget.py, tests/scores/size.tsv, TESTING.md "Size budget", docs/decisions.md; then the parked patches' areas (interp/run/construct.rs, compiler class codegen, the two new ops; object rest's copy_data_props path)
guide: interpreter.md if the construct story changes
decisions: D3510-D3519

## Goal
The owner decided (2026-11-21) to give the parked speedups room: raise D020's budget by
2 KB of gzip. Do it first, as its own small merge:
- record the owner's decision (D3510: the budget is 1.5x quickjs-wasi + 2,048 B gzip,
  made to land the speedups parked for size in 0270/0272/0275/0276/0278);
- budget.py: an explicit allowance (e.g. a `budget_allowance_gzip` key in size.tsv)
  added to 1.5x, printed in its output; TESTING.md's "Size budget" updated.
The ratchet ceiling stays a ratchet: each batch below raises it by what it needs, recorded.
Then land, re-measured on current main with --wasm-sync-tier-up / slope.py and the
hotcode.sh shared-path check (pcall's notes in D3410: new ops appended at the end of the
table, the class-ctor flag below bit 16):
1. the construct path (D3410, issue/0270-construct-parked, work/research/0270/construct-path.patch);
2. object rest (D3491, issue/0278-object-rest-parked, work/research/0278/object-rest.patch).

## Acceptance criteria
- [ ] the budget change merged first (others rebase onto it)
- [ ] both patches re-measured and landed (or a measured reason not to), run_hot's shared path unchanged
- [ ] size per batch reported; gate green; decisions recorded

## Notes
- pscan (0281, regexp matcher state reuse, ~+100 B) and pcall (0282, @fast builtin
  getters such as Map/Set size) land into the same 2 KB; merge the budget change quickly.
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
