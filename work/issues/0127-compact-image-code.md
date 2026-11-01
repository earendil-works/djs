# 0127: Size: compact image (internal names, bytecode density), containers, fast paths

status: in-progress
agent: perf
branch: issue/0127-compact-image-code
depends-on: 0122, 0123
guide: docs/guide/bytecode.md (if the encoding changes), docs/guide/snapshots.md / builtins.md as affected
touches: crates/djs-core/src/compiler/ (emit, peephole), crates/djs-core/src/vm/ (bytecode decoding, base image), crates/djs-prelude/, crates/djs-bgen/, collections/containers in djs-core; bloat (0126) owns Unicode tables and panic/fmt

## Goal
Bloat's report (docs/size-vs-quickjs.md) puts ~1.5x within reach without design changes,
via cuts 0122 didn't take:
1. Internal names in the image: 1,494 of 2,360 strings are prelude function / local
   names (52 KB raw, ~15 KB gzip). Keep only observable names (Function.prototype.name,
   stack frames as tested by test262 / corpus / pi suite); locals need no names in the
   shipped image.
2. Bytecode density for the prelude (short forms, narrower operands, a peephole pass,
   IC slots only where they pay): ~−9 KB gzip, and smaller user bytecode too; measure the
   interpreter cost (must not regress pi-like).
3. Out-of-line container growth (Vec push paths inlined everywhere): 10–15 KB.
4. Generated Rust fast paths kept only where benchmarks pay: 10–20 KB (prune with
   numbers, per D2020).

## Acceptance criteria
- [ ] Each cut measured (gzip via scripts/size, wasm instructions on all workloads); no
      workload slower than +1%, pi-like not slower, unless a trade is recorded.
- [ ] The size ratchet (tests/scores/size.tsv) lowered with each landing.
- [ ] Report the new ratio and what remains to 1.5x.

## Log

## Follow-ups

## Result
