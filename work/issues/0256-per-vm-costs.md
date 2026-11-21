# 0256: Per-VM and per-byte costs: base injection, hash keys, zeroing

status: done
agent: pstart
branch: issue/0256-per-vm-costs
depends-on: -
touches: crates/djs-wasm/src/lib.rs (base injection), crates/djs-wasm/src/allocator (a grow-without-zeroing hook only), crates/djs-prelude (is_base), crates/djs-core/src/heap/arena.rs (zero_ahead), crates/djs-core/src/heap/string.rs (set_hash_key only), js/src/base.ts, js/src/sandbox.ts (base injection / seed only)
guide: arena.md, snapshots.md if what a new VM does changes
decisions: D3270-D3279

## Goal
Fixed costs every VM and every allocated byte pays, found by the 2026-11-21 profiles
(IDEAS.md 0.4 and idea 4; BRIEF.md):
1. `djs_base_inject_ptr` zero-fills a 182 KB buffer that the host overwrites right after,
   and `djs_base_inject` hashes all of it with fnv64 (~3% of `sequential`; 2.6% for the
   inject alloc). The host can already write anywhere in the instance's memory, so the
   hash protects nothing the length + a build id check would not. Remove the zero-fill and
   the full hash (keep a cheap identity check).
2. `Heap::set_hash_key` walks every arena cell twice per VM at `vm_set_random_seed`
   (~3% of the smallest bodies, ~12 µs per VM): walk only what lies above the base's end,
   with the invariant asserted where the base image is built.
3. `Arena::zero_ahead` zeroes every arena byte before first use: 1.6% sequential to 6.9%
   fanout. Fresh pages from memory.grow are zero already, and string payloads are
   overwritten at once. Track a dirty high-water mark so only reused bytes are zeroed;
   leaf payloads skip zeroing. Growing the Vec without writing needs an embedder hook in
   djs-wasm (its allocator knows fresh pages); djs-core stays without new unsafe.
Then look for anything else that runs per VM or per byte in the profiles of the small
bodies (sequential, inspect, media) and is not the script's own work.

## Acceptance criteria
- [x] new-VM time and the run times of sequential/inspect/media/fanout measured main vs branch (wasm, run.ts)
- [x] the three items landed or measured and dropped with numbers
- [x] images, determinism and the image verifier unchanged in meaning (bytes above len still read as zero)
- [x] size reported; gate green; decisions recorded

## Notes
- Overlap: 0257 (pidle) changes djs-wasm's linking, stack and dlmalloc's first segment in
  the allocator; you add only the grow hook there. Tell each other before touching the
  other's lines.
- 0258 (pgc) works in heap/compactor.rs and gc.rs; zero_ahead's interaction with
  collections (truncation) is yours, the collector's policy is theirs.

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
- 2026-11-21: harness work/scratch/0256/ab.ts (primary checkout): two modules, interleaved
  fresh instance + VM per run, median ms; `newvm` is a Sandbox created and disposed. A/A at
  load 8: within 1%.
- Hacks first, to size each item (load 6-9, A/B interleaved, 300 reps): inject without
  zero-fill and hash: newvm −21%, sequential/inspect/media −6-7%, fanout −2%; then
  set_hash_key from BASE_END: newvm another −15% (≈20 µs), small bodies −4-5%. zero_ahead
  as a crude no-op above a global high-water mark: ≤3% on the small bodies, within noise
  elsewhere (and the hack trapped on fanout: the real thing needs the allocator's
  knowledge, see below).
- Batch 1 (aa350a92): D3270 (`set_hash_key` walks from `BASE_END`, `prepare_base_image`
  asserts no Map/Set in the prefix), D3271 (`djs_base_inject_ptr` does not zero, inject sets
  the length, `is_base` = length + FNV sample of the roots and one word per 504 B against a
  build-time `BASE_SAMPLE`). Tests: base tests corrupt a sampled byte now. host.md updated.
  Size 428,800 → 428,841 B gz (+41). djs-core lib, djs-prelude, djs-wasm, js tests (dev
  modules) pass.
- Merged batch 1 (main 78d993b5). A/B main vs branch, 600 interleaved runs, load ~18:
  new sandbox 0.182 → 0.130 ms (−28%), sequential −8%, inspect −7%, media −6%, fanout −3.4%.
- Item 3 built properly (patch work/scratch/0256/item3-v2.patch): `VmOptions::extend_zeroed`
  hook, Arena `high` mark (set at truncate), page allocator `fresh` page mark + top block's
  zero suffix. Counted: skips 112 KB (sequential) to 820 KB (json) of memset per run. Time:
  −1.1..+0.5%, noise (1000 runs, load ~18); +440 B gz. Dropped (D3272): the profile share is
  first-touch page faults (synthetic: touching 8 fresh pages after memory.grow costs 53 µs,
  ~6.6 µs per 64 KiB page), which move to the cells' first writes.
- Per new sandbox now (steps.ts/calls.ts, b1): Instance ~100 µs (V8; synthetic modules show
  ~0.3 µs per export, 65 exports ≈ 20 µs; data segment and memory the rest), provideBase
  copy ~21 µs (faults of the base's pages), wasm calls ~23 µs (inject_ptr 8: allocator init
  and two memory.grow; vm_define ×5 and vm_new_host_function ×4 ≈ 1 µs each, mostly first
  touch), Sandbox JS ~30 µs.
- Batch 2 (1d8fc522): entropy pool in sandbox.ts (getRandomValues 1-4 µs per call, ~1% of
  sequential in the profile; D3273); `base_len` a build-time constant. Size 428,737 B gz.
  Merged (main 9199a7ce; one gate retry: pi's worker-lifetime timing test, 1028 ms > 1000
  at load 53).
- Instantiation anatomy (work/scratch/0256/strip.py rewrites the module; inst4.ts, load
  25-40, so absolute µs inflated): all exports but one removed −47..−53 µs, data segments
  emptied −45..−80 µs, both −80..−100 µs of ~180-235 µs. Exports and the 97 KB data
  segment are each about a quarter of `new WebAssembly.Instance`.
- memory.grow calls per run (counter build): sequential 4 (7 pages), inspect 3, media 5,
  fanout 7, json 9, text 10; ~2 µs each.
- Final A/B (work/scratch/0256/final-ab.txt): main a0d85b06 vs branch, same JS, 1000
  interleaved fresh sandboxes per row, load 24: new sandbox 0.194 → 0.141 ms (−27.6%),
  sequential −8.6%, inspect −8.9%, media −7.5%, fanout −2.3%, loop −2.3%, text −1.1%, json
  −1.8%, heavy +0.7% (noise). run.ts (25 reps ×4 alternations, load ~20, dumps skipped):
  startup 0.259 → 0.205 ms; run-only times and memory after the run unchanged (the noise
  at that load is ±10%).

## Follow-ups
- Exports cost every instance ~0.3-0.8 µs each in V8 (65 now, ≈ a quarter of
  instantiation): fewer exports (rarely used ones folded into a dispatcher, or a separate
  module for tooling) is an ABI decision.
- The data segment (97 KB) is another quarter of instantiation (copy and first touch): the
  idle-memory work (0257, idea 11: Unicode tables out of linear memory) also buys startup.
- First touch of a fresh wasm page costs ~6.6 µs per 64 KiB (D3272): fewer pages touched
  per run is a speed lever too (arena start capacity, zero-ahead distance, dlmalloc segment).
- js/pi/vm.ts has its own per-VM `getRandomValues` entropy(); the same pool as D3273.
- `Heap::intern` of a rope compares candidates through `str_units` iterators
  (`str_eq` → FlatMap<Chunks, Units>): 0.8% of sequential; flatten (or compare chunks)
  first.
- Sandbox JS setup is ~30 µs of a new sandbox: 4 console host functions and 5 defines
  (~1 µs each, mostly first touch of the arena tail), `stageText` per name; one batched
  call would do.

## Result
Landed (main 78d993b5, 9199a7ce):
- D3271: `djs_base_inject_ptr` hands out the buffer without zeroing it; `djs_base_inject`
  sets its length and checks identity by length + an FNV-1a sample (roots + one word per
  504 B) against a build-time `BASE_SAMPLE`, not a hash of all 184 KB.
- D3270: `set_hash_key` walks from `BASE_END`; `prepare_base_image` asserts no Map/Set in
  the prefix.
- D3273: the JS Sandbox seeds from a 256-word entropy pool; `base_len` is a constant.
- D3272: zeroing over fresh pages measured and dropped (page faults, not memset, are the
  cost; patch in work/scratch/0256/item3-v2.patch).
Numbers: new sandbox −27.6% (0.194 → 0.141 ms), sequential/inspect/media −7.5..−8.9%,
fanout/loop/json −2%; memory unchanged; images and the base unchanged. Size 428,800 →
428,737 B gz (−63). Guide: host.md (the inject check). Known gaps: see Follow-ups (the
rest of a new sandbox is V8's instantiation, ~70% of it).
