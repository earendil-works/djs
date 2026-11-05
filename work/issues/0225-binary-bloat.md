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

## Follow-ups

## Result
