# 0043: wasm size audit and reduction

status: in-progress
agent: heap
branch: issue/0043-wasm-size
depends-on: 0039 (part 1)
touches: crates/djs-prelude/, crates/djs-core/src/compiler/ (privileged-code size options only), crates/djs-core/src/heap/ (image encoding only), crates/djs-wasm/, crates/djs-bench/ + tests/bench/size, scripts/

## Goal
ARCH G2 budget: < 400 KB release wasm after wasm-opt (quickjs.wasm is ~850 KB). Measured
by the coordinator on main (2026-11-01, after 0039 part 1), `scripts/build-wasm.sh`
(profile wasm, opt-level s):

| section | bytes |
| --- | ---: |
| code | 590,853 |
| data (mostly the embedded base image, 426,216 bytes) | 479,623 |
| name (custom, stripped for shipping) | 148,592 |
| total | 1,222,167 (gzip 356,032; base.img alone gzips to 102,155) |

The base image grew from 72 KB (0032) to 426 KB as the prelude grew (Promise, Map/Set,
Date, typed arrays, …), and the code is about 590 KB.

## Acceptance criteria
- [ ] Attribution: a script (no new tool installs; parse the name section, or use
      `cargo bloat` only if it is already available) that lists the code size per
      module/function and the base image size per cell kind (bytecode, constant pools,
      line tables, property tables, strings, atoms). Report the top items.
- [ ] Base image reductions, measured one by one. For example: no line tables or
      handler-table padding for privileged code where not needed (prelude frames are
      hidden from stack traces), compact constant pools, deduplicated strings, a
      compact image encoding for shipping (for example zero-run or LEB encoding of the
      arena, decoded at `vm_new` into the arena; measure the `vm_new` cost, since it
      must stay far below 100 µs), and checking free/padding space in the image.
- [ ] Code reductions: generic code that monomorphizes many times (u8/u16 string paths,
      macros), panic/format machinery in no_std (core::fmt pulls a lot), large match
      tables, and whether `opt-level=s` vs `z` per crate (the hot interpreter at s, the
      cold compiler/regexp parsers at z, via `[profile.wasm.package.*]`) gives a better
      trade-off (D1232).
- [ ] wasm-opt: not installed. Record the numbers without it, and ask the coordinator
      whether to install binaryen (the user decides).
- [ ] The size track in djs-bench records the sections separately (code, data, names).
- [ ] No behavior change: gate green, ratchets unchanged, and `vm_new` time recorded.

## Log
- Baseline (branch start, main fb6327d; `work/scratch/0043/measure.sh`): file 1,280,740;
  code 612,593; data 512,335 (base image 457,952); name 152,656; stripped 1,127,850;
  stripped gzip -9 343,719. `vm_new` (node, js/bench/sandbox.bench.ts) 5.3 µs,
  instantiate 49 µs.
- Attribution tools: `scripts/wasm-size.py` (sections, code per crate / path / function
  from the name section, demangled with rustfilt when present) and
  `cargo run -p djs-prelude --example image-size [-- --detail]` (base image per cell kind
  and role).
- Code: djs_core 469 KB (interp::run 142 KB: `Exec::run` 45 KB, `call_intrinsic` 36 KB;
  compiler::parser 82 KB; Heap 59 KB; json::parse 25 KB; resolve::finish 15 KB), core
  45 KB (slice::sort 20 KB: monomorphized sorts), alloc 42 KB (btree 18 KB), libm 19 KB,
  std 13 KB, dlmalloc 8 KB.
- Base image 457,952 bytes: PropTable 121 KB (889 tables, 4168 entry slots for 2268
  entries: 46 KB of slack), bytecode 71 KB, Str8 62 KB (1888 strings), FuncTemplate 43 KB
  (599 x 72), line tables 38 KB, ValueArray 27 KB (value stack 8 KB, atom entries 16 KB
  for 1251 atoms), Function objects 25 KB, U32Array 22 KB (atom index 16 KB), local names
  17 KB, constant pools 11 KB, capture descriptors 8 KB. 60.6% of the bytes are zero.
- R1 (D1430) privileged code without line tables and local names: base 457,952 ->
  403,056 (-54,896); stripped 1,127,850 -> 1,072,960; gzip 343,719 -> 320,112.
- R2 (D1431) `Heap::compact_for_image` in djs-prelude's build: base 403,056 -> 296,200
  (tables + atom entries -51,976; the value stack and the dead templates its stale
  slots kept alive -54,880). stripped 966,096; gzip 305,089. vm_new 3.6 µs, instantiate
  41.5 µs (node).
- Rejected: a compact image encoding (decoded at vm_new). On the 351 KB image, zero-run
  coding gives 73% (42K runs), a byte mask 54%; both decode byte- or run-wise, about
  50-300 µs per VM in wasm, against a 3.6 µs memcpy today. Transport gzip already gets
  the image to ~84 KB.
- Rejected: opt-level "z" for everything: code -88 KB, but 15-60% slower (loop 330 ->
  380 ms, calls 147 -> 213, props 463 -> 699, arrays 2292 -> 3747; perf.ts in scratch).
  Per-package opt-levels do nothing under LTO (libm at z: -300 bytes).
- R3 (D1432) no BTree collections (-16.4 KB code) and one small stable sort instead of
  core's (-14.4 KB): code 612,581 -> 581,874. Edits outside `touches:` (small): compiler
  func.rs/parser.rs/scan.rs, json/stringify.rs, interp/run.rs (for-in), heap/object.rs
  (own keys sort), builtins/array.rs (sort_primitives), new src/sort.rs.
- R4 (D1433) prelude closures without length/name/prototype (func_flags::PRIVILEGED,
  coordinated with the lexer via the coordinator): base 296,200 -> 268,480; stripped
  905,901. Builtins' own keys are now `length, name, prototype, ...` (qjs: prototype
  after the methods).
- R5 (D1434) string constants shared per compilation unit: base 268,480 -> 260,472.
- Size track: `size/<module> code|data|names` recorded separately (djs-bench size.rs,
  `section_sizes`). djs-wasm's `STATE` thread-local is `const` (-400 bytes code only:
  the `with` instantiations are mostly the export bodies).
- Code: no big remaining generic sink; the largest items are inherent (`Exec::run` 45 KB,
  `call_intrinsic` 36 KB, the parser 82 KB). core::fmt is ~6 KB in total, panics ~2 KB.
- Rebased onto main after 0041 (async/generators compiler) and 0039 (typed arrays),
  same build, main vs branch: stripped 1,200,328 -> 948,113 (-21%); code 640,526 ->
  603,729; data 556,783 -> 341,467; base image 501,760 -> 288,024; names 158,577 ->
  139,189; stripped gzip -9 362,806 -> 301,160. vm_new (node) 6.2 -> 3.7 µs,
  instantiate ~66 -> ~55 µs. Engine scripts in the Sandbox unchanged or slightly faster
  (loop 300 -> 295 ms, calls 160 -> 143, props 499 -> 457, arrays ~2330, json ~25).
  0039's typed array sort had brought core's sort back (-6 KB code by moving it to
  crate::sort too).
- wasm-opt: not installed (the user has not decided); numbers above are without it.

## Follow-ups
- `Vm::collect` (vm_collect) does not clear the value stack above `sp` the way the
  interpreter's safepoint does, so an embedder-triggered collection keeps popped values
  alive (found through the base image, where it kept 74 dead templates). vm/ is outside
  0043's touches.

## Result
