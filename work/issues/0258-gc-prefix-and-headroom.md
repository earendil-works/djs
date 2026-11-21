# 0258: GC: minor collections skip the builtin prefix; headroom and big-string survivors

status: in-progress
agent: pgc
branch: issue/0258-gc-prefix-headroom
depends-on: -
touches: crates/djs-core/src/heap/compactor.rs, heap/gc.rs, heap/roots (a latch flag), heap/object.rs (only the sites that set the latch), GC side-table allocation
guide: gc.md, arena.md
decisions: D3290-D3299

## Goal
Per-run memory above idle is high on text, loop and heavy (IDEAS.md 0.1), and djs's own GC
is 5-11% of fanout, media, text and json (BRIEF.md's gc column counted V8's collector; the
real shares are in IDEAS.md 0.2).
1. `Marker::mark_old` walks every old cell (the whole base prefix, ~4,800 cells) on every
   minor collection to find references into young cells. Nothing writes below the base's
   end unless a script reshapes a builtin (D2063-D2065): a latch bit set by those writes
   lets an unlatched VM start the walk at the base's end (IDEAS.md idea 1).
2. With collections cheaper, retune `BASE_ALLOWANCE` (288 KiB, sized for an old prefix)
   and `MIN_GROWTH`: sweep and measure memory and CPU on all workloads plus the native
   engine track's GC-heavy cases (json-objects, strbuild, pi-like).
3. heavy: the big-string growth discount for survivors only (0234's follow-up: a Roots
   field with the previous collection's end, or a header bit), which got heavy toward
   3.5 MiB without taxing transient strings.
4. GC side tables at multi-MB heaps land at memory's end (IDEAS.md idea 9): carve them from
   the arena's tail or otherwise keep them from adding pages.
5. If time remains: IDEAS.md section 5's fixed-nursery design is the only generational
   variant that can lower memory (D2042/D2046 rejected the proportional ones); prototype
   only if 1-4 leave a clear gap, and report before building it.

## Acceptance criteria
- [ ] memory (mem-perturb) and run time per workload, main vs branch; GC share in the profiles
- [ ] collection points stay a function of the program; the difftest's --gc-stress and determinism legs pass; pinned GC counts updated with reasons
- [ ] the debug barrier (D2065) still checks the latch's invariant
- [ ] size reported; gate green; decisions recorded; gc.md updated

## Notes
- Overlap: 0259 (pcall) wants a "builtins are pristine" signal cleared by writes to
  prefix objects: your latch is that signal. Own it, document it, and tell pcall its name
  and semantics as soon as it exists.
- 0250 (open, not assigned) would add write tracking over the sealed region; don't build
  that here.

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
