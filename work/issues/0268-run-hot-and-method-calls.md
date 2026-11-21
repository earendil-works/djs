# 0268: run_hot's sensitivity, receiver method calls, the general path

status: open
agent: pcall
branch: issue/0268-run-hot-method-calls
depends-on: 0259
touches: crates/djs-core/src/interp/run (run_hot, ic, decline paths, step_once, call_intrinsic, Fault), scripts/perf (tools for the investigation)
guide: interpreter.md
decisions: D3390-D3399

## Goal
1. Understand why any change to run_hot moves V8's code for the whole loop by several
   percent (D1352, D3122, D3128; 0231's follow-up): register pressure, spills, block
   layout, br_table shape, locals count? Compare V8's generated code (--print-wasm-code /
   --trace-turbo) for twins that differ by one arm. The goal is a rule or a structure
   that makes additions safe, since it blocks every interpreter improvement. IDEAS.md
   idea 13 (threaded/tail-call dispatch) is one option; judge it with data, including
   what wasm tail calls (`return_call`) would need from the Rust toolchain on stable.
2. Receiver method calls (`s.trim()`, `a.push(x)`, `s.charCodeAt(k)`): ~300 instructions
   is the prototype inline-cache hit inside run_hot, now the largest part of a builtin
   method call. With 1's understanding, make it cheaper (or move the receiver-kind check
   + builtin id into the IC entry), measured against a build-noise control.
3. The general path (IDEAS.md idea 12): step_once (~30 KB of wasm) and call_intrinsic
   (~39 KB), a boxed Fault; their self time is 5-6% of text/json/heavy. Smaller and
   cheaper where the profiles point.

## Acceptance criteria
- [ ] a written explanation of run_hot's sensitivity with evidence (in the issue and a decision)
- [ ] per-op instructions for method calls; workload instructions (wmeasure tight/calls/prop1/loop/strings/json + the codemode set) with a noise control
- [ ] size reported (room ~2.4 KB under the ceiling, shared); gate green; decisions recorded

## Notes
- pstart (0265 part 2) works in compiler/resolve.rs, not in interp/run.
- pidle (0266) works on promise/async paths in interp (suspend, jobs): coordinate through
  the coordinator if you touch the same functions.

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
