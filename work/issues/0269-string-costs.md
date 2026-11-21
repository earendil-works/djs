# 0269: String costs: views, short results, slices, appends

status: in-progress
agent: pgc
branch: issue/0269-string-costs
depends-on: 0258
touches: crates/djs-core/src/heap/string.rs (views, flat/rope/slice access, short allocation, append buffers), crates/djs-core/src/heap/arena.rs (small allocation paths only), string builtins' Rust leaves where they allocate results (builtins/string.rs, generated leaves only through their helpers), tests/corpus
guide: heap.md, values.md if the string story changes
decisions: D3400-D3409

## Goal
The "string" bucket is 20% of heavy and fanout, ~10% of sequential/text/json (BRIEF.md
profiles): str_view, str_view_flat, substring_slice, append_rope_buffer,
alloc_latin1_pieces, write_leaves, content_hash_with. The ops table's remaining gaps on
tiny string ops (trim 1.35x, slice 1.32x, charCodeAt loop 1.43x, template appends 1.20x,
String(i) ~1.1x after 0259) are, per the benchmarks page, mostly allocating the short
result string and getting a view of the input. Profile per op (wmicro) and per workload,
then cut: the cost of obtaining a view (kind dispatch, rope/slice resolution per call),
short-string allocation (header, size class, zeroing), slice creation vs copying for short
results, the append buffer's growth, hashing of strings used as keys.

## Acceptance criteria
- [ ] per-op instructions vs QuickJS and workload instructions before/after
- [ ] memory unchanged or better (slices vs copies trade memory: measure heavy, mem-perturb)
- [ ] exact semantics (WTF-16, one-byte/two-byte), corpus + test262 built-ins/String; images and dumps unchanged in meaning
- [ ] size reported (~2.3 KB room under the ceiling, shared); gate green; decisions recorded

## Notes
- pscan (0267) owns JSON and regexp scanning (swar.rs); pcall (0268) the call path and
  run_hot; pstart (0265) the compiler and big transient buffers (it may move
  Array.prototype.join's buffer into a reserved arena string: coordinate on string
  reservation helpers through the coordinator); pidle (0266) promises/Error.

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

### Profile (main 62cd4b66; scratch: work/scratch/0269 wmicro.py with cases*.py, bucket.py)
Tiny ops are now at or below QuickJS (trim 0.96, slice 0.91, charCodeAt 0.92, includes 0.6,
join 0.62, String(i) 0.85). Gaps: replace 3.4x, replaceAll 5.3x, substr 5.2x, o[k] string key
2.5–3.6x, repeat 1.35, startsWith 1.33, endsWith/at 1.24, (i).toString 1.19, template 1.16.
String bucket: heavy 19% of djs samples (find_latin1 6%: pscan's kernel), fanout 18%, text 10%.

### Batch 1 (D3400, D3401)
- concat_ranges (rope leaves copied once): fanout −3.0%, sequential −4.5%.
- @fast replace/replaceAll: 3.4x/5.3x → 1.2x/1.1x of QuickJS; text −1.5..−3%.
- slice views first: heavy −1.5%. str_array_index from the view: o[k] −160 instr.
- Size: +~700 B gz (428,178 vs 427,478). Guide: heap.md (ropes read in place).

## Follow-ups

## Result
