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

### 2026-11-23 pdiet: merge 1 (steps 0-2, mechanical), 767,666 -> 734,118 B gzip (-33.5 KB)

Rebased onto 0297's ABI 9 (6d558410, 750,859 B, which took the compile cache and the dump
maps' export): **750,859 -> 717,284 B gzip (-33.6 KB)**; dumps still byte-identical. The
table below was measured before that rebase.

Every commit's dumps are byte-identical to d481d1fe's over the corpus (3,777 dump files of
`difftest --plain-out`, the clock's value normalized), the v2 programs' 196 and the sleep
example's (work/scratch/0298/compare.sh, cmp.py).

| commit | change | module gzip |
| --- | --- | ---: |
| base d481d1fe | | 767,666 |
| DumpOptions::base goes (coordinator, from 0297's survey) | the dumper takes `BaseNames` (no `Option`): no base copy, no walk of it for the builtins' paths (`registry::reachable_names` is dead in the module), no per-VM `BaseTable` cache; tests take `dump_harvest::base_names()`, e2e djs-prelude's `BASE_NAMES` (djs-core's dev-dependency gets `names`) | 761,253 (-6.4 K) |
| `cat!` | the dump's `cat!` is `crate::text::cat!` (a `put` per piece; no `&[&dyn Piece]` array and no `size_hint`) | 759,048 (-2.2 K) |
| `note`/`edge` | one body behind `&mut dyn FnMut() -> String`; the generic wrappers only box the caller's closure (they were inlined into ~60 callers) | 752,562 (-6.5 K) |
| no ordered maps or core sorts | `StrSet`/`StrMap` (dump.rs: strings over the `Index`, insertion order, FNV-1a with a collision list) for every BTreeSet/BTreeMap of the dumper and the writer (the insertion order never reaches the output: the iterated ones only fill hashes or bitmaps); `Key` sets by `Key::text`; `recs` a Vec by line; `lazy_globals` by `Index`; std's `sort_by_key` and every `crate::sort::sort_by` of dump code -> `sort::sort_cold` (one `dyn`-compared merge sort of indexes, the permutation as swaps) | 734,119 (-18.4 K) |

(compile_cache.rs is left as it was: 0297 removes it.)

Dump cost (scripts/dump-costs/instructions.sh, native release, all cases): base dump 16,490
M instructions, load 12,766 M, re-dump 17,984 M; now dump 17,485 M (+6%), load 12,833 M,
re-dump 18,793 M (+4.5%). Of the dump's +995 M, 460 M are `cat!` without sizing its string
first (measured by putting `cat_sized` back); the rest is the hash sets, the dyn sort and
`note` behind a dyn call. Left as is for now: step 3 takes the text scans out, which are
most of a dump's time in the writer; measured again at the end.

`#[optimize(size)]` measured again on this tree (variant.sh, optsize-files.py over
vm/dump*.rs, compiler/smap*.rs, lazy.rs, registry.rs, compile_cache.rs): 661,320 -> 646,031,
-15.3 KB: still pays (step 4).

Guide: none of the pages' prose changed (dumps.md's dumper size is the status number
`dump/extra-gzip`, which the status refresh measures).

### 2026-11-23 pdiet: step 4 early, `#[optimize(size)]` (D3660): 717,284 -> 702,049 B (-15.2 KB)

Landed before the structural step since it is independent of it (a new dump function gets
the attribute line too: work/scratch/0298/optsize.py puts it on every fn of vm/dump*.rs
that lacks it). 588 functions in 18 files; `--cfg djs_optsize` from wasm-build-env.sh,
`check-cfg` in djs-core's Cargo.toml. Dump time in wasm, dump-costs.ts (the cases and the
harvested corpus, 3 reps, alternating, load 20-70): without 450.6/412.8/329.2 total ms, with
401.9/341.1/341.0: no difference beyond the noise. scripts/dump-costs/dump-costs.ts read a
`dumpPath` that 0297 had removed: its report line now names the one module (outside the
touches, one line).

### 2026-11-23 pdiet: after merge 2: where the rest is, calibration of a restyle

Dumper now (cf3e2d76): 701,709 vs 427,879 B gzip without `dump` = 273.8 KB (from 334.6:
-60.8 KB, 0297's compile-cache removal included). Raw with glue (attr.py, minsize build):
dump.rs 106 K, dump_cont.rs 102 K (+canon 15, heads 12, pattern 11, using 7.5), plain 73 K,
cat! sites (text.rs:151-152) 60 K, flight 47 K, plain_inline 31 K, smap_syntax 22 K, origin
16.5 K, sort.rs 16 K (sort_cold instances), smap 14.5 K, reread 8.4 K. Instruction mix of
the dump functions: local.get 25%, i32.const 17%, loads/stores ~20%; 1,470 dealloc call
sites, 1,900 Piece::put calls.

Calibration (coordinator chose A: dense style, byte-identical): converting all 85
`P::L(vec![s(".."), x, ...])` builders of cont::G into one-constant templates
(work/scratch/0298/tplconv.py) gave -539 B; an out-of-line `str` Piece::put +81 B; the cat!
variants +-2 KB. Under minsize the per-site style is cheap: the bytes are the logic. Told
the coordinator; a restyle would give ~5-10 KB. The factory's format-2 text (emit_group's
`text`/`body`/`ret`) is still read by the writer in two places (emit_items' `$djs.resume(`
test, split_callback's keys_in), so dropping it is a small structural change (~2-3 K raw).

Small wins on the branch since merge 2:
- note/edge take `&dyn Fn() -> String` (no wrapper per closure): -411 B.
- compiler/smap_syntax.rs, smap.rs: the parser's recording hooks (syn_open/kind/scope/part/
  close/decl/erased, names_push/pop, site) were `#[inline]` with their bodies, copied into
  ~220 parser call sites; now only the check is inline, the recording `#[inline(never)]`
  (a normal compile does the same check and no more): -2.9 KB (701,298 -> 698,391).
Both byte-identical (corpus, v2, sleep).

Waiting on the owner for the B items (subsystems that could go: canon double-write check,
the free-names checker, single-use inlining, operand re-reading, TS type stripping,
node-loadability); none started.

### 2026-11-23 pdiet: merge 3, the B items measured, the redesign reassessed

Merge 3 (main 0b15dec1): the parser's suspension-map hooks out of line, note/edge `&dyn Fn`:
701,738 -> 698,440 B gzip (-3.3 KB), ceiling 699,464. Dumps byte-identical.

The B items (owner's call), variant builds of 0b15dec1 with stub patches
(work/scratch/0298/stubs/, gzip of code+data, base 625,998): no free-names check of the
written text -4.1 KB (it is also ~35% of a dump's time); no operand re-reading
(dump_reread) -4.2 KB; no TS type stripping -2.2 KB. The canon double write is origin
tracking's layout of holes, not a separable check. Single-use inlining (dump_plain_inline,
~31 K raw) would make every dump bigger.

The structured-references redesign, mapped: 88 call sites scan texts (scan/refs/keys_in/
eager_*/dump_refs/only_keys/split_def/classify/...), most over texts that hold user
function source or composites of several values (slot values, factory members, the
placed texts of continuation blocks), so a structured form would have to carry
references through every text the Dumper and cont::G compose; the factory's format-2 text
is read in two places only (emit_items' `$djs.resume(` test, split_callback's keys_in).
Estimated net ~5-10 KB at high risk; recommended to the coordinator not to do it.

### 2026-11-23 pdiet: 0297's leftovers (coordinator)

Doc comments of dump.rs, dump_cont.rs, dump_origin.rs, dump_plain.rs, dump_harvest.rs and
dump_runtime.rs's module header (js/src/dump-runtime.js regenerated) point at
docs/guide/dumps.md and continuations.md instead of the deleted docs/plain-js-dumps.md and
functions-as-code.md; `tests/dump/v2` -> `tests/dump` in dump_resume_tests.rs;
tests/dump/plain-refusals.txt -> refusals.txt, plain-chain-refusals.txt ->
chain-refusals.txt; DJS_PLAIN_OUT -> DJS_DUMP_OUT; the dump tests and their helpers lose
`plain_` (dumps_at_every_idle_point, dump_chains_continue_like_the_run, dumps_of_*,
run_dump, dump_text, dump_text_at, load_dump, frozen_vm, idle_step, ...); gate.py,
TESTING.md, scripts/dump-size/dump-size.ts and tests/scores/dump-size.tsv follow. The
corpus's plain-fuzz directories and the difftest's own `--plain-*` flags and
tests/plain-refusals.txt stay.

## Follow-ups

## Result
