# 0234: Memory footprint: idle and per-run linear memory vs QuickJS

status: done
agent: pmem
branch: issue/0234-memory-footprint
depends-on: -
guide: gc.md, limits.md, arena.md if behaviour changes
touches: heap (GC thresholds, arena growth), the base image layout, djs-wasm memory growth; research first

## Goal
Measured by benchpage (0229) against the best QuickJS wasm build (QuickJS 2026-06-04 / QuickJS-NG 0.18, wasi-sdk -O2), node 26; scripts in .worktrees/benchpage/work/scratch/0229/ops.ts, ops2.ts (need vendor/engines from scripts/bench-engines.sh --setup on that branch).
Linear memory after a run, djs vs QuickJS: idle 0.6 vs 0.3 MiB; sequential 0.9 vs 0.4; text
1.3 vs 0.6; json 1.8 vs 1.1; heavy 4.8 vs 2.4 MiB. Suspected causes: the base image in the arena,
the tracing GC's threshold (next collection at ~2x live) vs QuickJS's refcounting freeing at once,
a 950 KB __DATA string and split arrays kept until a collection, wasm memory growth granularity.
Research first: account for the bytes (base image, arena slack, malloc'd buffers outside the
arena, wasm pages), send the coordinator a short ranked list with costs (GC tuning trades CPU for
memory: measure both), then fix the clear wins.
Constraints: exact semantics (test262 and the difftest stay green); measure in wasm instructions on the shipped module before/after and report size cost (the shipped module has ~15 KB gzip headroom shared by 0230-0234: keep each change small, report bytes; the coordinator arbitrates); several agents work in djs-core at once (0230 builtin calls, 0231 arrays/strings/collections, 0232 async, 0233 regexp, 0234 memory): stay in your touches, ask the coordinator before changing shared paths (the interpreter's call path, the heap). Batch merges, gate checks locally, niced builds, benchmarks only at load < ~4.

## Log

### Research (scratch: work/scratch/0234/: mem.ts, trace.ts, grow.ts, atrace.ts, cpu.ts, nat.sh, nat2.sh)

Measured with the benchpage harness (js/bench/engines, copied untracked) in node 26. Linear
memory of a djs instance, idle (640 KiB): 128 KiB shadow stack; 97 KiB static data + bss,
padded to the 4 initial pages (256 KiB); 128 KiB dlmalloc first segment (PRIME, ~45 KiB
used); the arena at 384 KiB with 256 KiB reserved for 187 KiB used (base 182 KiB + sandbox
setup; image + one page of slack). QuickJS (wasi-sdk): 64 KiB stack, ~100 KiB data, ~130 KiB
malloc'd realm = 320 KiB.

After a run, memory = 384 KiB + arena capacity + holes where the arena moved:
- below the GC threshold the arena's capacity doubles (sequential: 276 KiB used, 512 KiB
  reserved); wasm grows it in place, so doubling only reserves untouched pages;
- the GC threshold is live + young live + 288 KiB (BASE_ALLOWANCE): ~2x live. heavy: the
  first collection comes at 2.4 MiB (JSON.parse of the 930 KB `__DATA` has no safepoint),
  2.2 MiB survive, 1.8 MiB of them the data strings (`__DATA` and the parsed answers), next
  threshold 4.4 MiB, arena capacity 4.4 MiB;
- fanout: its 151 KB non-Latin-1 data goes through the staging buffer (vm_decode_wire); the
  staging buffer is a page run right after the arena and stays at its high-water size, so
  the arena moves past it and leaves a 640 KiB hole;
- a full GC's 64 KiB mark stack reservation does not fit dlmalloc's segment: one more page
  at memory's end;
- minor collections keep every slice parent: heavy's `first` snippets keep each 10 KB file
  text until a full collection (~500 KiB).
Native instruction counts (djs run --body, data inlined) for the CPU side; wasm counts in
node are within V8 tier-up noise (±5%).

### Coordinator decisions (after the ranked list)
1 (arena growth), 2 (exact start), 3 (staging), 7 (mark stack reserve): approved, shared heap
edits included; one merge. 4+5 (GC growth discounting big leaf cells, minor collections detach
slices): do, within ≤2% CPU on every workload, or 5 alone. 6 (smaller allowance): no. Shadow
stack: no. Base image size: follow-up.

### Batch 1 (D3140-D3143)
Landed 1, 2, 3, 7. KiB of linear memory after a run (fresh sandbox, 2nd instance of the module;
mem.ts):

| | idle | seq | fanout | media | inspect | loop | text | json | heavy |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before | 640 | 896 | 2496 | 1024 | 640 | 896 | 1344 | 1856 | 4864 |
| after | 576 | 704 | 1792 | 1024 | 640 | 896 | 1216 | 1472 | 4864 |
| quickjs | 320 | 448 | 1664 | 1088 | 384 | 512 | 640 | 1152 | 2432 |

Wasm instructions per run (node, 20 runs minus 0, two rounds): within noise (±2%) on every
workload. Module +148 B gzip (shipped), +131 B (slim). Guide: arena.md (start, growth steps,
staging) updated; gc.md unaffected.
The mark stack reserve shows no change in these numbers: at heavy's size the other side tables
(bitmap 66 KiB, table 33 KiB, remembered list 16 KiB) already overflow the 128 KiB segment.

### Batch 2 (D3144)
Measured on a native release djs with runtime knobs (scratch: mat.sh, cli-knobs.patch), memory
in wasm (memrow.sh). Leaf discount variants, heavy KiB / worst CPU:
- 75% of leaves ≥ 1 KiB: 3584 / json-objects +5.6% (stringify output pieces, 13 → 18 GCs);
- 75% ≥ 16 KiB: 4160 / strbuild +2.2%; 100% ≥ 16 KiB: 3840 / strbuild +6.8%;
- 60% ≥ 16 KiB: 4480 / strbuild +1.6%, json-indent ~+1.5% (landed);
- 50% ≥ 16 KiB: 4672 / json-indent +1.5%.
Minor collections detaching slices (5): heavy and the others unchanged (heavy collects twice,
the first full), pi-like +0.9% (two walks over the marked cells in each of 151 minors): dropped
(coordinator). Wasm GC counts of the codemode workloads unchanged except heavy; wasm heavy
instructions ±0.3%. Module +56 B gzip. Guide: gc.md "When it runs" updated.

## Follow-ups

- **Survivor mark for big strings** (a format change): the leaf discount cannot tell
  long-lived big strings (tool output held for the run) from transient ones (a growing
  string's flattened copies, JSON.stringify's output pieces). A header or flag bit set on a
  large leaf that survived a collection (cleared by the canonical copy, allowed by the image
  verifier), or a `Roots` field with the previous collection's end, would let the discount
  apply to survivors only: 75% of leaves ≥ 1 KiB took heavy to 3584 KiB (QuickJS 2432) but
  cost json-objects 5.6% and strbuild 2.2% because of the transient ones.
- **Slice detach in minor collections** (bounded retention): minor collections keep every
  slice's parent, so a long run keeps the texts its snippets were cut from until the next
  full collection (every 16th). In an experiment with a lower allowance, heavy's 16th
  collection (full) dropped from 2862 to 2126 KiB, mostly 10 KB file texts kept by 80-char
  `first` snippets. Detaching young parents in minors (no second pass) costs pi-like +0.9%;
  worth it only together with cheaper slice resolution (one walk, or only when slices were
  marked with big parents).
- The base image (182 KB) against QuickJS's ~130 KB realm: the largest part of idle memory
  after the stack.
- The GC's side tables at multi-MB heaps (bitmap len/64, table len/128, remembered 16 KiB, mark
  stack) overflow dlmalloc's 128 KiB first segment and take a page run at memory's end.

- The base image (182 KB) against QuickJS's ~130 KB realm: the largest piece of idle memory
  after the stack.

## Result

Linear memory after a run (KiB; benchpage harness, node 26, fresh sandbox of a loaded module):

| | idle | seq | fanout | media | inspect | loop | text | json | heavy |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| djs before | 640 | 896 | 2496 | 1024 | 640 | 896 | 1344 | 1856 | 4864 |
| djs after | 576 | 704 | 1792 | 1024 | 640 | 896 | 1216 | 1472 | 4480 |
| quickjs 2026-06-04 | 320 | 448 | 1664 | 1088 | 384 | 512 | 640 | 1152 | 2432 |
| quickjs-ng 0.18 | 384 | 448 | 1600 | 1152 | 384 | 576 | 704 | 1344 | 2432 |

Landed: the arena grows by an eighth in pages (D3140) and starts at the image in pages, with
the page allocator's HUGE at 128 KiB (D3141); big staging buffers leave the arena room and do
not outlive their use (D3142); a 16 KiB mark stack reserve (D3143); large live strings count
for 40% of the next collection's growth (D3144). CPU: within noise in wasm; native strbuild
+1.6%. Module +204 B gzip in all.

What is structurally left of the gap (idle 576 vs 320): the 128 KiB shadow stack (QuickJS:
64 KiB; halving it halves the parser's nesting depth, a semantic change, declined); 97 KiB of
static data padded to 4 pages; dlmalloc's 128 KiB first segment (~45 KiB used; one page made
the arena move); the base image (182 KB vs ~130 KB). Per run, a tracing collector's threshold
(~2x the live data that is not big strings) against reference counting, which frees at once;
see the survivor-mark follow-up for heavy.
