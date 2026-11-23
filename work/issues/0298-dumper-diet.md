# 0298: The dumper's size: same dumps, far less code

status: in-progress
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

### 2026-11-23 pdiet: where the code goes, and the plan

Measured on d481d1fe: shipped module 1,875,273 raw / **767,666 gzip**. A names+DWARF build
(work/scratch/0298/dbg) with every code byte charged to the djs source line that caused
it (std glue inlined into a function charged to the last djs-core row before it;
work/scratch/0298/attr.py): the dump-only sources are 724 KB raw with their glue:

| file | raw with glue | lines | B/line |
| --- | ---: | ---: | ---: |
| vm/dump.rs (Dumper: walk, data, classes, check) | 236 K | 5,160 | 46 |
| vm/dump_cont.rs (cont::G) | 104 K | 3,257 | 32 |
| vm/dump_plain.rs (the writer) | 96 K | ~3,370 | 28 |
| vm/dump_flight.rs | 51 K | 2,785 | 18 |
| vm/dump_plain_inline.rs | 36 K | 1,522 | 24 |
| djs-wasm vm_dump & co | 26 K | | |
| compiler/smap_syntax.rs + smap.rs + smap_codec.rs | 52 K | | |
| dump_origin + cont_canon + cont_heads + cont_pattern + cont_using + reread | 75 K | | |
| compile_cache, registry, lazy, statics, state, classes, flight_* | 38 K | | |

Std glue inside vm::dump functions (by its own rows): raw_vec 68 K (String/Vec drops and
with_capacity), vec 39 K, ptr (drop/moves) 36 K, slice iterators 28 K, `&str ==` 20 K,
Result/Option `?` 35 K. Single biggest line: **dump.rs:49, the `cat!` macro: 90.8 K with
glue at 577 sites** (~157 B per site: the `&[&dyn Piece]` array, the call, and the drops
of the temporary Strings the pieces are). Next: `Dumper::note`, generic over its path
closure and inlined into each of its ~60 callers (dump.rs:1320-1393, ~30 K); generic std
instances: 32 sorts (most from `BTreeMap/BTreeSet: FromIterator`), 35 btree instances, 136
drop_glue instances; the plain writer's text scanning (scan/refs/keys_in/eager_*,
dump_plain_inline's text substitution, dump_reread's operand parsing, the LATER marks):
~30 K in their own functions, ~60 K with the glue of their callers.

Plan, in landing order; every step keeps the dump text **byte-identical** over the corpus
(difftest --plain-out, v2 and sleep outputs of d481d1fe in work/scratch/0298/base,
compared with diff -r), except where a step says it changes the output:

1. **Text building** (mechanical): `cat!` sites that cost less (pieces put straight into
   one String, no `dyn` array; String-returning helpers that a caller only concatenates
   become `*_to(&mut String)` writers), `note`/`edge` with a `&dyn` path (one copy), the
   `Arc<str>`/`String` caches borrowed where they can be. Expected ~50-70 K raw, ~20-25 KB
   gzip.
2. **Generic glue** (mechanical): one sort (crate::sort, already in the module) instead of
   32 instances; no BTreeMap/BTreeSet in dump-only code (sorted Vecs, the dumper's
   `Index`); fewer drop-glue instances (fewer distinct owned aggregates: `Vec<String>`
   pairs become indices or `&str`). Expected ~30-40 K raw, ~12-15 KB.
3. **Structured references** (structural, may change the output's `$dump` numbering, not
   what it does): the exporter records with each line what it references and defines
   (the nodes, factories, activations, globals it names) when it writes them, so the
   writer stops scanning its own text (`scan`, `refs`, `keys_in`, `eager_*`,
   `classify`, `split_def`, `resolve_later`'s LATER marks) and inlines single-use values
   by node, not by text search; dump_reread takes the operand's value from the frame
   instead of parsing its text back. Expected ~60-80 K raw, ~20-30 KB, the size of the
   scanners and their glue less what recording costs.
4. **`#[optimize(size)]`** on the dump-only functions (cfg'd: only under the shipped
   build's RUSTC_BOOTSTRAP, off with DJS_STABLE_ONLY=1), if it still pays after 1-3
   (measured before them: -17.3 KB), with the dump-time cost measured. A decision.

1+2+4 are ~50-60 KB gzip by these estimates; with 3, ~75-90 KB. A third (111 KB) or a half
(167 KB) needs the large functions themselves to shrink: the Dumper's emit_* / render /
visit_activation (~150 K raw) and cont::G (~100 K raw) are written as String-returning
expression builders with many small Vec<String> collections; a half would mean rewriting
them to append into one output buffer with borrowed slices throughout (a rewrite of
~10 K lines, the writer's scheduling and placement included), which I will scope after
steps 1-3 show how much of the size is that style rather than the logic.

## Follow-ups

## Result
