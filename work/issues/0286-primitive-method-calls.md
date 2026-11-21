# 0286: Builtin method calls on primitives, outside the loop's shared path

status: open
agent: pcall
branch: issue/0286-primitive-method-calls
depends-on: 0282
touches: crates/djs-core/src/interp/run (decline paths, fastpath, the call path for builtin callees; run_hot only with the hotcode.sh check), builtins/fast.rs, crates/djs-core/src/compiler only if a call-site form helps
guide: interpreter.md
decisions: D3550-D3559

## Goal
The ops table's remaining red rows are mostly tiny builtin calls on primitives: trim 1.18,
slice 1.15, padStart 1.27, charCodeAt loop 1.14, String(i) 1.23, toFixed 1.14 (wall, best
QuickJS). A method call on a string/number pays ~150 instructions of prototype lookup plus
~250 of generic leaf dispatch; QuickJS calls a C function directly. In-loop variants
(D3265, D3301) lost to V8's register allocation. Find a path outside run_hot's shared code
that makes `prim.method(args)` with a @fast/@rust builtin callee cheap end to end: e.g. the
method IC hit carrying the builtin's leaf id so the call skips frame construction and the
generic dispatch (arguments passed in place, no Shadow copy), a specialized leaf signature
for 0-2 argument string methods, or a compile-time call form for `x.m(...)`. Measure with
slope.py per op and the hotcode.sh check; report the design with numbers before a big build.

## Acceptance criteria
- [ ] per-op instructions vs QuickJS 2026 before/after for the red rows; workloads; run_hot's shared path unchanged
- [ ] the every-stop equivalence test covers the new path
- [ ] size reported (~1 KB of the owner's allowance left, shared with 0285 and 0287); gate green; decisions recorded
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
