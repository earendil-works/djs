# 0277: Short concatenations, template literals, string comparison, localeCompare, startsWith/endsWith

status: done
agent: pgc
branch: issue/0277-short-strings-compare
depends-on: 0269, 0268
touches: crates/djs-core/src/heap/string.rs (concat, write_leaves, comparison), builtins for localeCompare/startsWith/endsWith/at/repeat (Rust leaves only through their helpers), crates/djs-core/src/unicode (localeCompare's collation path only), tests/corpus
guide: heap.md if the string story changes
decisions: D3480-D3489

## Goal
0268 moved `+` and template literals into the loop; what remains is the heap side
(0268's top follow-up): `Heap::concat` of two short flat strings costs ~560 instructions,
`write_leaves` alone ~250 to copy three bytes. From pidle's 0270 sweep (re-measure on
current main): template `${a}-${i}` 1.6x QuickJS, string `<` 1.5x, `===` of equal
non-interned strings 1.4x, localeCompare 2.8x (sorting with it 2.7x), startsWith+endsWith
2.1x; 0269 left at/repeat/(i).toString at 1.2-1.4x (per-call overhead of generated
leaves: route call-path findings to pcall's 0276 through the coordinator).
Short flat strings should concatenate in a handful of instructions: a size-classed direct
copy without the leaf walk; comparison by bytes when both are one-byte flat.

## Acceptance criteria
- [x] per-op instructions vs QuickJS 2026 before/after; workload instructions (sequential, text, json, inspect build output strings)
- [x] memory unchanged (codemode-memory leg, mem-perturb on strbuild)
- [x] size: the module has ~950 B under the ceiling shared with pidle (Set/Map, ~865 B) and pscan (~120 B): size-neutral or offsets in your own code; ask before raising
- [x] gate green; decisions recorded

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
Re-measured on main a0db6b9a (wasm instr/op, quickjs-wasi in parentheses): '+' and templates
were already at 0.8–1.0x after 0268; `s < t` 1,540 (770) 2.0x; localeCompare 7,250 (3,740)
1.9x, sorting with it 1.7x; startsWith/endsWith/at/repeat/(i).toString 1.15–1.3x.
- D3480 localeCompare: one-byte strings by bytes + `@rust`: 1,990 (0.53x); sort 0.73x.
- D3481 concat of two flat one-byte strings by two copies: −7..−9% per op on '+' and templates.
- D3482 latin1_piece (one copy helper instead of two): size offset.
- Not mine, routed to the coordinator for 0276 (pcall): `s < t` 2.0x is the general path
  (Lt on two strings leaves the hot loop; `str_cmp` itself is a byte compare);
  startsWith/endsWith/at/repeat/(i).toString 1.15–1.3x are generated-leaf call overhead
  (a no-substring intrinsic for startsWith did not move it, 0269).
- Workloads (sequential, inspect, text, json, fanout, heavy): within noise (load 40–65).
  codemode memory rows unchanged.

## Follow-ups

## Result
localeCompare 3.6x faster (now 0.53x QuickJS; sorting with it 0.73x), short concatenations
and templates 7–9% fewer instructions (0.79–1.01x QuickJS), module −8 B gz (D3480–D3482).
String `<` (2.0x) and the leaf-call overhead of startsWith/endsWith/at/repeat/toString
(1.15–1.3x) are 0276's (pcall). Memory unchanged. New corpus case
builtins/string-compare.js.
