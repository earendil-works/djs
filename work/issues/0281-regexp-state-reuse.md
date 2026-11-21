# 0281: Land the regexp matcher's state reuse (parked in 0275)

status: done
agent: pscan
branch: issue/0281-regexp-state-reuse
depends-on: 0280 (budget change)
touches: crates/djs-core/src/regexp (matcher state), the exec leaves, tests
guide: -
decisions: D3520-D3529

## Goal
0275's D3461 kept the matcher's memory between matches: −6% on match, −8% on test, for
+100-118 B gz; parked for size. The owner raised the budget (0280's D3510). Rebase the
patch (work/scratch/0275/re-state-reuse.patch) onto current main after 0280's budget merge,
re-measure, and land it. Check memory (codemode-memory leg) since kept state is retained
memory. Also look at 0275's other follow-up if it is cheap: each evaluation of a regexp
literal defines `lastIndex` the slow way (~4% of a match).

## Acceptance criteria
- [ ] re-measured per op and on text/heavy/loop; memory unchanged
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
- Reuse patch rebased onto 7b302395: +470 B gz for match −3.7%, replace −2.0%, test −2.1%. A
  dummy Vec field on Heap alone is +317 B gz (layout reorder re-offsets every heap field
  access); field placement makes no difference; a bigger reserve without reuse gains nothing.
  Not landed (coordinator agreed): D3520.
- lastIndex of a new RegExp via heap.add_fresh_data: test −6%, match −2.5%, −11 B gz (D3521).
- Workloads (ins.py): heavy, text unchanged, loop −1.8% (noise level).
- Tests: test262 built-ins/RegExp, String/prototype; difftest builtins, codemode, language
  plain and --suspend-every 97 --gc-stress; lib tests.

## Result
Landed D3521 only: a new RegExp's `lastIndex` is inserted directly (test −6%, match −2.5%,
−11 B gz, memory unchanged, no ceiling change). The state reuse (D3461) is not landed: on
current main it costs +470 B gz for −2..−4%, most of it from adding a field to `Heap`
(D3520: any new Heap field costs ~300 B gz through re-offset accesses).
