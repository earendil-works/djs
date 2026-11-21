# 0259: Builtin calls without finding the callee

status: open
agent: pcall
branch: issue/0259-builtin-call-cost
depends-on: -
touches: crates/djs-core/src/compiler (resolve: recognizing builtin call sites), crates/djs-core/src/bytecode (new guard ops), crates/djs-core/src/interp/run (decline_fast, fastpath, call; run_hot only with measurements), builtins/fast table, value tags if a token value is introduced (and the dumper's handling of it)
guide: interpreter.md, builtins.md
decisions: D3300-D3309

## Goal
Small builtin calls are the speed gap left on long bodies: padStart 2.25x QuickJS,
Object.entries/fromEntries 2.38, sort with a comparator 1.84, Map.set 1.81, Set.add 1.52,
object spread 1.45, charCodeAt loop 1.43, trim 1.35, slice 1.32, Math.floor 1.30.
D3265 measured a builtin-call statement at ~1,000 wasm instructions: ~450 to find the
callee, 150-200 in fast_leaf_call's generic dispatch, leaving the loop the smallest part.
Its separate call opcode was dropped because adding to run_hot slowed other code 1-2%.
Attack the lookup instead (IDEAS.md idea 3, "known-builtin call tokens"): recognize
`Math.floor(x)`, `s.trim()`, `String(i)`-shaped sites at compile time, guard cheaply that
the builtin and the path to it are original, and call the leaf by id; any failed guard
falls through to the unchanged generic code, with the spec's evaluation order.
Start with the cheapest experiment (three hard-coded ids, guard in decline_fast, count
instructions per statement), report the numbers, then generalize to the hot table. Then
the per-op items above that are not lookup-bound (Map.set/Set.add string keys, comparator
sort, Object.entries/fromEntries, spread): profile each and fix what is cheap.

## Acceptance criteria
- [ ] instructions per statement (wmicro) and the ops table's ratios (js/bench/engines/ops.ts, djs vs best QuickJS) before/after
- [ ] no regression above noise on code without builtin calls (wmeasure tight, calls, prop1, loop)
- [ ] exact semantics: corpus + test262 cases for replaced/shadowed/monkey-patched builtins (globals reassigned, prototypes patched, frozen builtins mode)
- [ ] suspension and dumps: a token (if any) is handled by GC, frames, stack traces and the dumper
- [ ] size reported; gate green; decisions recorded

## Notes
- Overlap: 0258 (pgc) adds a latch flag set when a script writes below the base's end
  (reshapes a builtin). That is your "pristine" signal; use it rather than adding a
  second one. If you need it before 0258 lands, agree with pgc on the name and who adds it.
- run_hot is sensitive to any code change (D1352, D3122, D3128): measure every change to
  it against a build-noise control.

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
