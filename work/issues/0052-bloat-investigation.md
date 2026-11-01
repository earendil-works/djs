# 0052: Code bloat investigation (report only)

status: done
agent: bloat
branch: issue/0052-bloat-investigation
depends-on: -
touches: work/issues/0052-bloat-investigation.md (the report), work/scratch/0052/ (measurements), scripts/ (new analysis scripts only, no changes to existing ones). No changes to code or Cargo.toml.

## Goal
The owner wants to understand where djs's size comes from: "this comes out quite beefy
already". Investigate the wasm binary (and the native one, if useful) and write a clear
report: what takes the space, why, and what could realistically be done about it, with
numbers. This is an investigation: no code changes. Proposals go to the coordinator.

## Context
- ARCH G2 budget: < 400 KB release wasm after wasm-opt (quickjs.wasm is ~850 KB).
  wasm-opt/binaryen is not installed (the owner decides whether to install it).
- 0043 (heap agent) did a first pass: stripped module 1,200 → 948 KB, base image 502 →
  288 KB. Its tools are `scripts/wasm-size.py` (code per crate/module/function from the
  name section) and `cargo run -p djs-prelude --example image-size` (base image per cell
  kind). Read work/issues/0043-wasm-size.md for its attribution and the rejected ideas
  (compact image encoding: decoding is too slow; opt-level z: 15–60% slower; per-crate
  opt-levels: ignored under LTO).
- Since then BigInt (+35 KB code), RegExp, Proxy, classes, typed arrays and more
  landed. Build with `scripts/build-wasm.sh` (profile wasm, opt-level s).

## What to deliver
- [x] The current numbers on main: code / data / names / total, stripped and gzipped,
      plus the base image per cell kind, and the top contributors by crate, module and
      function. How the size developed over the project history (sample a few commits
      along `git log` and build them in your own worktree: the 0019 bench `size` track
      in tests/scores/bench.tsv has some history too).
- [x] Root causes, not just sizes. For example: monomorphization (generic code
      instantiated per string width or per type), inlining at opt-level s (the dispatch
      loop and `call_intrinsic`), big match statements, Unicode/case/property tables,
      libm, alloc/core generics, panic/format machinery, debug-only code that leaks into
      release, duplicated logic between the Rust leaves and the builtin-language
      prelude, the base image's content (bytecode vs constants vs property tables vs
      atoms vs strings), and anything else you find.
- [x] A comparison point: quickjs-wasi's wasm in pi-private's node_modules (read-only),
      the same metrics for it, and where djs is bigger or smaller per area (parser,
      interpreter, builtins, regex, number formatting, unicode).
- [x] Ranked opportunities: for each, the expected saving, the cost (performance,
      effort, risk), and whether it needs wasm-opt numbers first. Separate quick wins
      from structural changes.

## Notes
- Your own worktree (`.worktrees/0052-bloat-investigation`) for builds, so you don't
  disturb other agents. Builds are expensive on this shared machine: reuse your target
  dir and build historical commits sparingly.
- Use `scripts/timeout.sh` for long runs.

## Log
- Measured main 379682a (0048 TypeScript merged) with `scripts/build-wasm.sh`; all
  numbers below are that build unless noted. Scratch: `work/scratch/0052/` (modules,
  disassembly, line tables, experiment patches in `patches/`, perf results).
- New scripts (analysis only, nothing existing changed):
  - `scripts/wasm-funcs.py`: per-function sizes; `--areas` (code per functional area),
    `--generic` (instantiation counts).
  - `scripts/wasm-dwarf-lines.py`: code bytes per *source file/line* from the DWARF line
    table, so inlined code is charged to where it is written (e.g. `core/result.rs` for
    `?`). Needs a build with line tables: `CARGO_PROFILE_WASM_DEBUG=1` in a separate
    target dir; the code section is byte-identical to the normal build (checked).
  - `scripts/wasm-leb-padding.py`: bytes of padded 5-byte LEB immediates.
  - `scripts/wasm-size-history.sh`: builds djs-wasm at a list of commits (`git archive`,
    shared target dir; `tar -m` matters, see the script).
  - `scripts/wasm-areas-unnamed.py`: rough per-area attribution of a module without a
    name section (quickjs-wasi), from referenced C strings plus call-graph propagation.
- Data attribution: link map (`RUSTFLAGS=-C link-arg=--Map=…`) of the same build.
- Experiments (not committed, in a scratch copy `work/scratch/0052/exp-src`): four small
  code edits A–D and the `--compress-relocations` link flag, each measured alone and
  combined; perf on the wasm build with `work/scratch/0052/perf.ts` (compile of
  typescript.js 9 MB + the tests/bench scripts + two throw loops), min of 4–8 interleaved
  runs. The machine was under load 9–24 the whole time (other agents), so perf numbers
  are only good to about ±5–10%.
- wasm-opt is not installed, but wasm-pack left binaryen 117 in
  `~/Library/Caches/.wasm-pack/wasm-opt-50385c9e73ccee70/bin/wasm-opt`. Used only to
  measure, nothing installed.

## Findings

### 1. Current numbers (main 379682a)

| | djs | quickjs-wasi 3.6.2 |
| --- | ---: | ---: |
| file as built | 1,359,191 | 637,405 |
| code section | 779,951 | 509,052 |
| data section | 400,955 (base image 338,456 + Rust rodata 62,499) | 115,796 |
| name section (custom) | 174,754 | none |
| stripped | 1,184,203 | 637,405 |
| stripped, gzip -9 | 376,645 | 286,655 |
| stripped, brotli 11 | 301,299 | 247,099 |
| functions | 1,641 | 1,698 |

Note that the file `scripts/build-wasm.sh` leaves behind still has its 175 KB name
section; `--strip` removes it. The shipping comparison is "stripped": 1.86x quickjs-wasi
raw, 1.31x gzipped. Per section gzip: djs code 261,819 / data 112,359; quickjs code
208,930 / data 70,471.

### 2. History

Built with `scripts/wasm-size-history.sh` (profile wasm, same toolchain):

| commit | after | code | data | stripped | gzip |
| --- | --- | ---: | ---: | ---: | ---: |
| f368c48 | 0023 djs-wasm | 340,768 | 37,271 | 380,159 | 139,068 |
| 9d2f578 | 0026 leaf perf | 409,829 | 49,379 | 461,494 | 168,319 |
| fa25039 | 0032 base snapshot | 390,739 | 108,623 | 501,613 | 172,674 |
| 7ebc262 | 0031 builtins 1 | 422,019 | 216,695 | 641,077 | 208,940 |
| 3c2e9b6 | 0034 builtins 2 | 489,490 | 268,319 | 760,242 | 247,151 |
| 5cab0f5 | 0038 builtins 3 | 526,807 | 383,727 | 913,082 | 288,500 |
| fb6327d | 0042 async | 612,593 | 512,335 | 1,127,850 | 343,734 |
| 23be57f | 0039 typed arrays | 624,263 | 556,399 | 1,183,642 | 357,477 |
| c6c8794 | 0043 size pass | 603,729 | 341,467 | 948,113 | 301,174 |
| 8ee33a6 | 0045 Proxy | 614,735 | 362,171 | 979,885 | 311,320 |
| 3d6a34d | 0040 classes | 681,938 | 370,483 | 1,055,532 | 335,443 |
| 059e54b | 0033 RegExp | 743,793 | 399,899 | 1,146,940 | 366,794 |
| eda6118 | 0046 BigInt | 748,782 | 400,795 | 1,152,834 | 368,571 |
| 379682a | 0048 TypeScript | 779,951 | 400,955 | 1,184,203 | 376,660 |

The data grew with every builtin wave (the base image), until 0043 cut it by 40%; since
then the growth is code: +176 KB since 0043, of which RegExp +60 KB, compiler +35 KB
(classes, TS hooks), TypeScript +24 KB, BigInt +19 KB, interpreter +27 KB (classes,
Proxy, generators), heap +6 KB.

### 3. Code by area (`scripts/wasm-funcs.py --areas`)

| area | bytes | % |
| --- | ---: | ---: |
| compiler (parser + codegen + resolve) | 177,100 | 22.7 |
| interpreter (without the two below) | 95,815 | 12.3 |
| heap / objects / GC | 85,758 | 11.0 |
| regexp (parser, compiler, exec, builtins) | 62,269 | 8.0 |
| `Exec::run` (dispatch loop) | 56,109 | 7.2 |
| `call_intrinsic` + `fast_intrinsic` | 40,168 | 5.2 |
| JSON | 36,039 | 4.6 |
| Rust builtin leaves (array, string, typed, …) | 30,769 | 3.9 |
| VM API + djs-wasm exports | 27,178 | 3.5 |
| number parse/format | 24,990 | 3.2 |
| TypeScript | 23,628 | 3.0 |
| libm | 20,321 | 2.6 |
| BigInt | 19,328 | 2.5 |
| lexer | 18,246 | 2.3 |
| alloc/core generics (not inlined) | 16,864 | 2.2 |
| core::fmt / panic | 12,716 | 1.6 |
| Date | 10,406 | 1.3 |
| dlmalloc | 9,272 | 1.2 |
| wire / snapshot | 6,870 | 0.9 |
| other | ~6,000 | 0.8 |

Largest functions: `Exec::run` 56 KB, `call_intrinsic` 38 KB, `json_regexp_intrinsic`
13.5 KB, `resolve::emit_function` 12.9 KB, `parse_primary` 9.6 KB, `parse_class_element`
8.3 KB, `parse_for` 7.9 KB, `json::parse` 7.4 KB, `parse_pattern` 7.0 KB,
`ts_skip_type2` 6.6 KB.

### 4. Root causes (code)

Attributing the code by *source line* (DWARF, inlined code charged to where it is
written) shows that a large part of the code is Rust plumbing inlined at many sites, not
JS semantics:

| source of the bytes | bytes | % of code |
| --- | ---: | ---: |
| `core/result.rs` (`?`: branch / from_residual, moving `Result`s) | 70,720 | 9.1 |
| `alloc/vec` + `alloc/raw_vec` (push, grow, drop inlined) | 94,658 | 12.1 |
| `djs-core/heap/layout.rs` (bounds-checked field accessors) | 47,369 | 6.1 |
| `core/ptr` + `core/option` + `core/slice/index` + `uint_macros` | 64,611 | 8.3 |
| `djs-core/value.rs` (NaN-box encode/decode) | 23,586 | 3.0 |

Inside the compiler (201 KB including TS) 17% is `result.rs`, 21% `vec`+`raw_vec`;
inside `Exec::run` 28% is `layout.rs` and 10% `result.rs`.

The causes, with what fixing each one measured (§ Opportunities):

1. **Padded relocations (toolchain): 52 KB, 6.7% of code.** wasm-ld writes every call
   target, data address and `__stack_pointer` access as a 5-byte LEB so it can patch it;
   nothing compacts them afterwards (no wasm-opt). 17,094 sites: 11,149 calls (36 KB),
   2,991 global.get/set (12 KB), 2,849 i32.const (4 KB). quickjs-wasi has none.
   `-C link-arg=--compress-relocations` removes them: code -53.7 KB.
2. **Error types that do not fit in a register.** The compiler's `PErr` (16 bytes:
   `pos` + `Cow<'static, str>`) makes every `PResult<T>` a stack-returned struct, so
   each of the ~800 `?` in the compiler copies the error out and branches on a tag in
   memory. Boxing it (`PResult<()>` becomes one i32) is -27.5 KB. The interpreter's
   `Fault` (`Throw(Value) | Error(ErrorKind, String) | Heap`) is the same pattern on
   ~500 `?`; boxing it is -9.8 KB (`Result<Value, Fault>` stays 16 bytes, so less).
3. **The bytecode emitter inlines `Vec::push` into every emit site.** The generated
   per-opcode emitters are `#[inline]` and call `begin` (`code.push(op)`) and
   `push_bytes` (`extend_from_slice`), also `#[inline]`: 554 inlined
   `RawVec<u8>::grow_one` call sites plus 312 `spec_extend`. Making `begin`,
   `push_bytes` and `push_target` `#[inline(never)]` is -34.6 KB.
4. **Bounds-checked heap access (by design, "no unsafe").** Every field read/write is a
   check plus a call to `out_of_bounds(off, n, len)`: 1,345 call sites, each also
   materializing three arguments. Passing no arguments (keeping the check) is
   -16.4 KB. The checks themselves (~10 bytes each) stay; ~600 more are Rust slice checks
   (414 `panic_bounds_check`, 177 `slice_index_fail`).
5. **Panic and format machinery: ~41 KB.** Measured with nightly
   `-Zbuild-std -Cpanic=immediate-abort`: code -24 KB, data -17 KB (panic locations,
   messages). Not shippable as is (nightly, loses panic messages); it bounds what
   trimming panics could give. 196 `format!` sites (error messages), 32 `Debug` impls
   (4 KB, mostly `{:?}` in error paths), 104 `expect`, 48 `unwrap`.
6. **Monomorphization is not a big factor.** True duplicates (same code per string
   width or input type) are ~10–15 KB: `json::parse::Parser::next`/`key_colon` x3
   (6.5 KB extra), `scan_decimal`, `string_to_number`, `parse_digits`, `wire::transcode`
   x2. The other "instantiations" (`LocalKey::with` x37 = the export bodies, 50
   `collect()` sites) are distinct code.
7. **Features (inherent).** RegExp 62 KB, TypeScript 24 KB, BigInt 19 KB, Date 10 KB,
   JSON 36 KB (deliberately fast paths, x3 for Latin-1/UTF-16/UTF-8 input). libm 20 KB
   (quickjs 12 KB; `fmod` is in twice, libm's and compiler_builtins' via `%`: 1.4 KB).
   dlmalloc 9 KB; std proper is only 2 KB.
8. **Debug-only code leaking:** none of note. `Heap::verify` (6.5 KB with closures) is
   reachable from `vm_from_bytes` (untrusted images), the disassembler is not linked,
   `Vm::new` + the prelude source are not linked (`compiled-prelude` feature off).
   Build-time-only data that is linked: the intrinsic table with names (320 entries,
   3.8 KB + ~3.5 KB names) for privileged compilation.

### 5. Data: the base image (338 KB, 84% of the data)

`cargo run -p djs-prelude --example image-size -- --detail`:

| cell kind | bytes | cells |
| --- | ---: | ---: |
| Bytes: bytecode | 77,432 | 690 |
| Str8 | 63,064 | 1,882 |
| PropTable | 50,712 | 509 |
| FuncTemplate | 49,680 | 690 (72 B each) |
| Object: functions | 30,520 | 690 |
| U32Array (atom index 16 KB) | 22,448 | 19 |
| ValueArray (atom entries 12.5 KB, value stack 2 KB) | 17,224 | 5 |
| capture descriptors | 11,000 | 519 |
| constant pools | 10,056 | 294 |
| other | 6,320 | |

Root cause: the image is the in-memory heap layout of ~690 prelude functions (765 in
prelude.js, 248 KB of JS) and their objects, 8-byte aligned, 8-byte Values, so 56% of
its bytes are zero. Per builtin function that is ~490 bytes: template 72, function
object 44, a 2-entry PropTable for `length`/`name` 72 (424 of them), bytecode ~112,
plus strings and descriptors. Strings are header-heavy: 1,882 Str8 cells carry 30 KB of
headers (8 cell + LEN + HASH) for 26 KB of characters. PropEntry is 24 bytes (key,
flags, value, setter), the setter slot is unused for data properties (12.8 KB here, and
in every heap). The atom hash index is at 38% load (4,096 buckets, 1,561 atoms).

It compresses well, which is why gzip hides it: gzip 85 KB, LZ4-HC 104 KB, zstd -19
72 KB, brotli 67 KB, a zero-byte mask 191 KB.

The rest of the data (62 KB): Unicode ID/case tables 12.5 KB, the intrinsic table +
names ~7 KB, the opcode table ~5 KB with names, ~17 KB panic locations/messages, the rest
error strings and switch tables.

### 6. Comparison with quickjs-wasi (QuickJS-NG 0.17, wasm32-wasi)

quickjs-wasi has no name section, so its areas are estimated by
`scripts/wasm-areas-unnamed.py` (strings the functions reference + call graph; the
"seeded" bytes are labeled directly, the rest is inferred), cross-checked with the
native dquickts `libquickjs.a` (compiler ≈ 17% of quickjs.c's text). Treat ±30%.

| area | djs code | quickjs-wasi code (est.) | note |
| --- | ---: | ---: | --- |
| compiler + lexer | 195 KB | 60–80 KB | djs compiler 123 KB after Q2+Q3 |
| TypeScript | 24 KB | – | qjs has none |
| interpreter loop | 56 KB | 37 KB (`JS_CallInternal`) | |
| intrinsic dispatch | 40 KB | – | djs-only layer (320 intrinsics) |
| runtime, objects, GC, other builtins | 212 KB | ~150 KB | qjs builtins are C; djs's are prelude bytecode (data) |
| regexp | 62 KB | 35–56 KB | qjs also has `\p{}` tables (data) |
| JSON | 36 KB | 8–15 KB | djs: three input widths, fast paths |
| number parse/format | 25 KB | 8–9 KB (dtoa.c) | |
| BigInt | 19 KB | 11 KB | |
| Date | 10 KB | 7–8 KB | |
| libm | 20 KB | 12 KB | |
| libc / malloc / printf | 9 KB malloc + 13 KB fmt/panic | 25–40 KB | |
| modules, debug dumps, snapshot | – | ~25 KB | qjs-only |
| unicode | 1.5 KB code + 12.5 KB data | ~7 KB code + ~40 KB data | djs has no `\p{}`/normalization tables yet |
| builtins as data | 338 KB base image | ~30 KB function lists/atoms | |
| relocation padding | 52 KB | 0 | |

Where djs is bigger: (1) the base image (+~300 KB raw, ~+15 KB gzip: qjs builds its
realm at runtime from compact C tables, djs ships the realm materialized for a 3.6 µs
`vm_new`); (2) the compiler (~2.5x, most of it the Rust `?`/`Vec` plumbing above, not
a different design); (3) the toolchain padding (52 KB); (4) the intrinsic layer plus
JS wrappers: a builtin costs ~490 B of image plus ~120 B of `call_intrinsic`, a qjs
builtin ~200 B of C plus a ~20 B table entry; (5) number formatting and JSON (speed
choices). Where djs is smaller or equal: Unicode tables (qjs has more features there),
libc, no modules/debug dumps.

### 7. wasm-opt (binaryen 117, measurement only)

| | code | stripped | gzip |
| --- | ---: | ---: | ---: |
| main | 779,951 | 1,184,203 | 376,645 |
| wasm-opt -Os | 681,434 | 1,067,855 | 385,182 |
| wasm-opt -Os, skip memory-packing | 681,434 | 1,084,863 | 373,940 |
| fixes A–D + compress-relocations | 641,467 | 1,045,695 | 359,058 |
| that + wasm-opt -Os, skip memory-packing | 605,164 | 1,008,591 | 361,470 |

wasm-opt's memory-packing splits the image into 3,276 segments and makes gzip worse
(+11 KB). Most of what wasm-opt saves on code is the LEB padding, which the linker flag
also removes; on top of the fixes it gives -36 KB raw, +2 KB gzip.

## Opportunities

Savings are measured on stripped modules (code section), unless marked "est.". Perf:
`work/scratch/0052/perf.ts`, min of interleaved runs on a loaded machine (±5–10%).

### Quick wins (small, local; all measured)

| # | change | code | gzip | perf | risk / cost |
| --- | --- | ---: | ---: | --- | --- |
| Q1 | `-C link-arg=--compress-relocations` for the wasm target (build-wasm.sh or target rustflags) | -53.7 KB | -5.8 KB | none (same code) | very low; flag is incompatible with debug info, the wasm profile has none |
| Q2 | emitter helpers `begin`/`push_bytes`/`push_target` `#[inline(never)]` (A) | -34.6 KB | -5.7 KB | compile within noise (0–8% slower on 9 MB of TS source) | 3 attributes |
| Q3 | box the compiler error: `type PErr = Box<PErrKind>` (B) | -27.5 KB | -4.2 KB | none measurable | ~15 lines in compiler/{mod,parser,resolve}.rs |
| Q4 | `out_of_bounds()` without arguments (D) | -16.4 KB | -2.5 KB | none measurable | panic message loses offsets (keep them under `debug_assertions`) |
| Q5 | box the interpreter `Fault` (C) | -9.8 KB | ~0 | none measurable on throw loops | one allocation per thrown error; ~25 lines |
| Q6 | strip the name section in the shipped build (`--strip` by default or `strip` in the profile) | file -175 KB | – | – | lose names in devtools profiles |

Q1–Q5 together (patches in `work/scratch/0052/patches/`, 50 lines): code 779,951 →
641,467 (-17.8%), stripped 1,184,203 → 1,045,695, gzip 376,645 → 359,058. No
measurable perf change (compile, throws, loop, calls, props, higher-order, strings,
regexp). Not run through the gate; the experiments only had to build and run the bench
scripts.

Smaller ones (est.): `Debug` → static strings in error paths (~4 KB), libm `arch`
feature so `sqrt/floor/ceil/trunc` become instructions and one `fmod` (~2 KB), the
intrinsic name table only for privileged compilation (~7 KB data), JSON/num parsers
generic over fewer input types (~10 KB, costs JSON speed: not recommended).

### Structural (bigger, need decisions)

| # | change | saving | cost / risk |
| --- | --- | --- | --- |
| S1 | Ship the base image LZ4-compressed, decode **once per module instance** into a static buffer; `vm_new` stays a memcpy (0043 rejected decoding per `vm_new`, this avoids that) | -234 KB raw (338 → 104 KB), gzip neutral (85.3 → 84.6 KB) | one-time decode est. 0.2–0.5 ms in wasm (lz4 native: 3.9 GB/s → 86 µs) on the first `vm_new` of an instance; +104 KB linear memory; ~60-line safe decoder in djs-prelude, encoder in its build.rs. Owner decision: is an instance reused across VMs in pi? |
| S2 | Image/heap layout: PropEntry 24 → 16 B (accessor pairs elsewhere), `length`/`name` of builtins shared or lazy (424 tables), a slimmer template for privileged functions, atom index at ~75% load, smaller Str8 header | est. 70–90 KB image (≈20–25%), also less memory per VM | touches heap layout, GC, snapshot format; high effort, needs its own issue |
| S3 | Fewer bounds checks in the interpreter: one checked cell view per op instead of per field | est. 15–25 KB | large refactor of heap accessors; likely faster, not slower |
| S4 | Builtins: the bgen Rust backend (`@native`) trades image bytes for code bytes; worth it only for hot builtins, not for size | – | ARCH plan already |
| S5 | wasm-opt -Os (skip memory-packing) after Q1–Q5 | -36 KB raw, +2 KB gzip | needs binaryen installed (owner decision); the linker flag gets most of it already |

### Not recommended
- opt-level "z" (0043: -88 KB but 15–60% slower).
- `panic=immediate-abort` (nightly only, loses panic messages; -41 KB).
- wasm-opt with memory-packing (worse gzip).

### Upcoming growth to plan for
Unicode `\p{}` property tables and normalization (qjs carries ~40 KB of Unicode data),
the remaining builtins (0051), each builtin wave adds ~0.5 KB of image per function.

## Result
Report only (no code changes). The current module is 1,184 KB stripped (code 780 KB,
data 401 KB of which the base image is 338 KB), 1.86x quickjs-wasi raw and 1.31x
gzipped. The code excess is mostly Rust plumbing, not design: 52 KB of padded
relocations, ~70 KB of `?` handling with large error types, ~95 KB of inlined `Vec`
code (the emitter), ~47 KB of bounds-check calls. Five small changes (Q1–Q5, 50 lines +
one link flag) measured -138.5 KB of code (-17.8%) with no measurable perf change. The
data excess is the base image (a materialized realm, 56% zeros); compressing it with
decode-once-per-instance would take -234 KB raw at a one-time ~0.3 ms cost. New tools:
`scripts/wasm-funcs.py`, `scripts/wasm-dwarf-lines.py`, `scripts/wasm-leb-padding.py`,
`scripts/wasm-size-history.sh`, `scripts/wasm-areas-unnamed.py`.
