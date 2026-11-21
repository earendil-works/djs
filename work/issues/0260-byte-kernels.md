# 0260: Byte kernels: search, JSON scanning and splitting, word at a time

status: in-progress
agent: pscan
branch: issue/0260-byte-kernels
depends-on: -
touches: crates/djs-core/src/builtins/string.rs (find_latin1, index_of, split helpers), crates/djs-core/src/json (parse scanners, write_escaped_*), crates/djs-core/src/regexp (literal-prefix scanning only), a small shared byte-scan module
guide: builtins.md if a fast-path story changes
decisions: D3310-D3319

## Goal
heavy spends >20% in byte kernels (find_latin1 7.5%, json write_escaped_latin1 8.1%,
plain_run 5.3%, scan_rest, str_view), fanout ~29% in json, media 21%. These are
byte-at-a-time loops. Make them word-at-a-time:
1. SWAR over u64 (safe Rust, no target feature, works natively and in wasm): finding a
   byte, finding "a byte that needs escaping / ends a JSON string run", memchr-style
   two-byte search for includes/indexOf, newline splitting.
2. wasm simd128 is a separate question: D016 and 0077 rule it out by default (it drops
   SIMD-less runtimes). Measure what simd128 would add over SWAR on the same kernels in a
   scratch build and report the numbers for the owner; do not ship it without a decision.
Keep exact semantics (WTF-16, Latin-1 vs two-byte strings).

## Acceptance criteria
- [ ] per-kernel instructions (wmicro) and heavy/fanout/media/text/json run times, main vs branch
- [ ] tests for edge cases: alignment, lengths 0-17, matches at every offset, non-ASCII Latin-1, two-byte strings
- [ ] size reported (kernels should be small); gate green; decisions recorded
- [ ] the simd128 numbers written up in the issue's Result

## Notes
- Overlap: 0256 (pstart) touches heap/string.rs's set_hash_key only; you touch the search
  helpers. Keep to your functions.
- The JSON parse/stringify paths are also what the benchmark's tool bridge uses, so the
  workloads show them directly.

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
- 2026-11-21: Measured first (main a0d85b06, wasm keep-names profiles, kernels in
  work/scratch/0260/: k-*.js through kt.ts (interleaved min-of-R µs) and kern.py/ins.py
  (instructions, `--single-threaded --no-liftoff` for kernels)). The kernels were already
  SWAR (find_latin1, find_byte, plain_run, stringify's quote); the costs were elsewhere:
  scan_string a byte loop up to the first escape (media's 177 KB base64 string), per
  escape a jump-table match plus a find_byte call plus a `memory.copy` (V8 calls out for
  it: 4-6 ns vs 1 ns for a word store at 8-16 B), two-byte input unit by unit (fanout's
  __DATA has a ✓, so every string of it), find_latin1's byte tail on short lines, the
  regexp first set a bitmap test per byte. Instruction counts miss the mispredict wins
  (escape table, find tail): use time.
- Split into 8 commits with cumulative size and effect (coordinator's request); steps 1-4
  merged (D3310), 5-8 held on branch 0260-rest for the owner's budget answer:
  | step | gz B | effect |
  | --- | ---: | --- |
  | 1 swar + overlapping tail | +17 | find_latin1 self −30% (includes kernel) |
  | 2 one-byte scan | +113 | media −14% instr / −10% time, json −1.4% |
  | 3 escape table | +3 (−602 raw) | mispredicts |
  | 4 fused escaped writer | +32 | heavy −2.6%, fanout −2% |
  | 5 two-byte scan | +221 | fanout −8.5% instr / −6% time |
  | 6 two-byte escaped writer | +202 | fanout −0.6% (kernel −20%) |
  | 7 regexp first set 1-4 bytes | +377 | heavy −3% |
  | 8 multiline ^ | +57 | text −2.8% |
  All 8 (wall, load ~12): heavy 0.95, fanout 0.90, media 0.83, text 0.96; +1,022 B.
- Merged steps 1-4 (84ed9975). Offsets in json (D3311): −205 B gz.
- simd128 (scratch only, patches work/scratch/0260/simd-hand*.patch): see Result.
- Guide: no page describes these kernels (builtins.md lists the fast entries only); none
  updated.

## Follow-ups
- Branch 0260-rest (held for the size budget, rebased on 5a3891f6): two-byte JSON scan
  (+156 B gz) and escaped writer (+189): fanout −8%; regexp first sets of one or two
  bytes and multiline `^` through `swar::find_either` (+349): heavy −5%, text −4%.
- The `+simd128` target feature alone shrinks the shipped module by ~764 B gz at equal
  speed (autovectorization; scratch build): an owner question against D016.
- includes/indexOf on short lines are ~80% call overhead (run_hot, decline_fast,
  str_view, shape lookups): 0259's area, not the kernel.
- Escape-dense JSON still pays ~12 ns per escape (two passes, a mispredicted run end in
  each); a scan that records escape positions for the writer could halve it.

## Result
Landed (main 84ed9975, 5a3891f6; D3310, D3311): `crates/djs-core/src/swar.rs` (lane
tests, `find_byte`), find_latin1/find_byte overlapping last word, one-byte JSON strings
scanned a word at a time from the first byte, the escape table, the one-pass escaped
writer, size offsets in json. Shipped module 429,353 → 428,963 B gz on balance (+139 for
the kernels, −205 offsets, other merges in between).

Numbers (wasm in node 26; instructions per run are (I(R)−I(0))/R; wall is interleaved
min-of-15; load 10-50, so wall ±3-5%):
| workload | instr | wall |
| --- | ---: | ---: |
| media | −15% | −17% |
| heavy | −2.6% | −3% |
| fanout | −4% | −4% |
| json | −1.4% | ~0 |
| text, loop, inspect, sequential | ~0 | ~0 |
Kernels (µs per iteration, wall): JSON.parse of an 80 KB plain text 9.9 vs 52 (0.19x),
escaped text (3,266 escapes) 0.87x, find_latin1 self time in an includes loop −30%.
Held on 0260-rest (see Follow-ups): fanout −8%, heavy −5%, text −4% more for +694 B.

simd128 (scratch, patches in work/scratch/0260/simd-hand*.patch; same code as main
otherwise):
- `-Ctarget-feature=+simd128` alone: −764 B gz (428,404 vs 429,168), workloads within
  ±3% (heavy 1.02, fanout 0.98, media 0.96, json 1.00, text 0.99 min-time ratios).
- Hand-written v128 kernels on top (16-byte JSON scan, fused escaped copy, find_byte,
  find_latin1 with 16 candidates): plain JSON scan 0.60x of SWAR, a 1-escape 80 KB string
  0.84x, but escape-dense text 1.08-1.25x slower (`i8x16.bitmask` costs more than a word
  test when the run ends in the first 16 bytes; a SWAR first word then SIMD halves that
  loss) and the workloads 0.99-1.04x. SWAR already scans long runs at ~10 GB/s; the
  workloads' costs are per escape and per call, which SIMD does not reduce. Not worth
  dropping SIMD-less runtimes for; only the flag's size effect is.
