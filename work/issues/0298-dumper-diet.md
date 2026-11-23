# 0298: The dumper's size: same dumps, far less code

status: open
agent: pdiet
branch: issue/0298-dumper-diet
depends-on: -
touches: crates/djs-core/src/vm/dump*.rs, crates/djs-core/src/vm/registry*, crates/djs-core/src/vm/compile_cache*, crates/djs-core/src/compiler/smap*.rs, crates/djs-core/src/compiler/lazy.rs, crates/djs-core/src/vm/mod.rs (dump parts), crates/djs-wasm/src/lib.rs (vm_dump), crates/djs-prelude (names), tests/dump/, tests/scores/size.tsv, docs/decisions.md, docs/guide/continuations.md, docs/guide/dumps.md
guide: docs/guide/continuations.md, docs/guide/dumps.md (how dumps are written), pages quoting module sizes

## Goal
Owner, 2026-11-23: the dumper is ripe for optimization. Since 0294 the one shipped module
carries it: 767.7 KB gzip, of which the `dump` feature is **334.6 KB** (the module without
it: 432.9 KB). Make the dumper much smaller while it writes dumps that load and continue
exactly as today's do (the gate's dump legs: plain dumps in djs and node, refusals ratchet,
dump-every-idle, golden dumps). Compatibility with today's dump text, the format-2
intermediate or anything older is **not** needed (0297, D024): change the dumper's
internals and, where it helps, its output.

## Accounting (coordinator, main 706c0923; /Users/mitsuhiko/Development/djs/work/scratch/size-audit/NOTES.md)
Raw code from a names build (~6% LEB padding over the stripped module):
- `vm::dump`: 635 KB raw, ~225 KB gzip marginal: `Dumper` (heap walk, activations,
  finish, emit_*, render, flight) 283 KB; `cont::G` (continuation generator) 184 KB;
  `plain` (the writer: placement, scheduling, inlining) 137 KB.
- dump-only code outside it, 209 KB raw: Vm (compile_cached 16K, ...) 26K, compiler::smap
  20K + smap_syntax 6K, parser site recording +18.5K, registry 7.6K, vm_dump 7.3K, lazy
  5.7K; and generic std glue: slice::sort 21K in **32** instances, BTreeMap/BTreeSet 12.5K
  in 35, drop_glue 13.6K in 101, Vec 7.3K, iter adapters 7.1K, int formatting 4.3K,
  str::pattern 3.2K, f64 parsing 1.1K.
- data: +67 KB raw / +30 KB gzip (messages, the runtime helpers' JS text, BASE_NAMES
  17 KB raw / 3.8 KB gz).
- Density: the dump sources are ~26K non-test lines at ~24 B of code per line; the rest of
  djs-core ~11 B/line. The dumper works on **text**: the exporter writes format-2-shaped
  lines that no host loads (D2860, D2888), the plain writer re-scans them for `$dump.X`
  references (`plain::scan`, `refs`, `scan_spans`), inlines at the text level
  (dump_plain_inline.rs), re-reads operands (dump_reread.rs); ~245 String constructions,
  362 `.clone()`, 123 `Vec<String>`.
- Measured variants (scripts/size/variant.sh, gzip of code+data): `#[optimize(size)]` on
  vm/dump*.rs −16.6 KB; plus registry/compile_cache/smap/lazy −17.3 KB. Whole module
  opt-level z −47 KB (not wanted: +30–70% instructions).

## Acceptance criteria
- [ ] A plan in the Log first: where the dumper's code goes (finer than above), and which
      structural changes buy what, before the large edits.
- [ ] Structural: no intermediate format-2 lines and no re-scanning of the dumper's own
      text where a structured form (values, references, statements recorded when they
      are made) can carry the same information to the writer.
- [ ] Mechanical: one sort instance (or a few) instead of 32, no BTreeMap/BTreeSet where a
      sorted Vec or the engine's maps do, less String churn (borrowed slices, one output
      buffer), no float parsing of its own text where the value is known.
- [ ] `#[optimize(size)]` on dump-only code if it still pays after the above (nightly
      attribute under the shipped build's RUSTC_BOOTSTRAP, cfg'd out under
      DJS_STABLE_ONLY=1; record a decision), with the dump-time cost measured.
- [ ] Dumps still load and continue: every dump gate leg green, refusals not grown. Dump
      time and dump size (tests/scores/dump-size.tsv) not worse beyond noise; say what changed.
- [ ] Module gzip before/after per landed step; ceiling lowered each time
      (`scripts/size/budget.py MODULE --update`). Target: at least a third of the 334.6 KB
      off; say what a half would take.
- [ ] Land in steps (several merges are fine and preferred): each one gated, each with its
      measured saving in the Log.

## Notes
- 0297 runs at the same time over the rest of the repository; it does not touch the files
  in `touches:` here and sends dumper-side compatibility leftovers to the coordinator,
  who forwards them to you.
- Tools: /Users/mitsuhiko/Development/djs/work/scratch/size-audit/{funcs.py,attr.py,diff.py,optsize-files.py} (per-function
  sizes, marginal gzip per path, diff between builds, the optsize variant patch);
  scripts/wasm-size.py, scripts/size/. Build with names:
  `CARGO_PROFILE_WASM_STRIP=false` (see scripts/size/README.md).

## Log

## Follow-ups

## Result
