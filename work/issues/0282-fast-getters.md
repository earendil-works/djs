# 0282: @fast entries for builtin getters (Map/Set size and friends)

status: done
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
- [x] per-op instructions vs QuickJS before/after; workloads; hotcode.sh where run_hot moves
- [x] size reported, ceiling raised by what's needed (recorded); gate green
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
- Baseline (main 7b302395, steady state vs quickjs-wasi): every builtin getter 2.8–6.9x
  (`re.global` 4,700, `m.size` 3,258, `ta.length` 3,290, `sym.description` 3,099,
  `e.stack` 2,880, `ab.byteLength` 1,907).
- `@rust` on the size getters works (bgen accepts it on a getter) but gives 1,748: its
  intrinsics go through `call_intrinsic`. Hand entries in fast.rs: 1,066 through the
  getter path of 0276; run from `field_slow` inside the loop: 620.
- Typed arrays: entries for `length`, `byteLength`, `byteOffset`; the caches take a typed
  array receiver for named keys (`NAMED_KEY` in the slot). First attempt (skip the
  numeric test for any filled slot) was wrong; e2e proto_caches.js caught it.
- A rebase onto pidle's construct path made `hot_loop_is_invisible` fail: a stop landed
  right after a fast getter, whose dead slot the loop had not written. Wrote
  `hot_loop_paths_are_invisible_at_every_stop`, which then found that the string-key
  paths of 0268/0269 intern differently in the two modes; `exec` now uses the same
  helpers (D3531).
- New corpus case builtins/fast-getters.js (recorded from qjs): sizes, typed array
  fields, every declined receiver, patched, shadowed and borrowed getters, resizable
  buffers.
- Not done: undoing `exec`'s preload (+718 B gz for props −1.5%); a slot hint for fast
  getters (no gain, and it made the two modes' images differ); the other builtin getters.
- Outside `touches:`: run.rs `exec` (string-key helpers in four arms), vm/tests.rs (the
  new test), vm/compat.rs (COMPAT_VERSION 10), tests/scores/size.tsv.
- Guide: builtins.md (`@fast` getters), interpreter.md (caches: fast getters, typed
  arrays).
- Checked: lib tests (630), e2e in four modes, difftest (corpus plain; builtins/,
  codemode/, language/, dquickts/ with suspension, GC stress and determinism; codemode/
  lazy with plain dumps), test262 built-ins and language (39,251 active, no new
  failures), djs-bgen tests.

## Follow-ups
- Typed array element reads: `ta[j]` is 603 instructions (1.96x QuickJS); `Heap::ta_length`
  is recomputed per element (heap/typed.rs). `ta.length` itself is 842 for the same
  reason.
- The other builtin getters (`re.global` and the RegExp flags 6.9x, `sym.description`
  5.2x, `e.stack` 4.2x, `re.source` 3.4x, `ab.byteLength` 2.8x): entries only where a
  workload shows them.
- User getters are 1.26x (class) and 1.46x (own): a getter's frame pushed inside the
  loop (0276's follow-up 2).
- `exec`'s operand preload (props +1.5%): +718 B gz to undo.

## Result
Merged with this commit; D3530, D3531. `m.size` 3,283 → 620 instructions (4.7x → 0.89x
QuickJS), `st.size` 0.91x, a typed array's `length` 4.5x → 1.15x, `byteLength` 1.17x,
its method lookups 3.5x → 0.76x, `ta[j]` 2.42x → 1.96x. User getters and absent reads
pay 3–8% for `field_slow`'s new tests. Loops and workloads unchanged in steady state
(±0.25%; wmeasure ±0.1% with sync tier-up, +0.0..0.5% under default tiering from the
tier-up of a slightly larger run_hot). Shipped 430,436 → 430,951 B gz (+515), slim +478;
ceilings raised to those from the owner's allowance (1,079 B left).
