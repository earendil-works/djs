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

## Follow-ups

## Result
