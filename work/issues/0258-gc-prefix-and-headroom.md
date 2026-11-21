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

### Batch 1 (D3290, D3291)
- Latch `ROOT_PREFIX_WRITTEN` (Roots::FLAGS bit 8, sticky, image state), set only in
  `Heap::set_prefix_written` (0259 hooks there). General property ops latch through
  `refuse_write`/`note_write`; fast writers compare `gate_end` and decline (no new call in
  run_hot / IC hits). `mark_compact` skips `[roots end, BASE_END)` while unlatched, in full
  and minor collections. Debug builds verify at each skipping collection that the prefix
  references nothing ≥ BASE_END. That check, under a gc-stress hack in the test262 runner
  (work/scratch/0258/t262-stress-hack.patch), found create_data_property's new-key fast path
  writing builtins unlatched (also e2e bigint.js in gc mode); fixed.
- Instructions per run (wasm, node 26, fresh VMs, ±1–2% noise, load 17–60): fanout −1.5%,
  media −2%, text −2%, json −0.6%, heavy −0.4%; wmeasure props −1.8, pi-like −1.3,
  json-objects −1.3. Shipped module +~370 B gz (429,167 vs 428,796).
- Codemode runs collect only 0–3 times (trace: work/scratch/0258/trace.ts with
  trace-full.patch). heavy's 3rd threshold was crossed by 416 B and the arena's "past target:
  +1/4" step reserved 864 KB: D3291 grows by half the overshoot. KiB after a run, main →
  branch: fanout 1792→1664, json 1472→1408, ts 1664→1536, heavy 4736→3904; others equal.
- Allowance sweep (before D3291; KiB after run): BASE_ALLOWANCE=MIN_GROWTH 224: loop −64,
  json −192, ts −256, heavy −512; 192: also text −64; ≤160: fanout +192. json/ts +2–4%
  instructions (one more collection). To be redone on top of D3291.
- Guide: gc.md (skipping the prefix), arena.md (growth past the threshold).

### Batch 2 (D3292-D3294)
- D3292 (negative): allowance retune after D3291 is −0.3..−0.8% memory geomean for +2–4%
  instr on json/ts; fanout's ≤160 KiB cliff is an arena move behind a compile peak (pstart).
  Survivor-only big-string discount made heavy worse (3840 → 3904–3968). Patch:
  work/scratch/0258/survivor-discount-wip.patch.
- D3293 (landed): growth past 1 MiB counts half. json-objects 7.1→5.8 MiB, strbuild
  24.2→11.9, jsonh 4.3→3.7, json 4.4→3.9; instr +0..3.3%; codemode equal (heavy +128 KiB
  by chance). Module −6 B gz.
- D3294 (measured, not built): fixed nursery without barrier: heavy −13% but +5–9% instr.
  Slice parents don't set heavy's peak.
- Experiment scaffolding in work/scratch/0258: trace.py on/off (GC + page trace, trace.ts
  reads it), vbuild.sh + v-*.py variant builds, mem.sh/perturb.sh/instr.sh.

## Follow-ups

## Result
