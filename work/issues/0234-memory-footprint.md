# 0234: Memory footprint: idle and per-run linear memory vs QuickJS

status: in-progress
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

## Follow-ups

## Result
