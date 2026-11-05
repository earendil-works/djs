# 0225: Binary bloat: where the shipped module's bytes go and how to win them back

status: in-progress
agent: bytes
branch: issue/0225-binary-bloat
depends-on: 0223
guide: pages quoting sizes (generated via status.tsv)
touches: research first (work/research/0225/); then djs-core/djs-wasm/build config cuts, scripts/size/, tests/scores/size.tsv

## Goal (owner: why do we waste so much binary space now)
0223 measured the shipped module at 1,034 KB raw / 443 KB gzip (ceiling 442,831; the budget D020
is 1.5x quickjs-wasi's 286,655 = 430 KB, so djs is over budget), slim 906 KB / 378 KB, and the dump
module 1,966 KB / 785 KB. Every recent feature had to scrape bytes to fit.

1. **Account for every byte** of the shipped module: per crate/module/function (twiggy or the
   existing scripts/size/ tools: areas.py, families.py, sections.py, datamap.py, vec-sites.py,
   bytecode-density.py), data vs code, the base image, generic monomorphization duplicates,
   panics/formatting, tables, builtins (Rust vs bytecode), regexp, unicode tables, TS stripping,
   dump-loading code, lazy/compile-cache cfgs, debug info and custom sections. Compare with the
   size history (tests/scores/size.tsv commits, D2892 and earlier trades) to see what grew since
   the last lean point and why.
2. **Rank the waste**: duplicate monomorphizations, code only some hosts need that could be
   feature-gated or moved to the dump module, cold paths that could be bytecode instead of Rust,
   oversized tables, dead code, build settings (opt-level s/z per crate, lto, codegen-units,
   wasm-opt passes, panic=abort, location-detail, strip). Estimate each saving (gzip) and its cost.
3. Send the coordinator a short summary with the ranked list before big changes; then implement
   the clear wins in batched merges, lowering the ceilings (`--update`) as bytes come off.
   Target: well under the D020 budget (430 KB gzip), with real headroom for features.

Constraints: 0224 (perf regressions) runs at the same time; size cuts must not cost speed without
numbers and the coordinator's OK (wasm instruction counts per workload as in 0223). Batched merges.

## Log
- Accounting and history in work/research/0225/README.md: code 336.9 KB gz, data 56.6
  (Unicode 32.3), coded base 45.7. No net growth since 0161 (+2.2 code, −1.5 base);
  never within budget (510 KB at 0122). Ranked list sent to the coordinator; decisions:
  #1 build-std approved, #5 go, #4 prototype + instantiate numbers first, #2 after 0224,
  #3 to the owner, #6 cheap ones after #1/#5.
- Batch 1: D3090 (build-std core/alloc, `-Cpanic=immediate-abort`,
  `-Zshare-generics=no`; −10.8 KB gz, instructions −3.1..+2.3%; trap.ts says "engine
  panic"; AGENTS.md, README requirements, docs.yml rust-src, guide host.md) and D3091
  (intrinsic run-time tables, −0.8 KB). Shipped 442,576 → 430,963; slim 377.2 → 365.3;
  dump module 784.7 → 762.8. Ceilings lowered; status sizes re-recorded. Guide pages:
  host.md prose (panics); generated numbers in README, host.md, dumps.md, status.md.
- #4 shelved (coordinator): data segment lzrc −5.8 KB gz, per instance ±0 (29–31 µs
  either way), +~3 ms per module (first instance), every build path and host would need a
  rewrite step / data restore. Written up in work/research/0225.
- #6: `#[optimize(size)]` on compiler/lexer/bigint/wire/regexp parser+compiler: −2.5 KB,
  compile ±1% (typescript.js +1.0%, bodies −1..−2%), but on unicode +23% on case
  mapping/normalization (excluded) and ~750 per-function attributes (rustc rejects it
  on modules/impls) behind a cfg for stable: not done. Unicode re-packing: the tables are
  already varint/delta-coded (D2334, D1760), lzrc of all of them saves only 3.1 KB, so
  the encoding is near its entropy: not done.
- Batch 2: D3092 (opcode names apart from OPS; −1.6 KB). Shipped 429,334 (1.498x,
  within the D020 budget), slim 363,857, dump 760,897. Measured and dropped: libm::fmod
  → `%` (−888 raw, −12 gz: gzip absorbs identical bodies, so duplicate monomorphs cost
  little), `-Zmerge-functions=trampolines` (+41), `-Zinline-mir=no` (0),
  `-Zmir-opt-level=1` (+353). Cell layout names: −1.3 KB measured (variant), but
  for_each_slot (GC) reads those tables: deferred while 0224 bisects GC.
- Batch 3: D3093 (about 40 compiler intrinsic lookups by name → `intrinsic_id_of!`,
  const): typescript.js compile −6.2% instructions, bodies −0.2..−2.5%, −0.8 KB gz;
  shipped 428,507.
- #2 prototyped (work/research/0225/niche.md): Value niche + zero-sized Fault is +2.6 KB
  (unchecked, lower bound) to +4.0 KB (checked) and +0.6..+8.0% instructions; the
  zero-sized Fault alone −0.2 KB. Recommendation: don't build it.
- Open: cell layout names (−1.3 KB measured) after perfreg's GC batch; unsafe-heap with
  the owner; #4 shelved.
- Batch 4 (top-50 review, rebased on 0224's gc batch, main 428,933 incl. its +1,003 B):
  D3094 (wasm builds carry the prelude's `%Install*` intrinsics and their helpers only
  with djs-core's `prelude` feature; −4.9 KB) and D3095 (cell layouts and their names out
  of the run time, `CellKind::min_size()`; −1.7 KB). Shipped 422,316 (1.473x), slim
  356,917. Instructions −1.1..+0.4% on the 12 workloads, typescript.js compile ±0.
  Measured and dropped in the review: one shared `bad_argument` exit in
  `call_intrinsic` (−0.4 KB, but re-indents the whole 1,600-line dispatch); arena checks
  in u32 (`saturating_sub` +9.1 KB, `wrapping_sub` +6.1 KB: LLVM keeps the slice checks,
  as D2520 found); the JSON serializer natives (kept by D2310's group decision). The rest
  of the top 50 is diffuse: inlined drop glue (1,014 dealloc sites in 328 functions),
  `?` conversions of HeapError (355 sites), arena bounds checks (unsafe-heap, with the
  owner).

## Follow-ups

## Result
