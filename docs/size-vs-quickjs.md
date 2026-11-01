# Why djs is bigger than QuickJS

Issue 0123, measured at main 4b30eea2 (2026-11-07). The owner asked for the fundamental
reasons that the shipped djs module is ~1.85x quickjs-wasi's gzip size, not for a list of
cuts (0122 makes the cuts). All sizes are bytes of the shipped module (stripped);
"gz" is gzip -9.

| | raw | gz | code (gz) | data (gz) |
| --- | ---: | ---: | ---: | ---: |
| djs | 1,480,534 | 529,889 | 910,248 (344,207) | 565,945 (182,597) |
| quickjs-wasi 3.6.2 (QuickJS-NG) | 637,238 | 286,630 | 509,052 (208,930) | 115,796 (70,471) |
| QuickJS = dquickts, our build (below) | 809,813 | 299,380 | 709,894 (237,105) | 96,347 (59,627) |
| djs / quickjs-wasi | 2.32x | 1.85x | 1.79x (1.65x) | 4.89x (2.59x) |

## The answer in short

The area-by-area comparison is against QuickJS as dquickts (the engine djs follows, built
here with names; 299 KB gz). The gap to it is 231 KB gz; quickjs-wasi is another 13 KB
smaller (10.6 KB of that is TypeScript, which it doesn't have).

1. **Rust code is ~1.7x the gzip size of C for the same features: +90 KB gz (39%).**
   Where both engines do the same job (compiler, lexer, runtime core, regexp, BigInt,
   number formatting, standard library), djs is 1.4–2.1x QuickJS's gzip, ~1.7x overall.
   A controlled experiment (six kernels in idiomatic C and idiomatic safe Rust) gives
   1.64x raw. It is a sum of small taxes, measured: inlined `Vec` growth, `Result`
   plumbing, bounds checks (11 KB gz), panic and format paths (17 KB gz).
   Monomorphization and the optimization level are not among them.
2. **The builtins live in a shipped heap image as bytecode: +81 KB gz net (35%).** The
   image is 105 KB gz. QuickJS ships each builtin once, as a C function plus a 16-byte
   table entry, and builds its realm at startup; djs ships the realm already built (for
   a 3.6 µs `vm_new`): 874 functions as bytecode, templates and objects, 2,360 strings,
   property tables and an atom index. It still needs Rust leaves behind them (31 KB gz,
   20 KB less than QuickJS's builtin C). Two thirds of the image's strings are internal
   names a C engine doesn't ship, and djs bytecode is 1.4–2x QuickJS's for the same
   source.
3. **Speed and embedding features QuickJS doesn't have: +48 KB gz (21%).** Generated
   Rust builtins and fast paths that duplicate image builtins (21 KB), JSON paths for
   three input widths (+9), suspension, heap images and the wire format (+6), the host
   ABI (+9), more TypeScript code (+3; +13 against quickjs-wasi).
4. **Unicode tables in a looser encoding: +15 KB gz (6%).** 107 KB raw vs QuickJS's
   43 KB, which packs its tables with a custom run-length scheme.
5. Other data +6.5; QuickJS-only code (modules, more Date code) −9.

What is **not** a cause: the garbage collectors (two collectors, 5.2 KB gz, the same as
QuickJS's reference counting cycle collector), the optimization level (both `-Os`/`s`),
the LLVM versions (rustc 1.99 with LLVM 23, clang 22), libm, and monomorphization
(≤ 5 KB gz of duplicated instances).

About half of the gap is fundamental to djs's goals as designed (a shipped realm image,
safe Rust, speed and embedding features), half is incidental (encodings, internal names,
Rust idioms, duplicated fast paths). ~1.5x is reachable without changing the design;
~1.0x needs the image gone and Rust written at C density, i.e. a different engine
(section 6).

## 1. Method

- **djs:** the shipped module (`scripts/build-wasm.sh`) for sizes; the same build with a
  name section for names (`--keep-names` keeps the same functions in the same order).
  `scripts/size/areas.py djs` sorts functions into feature areas by name. A build with
  DWARF line tables (`CARGO_PROFILE_WASM_DEBUG=1`) attributes bytes to the source line
  they came from, inlined code included (`scripts/wasm-dwarf-lines.py`). The data by a
  link map (`-C link-arg=--Map`), the base image by cell kind (perf's
  `scripts/size/image-kinds.py`).
- **QuickJS:** quickjs-wasi has no names, so the area comparison uses a named build of
  the same engine family: dquickts, the oracle djs follows (Bellard's QuickJS 2026-06-04
  plus TypeScript stripping and suspension), its sources copied (never built in place)
  and compiled with zig's clang for wasm32-wasi at `-Os`, the core only (no
  Temporal/Intl, no quickjs-libc), with a minimal entry point (`scripts/size/qjs/`).
  `-g` gives names and line tables without changing the code. Its gzip (299 KB) is
  within 4% of quickjs-wasi's (287 KB); it carries TypeScript (10.6 KB gz) and padded
  relocations (zig's linker can't compress them; gzip absorbs most of that).
  `scripts/size/areas.py c` uses the same area names, recognizing library files by the
  DWARF source file.
- **gz per area:** the area's function bodies compressed on their own, scaled so the
  areas add up to the whole code section's gzip. Good to a few percent per area.
- **Experiments** are variant builds in scratch copies (`scripts/size/variant.sh`), never
  merged.

## 2. Area by area

Code, raw and gz:

| area | djs raw | djs gz | QuickJS raw | QuickJS gz | gz ratio |
| --- | ---: | ---: | ---: | ---: | ---: |
| compiler (parser, scopes, codegen) | 157,068 | 64,267 | 107,761 | 37,085 | 1.73 |
| lexer | 14,124 | 6,076 | 7,670 | 3,265 | 1.86 |
| TypeScript | 34,510 | 13,451 | 29,874 | 10,558 | 1.27 |
| interpreter dispatch loop (djs: `step_once` + `run_hot`) | 52,449 | 15,352 | 54,427 | 13,646 | 1.12 |
| runtime core (calls, properties, objects, shapes, strings, atoms, operators, errors) | 187,751 | 72,811 | 116,205 | 41,152 | 1.77 |
| GC (djs: copying + mark-compact; QuickJS: cycle collector) | 10,676 | 5,211 | 19,321 | 4,956 | 1.05 |
| builtins, plain (djs: intrinsic leaves) | 89,747 | 31,419 | 186,449 | 51,480 | 0.61 |
| builtins, generated Rust (`@native`) | 49,359 | 11,863 | – | – | |
| builtins, fast paths and inline caches | 29,803 | 8,971 | – | – | |
| realm setup from tables | 357 | 222 | 11,637 | 4,091 | |
| regexp | 79,121 | 31,110 | 48,758 | 17,631 | 1.76 |
| JSON | 39,607 | 14,906 | 15,078 | 5,562 | 2.68 |
| BigInt | 23,184 | 9,560 | 10,873 | 4,628 | 2.07 |
| number parse/format | 19,059 | 8,765 | 13,385 | 6,091 | 1.44 |
| unicode | 15,354 | 7,055 | 13,067 | 5,547 | 1.27 |
| Date | 6,523 | 2,779 | 11,592 | 4,992 | 0.56 |
| modules | – | – | 17,531 | 6,611 | |
| snapshots, suspension, wire format | 14,249 | 6,196 | 591 | 284 | |
| host API / ABI | 24,708 | 9,349 | 489 | 321 | |
| std: fmt, panic, malloc, generics / libc: printf, malloc, qsort | 42,778 | 16,172 | 23,336 | 9,674 | 1.67 |
| libm, soft float | 19,809 | 8,656 | 21,848 | 9,535 | 0.91 |
| **code total** | **910,248** | **344,207** | **709,894** | **237,105** | **1.45** |

Data:

| | djs raw | djs gz | QuickJS raw | QuickJS gz |
| --- | ---: | ---: | ---: | ---: |
| base image (the realm: builtins as bytecode, objects, strings) | 385,440 | 105,251 | – | – |
| function lists, atoms, opcode/class tables, messages | 73,429 | 28,282 | 51,459 | 21,797 |
| Unicode tables | 106,591 | 46,517 | 42,868 | 33,403 |
| **data total** | **565,945** | **182,597** | **96,347** | **59,627** |

(quickjs-wasi's data is 116 KB / 70 KB gz; it has more Unicode data than dquickts.)

How to read it: where the two engines do the same job in the same place, djs's code is
1.7–2.1x QuickJS's gzip (compiler, lexer, runtime core, regexp, BigInt, std). Where the
work is a big `switch` either way (the dispatch loop, GC, libm) the ratio is ~1. The
builtins row is *smaller* in djs only because most of the builtins' logic lives in the
base image, which is data.

## 3. Root causes, ranked

### 3.1 Rust code density (+90 KB gz on the same features)

On equal features djs's code is ~1.7x QuickJS's gzip. Measurements of where that comes
from:

**Controlled kernels** (`scripts/size/density/`): six small functions written the way
QuickJS writes C and the way djs writes safe Rust, compiled with clang `-Os` and rustc
`opt-level=s`, both with compressed relocations.

| kernel | C -Os | Rust s | ratio | what differs |
| --- | ---: | ---: | ---: | --- |
| escaping writer into a growable buffer | 302 | 922 | 3.05 | `Vec::push` inlined at 7 sites (capacity check + grow call); C calls one out-of-line `dbuf_put` |
| expression parser with error propagation | 842 | 1,196 | 1.42 | `Result<i32, Error>` moved through memory at every `?` (boxed error: 1,129) |
| UTF-8 decoder | 302 | 428 | 1.42 | bounds checks on input and output slices |
| gather-sum | 63 | 93 | 1.48 | bounds check per element |
| hash lookup | 78 | 99 | 1.27 | bounds check |
| number scanner | 210 | 214 | 1.02 | checks hoisted: no difference |
| **total** | **1,797** | **2,952** | **1.64** | |

**Where djs's bytes come from by source line** (inlined code charged to where it is
written; the line-table build, 977 KB): 38% of the code is Rust library code inlined
into djs functions: `Vec`/`RawVec` 105 KB, `Result` 63 KB, slices 41 KB, pointer/mem 34
KB, numeric helpers 25 KB, `Option` 24 KB, iterators 22 KB. Plus djs's own checked
arena accessors (`heap/layout.rs`) 41 KB and NaN-box encode/decode 35 KB. QuickJS has
the same kind of overhead, smaller: 18% of its code is inlined from its headers, mostly
reference counting (`JS_FreeValue`/`JS_DupValue`, ~60 KB) and exception checks (7 KB).

**Experiments** (variant builds of main, module gz):

| change | code gz | module gz | Δ |
| --- | ---: | ---: | ---: |
| shipped (opt-level s) | 344,219 | 529,901 | |
| no arena bounds checks (`get_unchecked`, needs unsafe) | 333,131 | 518,703 | −11.2 KB |
| panics abort without messages (nightly `-Cpanic=immediate-abort`, build-std) | 332,428 | 510,991 | −17.5 KB (vs. a nightly build-std baseline 528,464) |
| opt-level z | 307,673 | 494,821 | −35 KB, +30–70% instructions (perf) |
| opt-level 3 | 423,121 | 607,777 | +78 KB |
| (QuickJS -O2 instead of -Os) | | | code 1,076 KB raw vs 710 KB |

0057 already took the cheap ones: compressed relocations, emitter helpers out of line,
boxed compiler and interpreter errors (−150 KB raw, −20 KB gz). Monomorphization:
generic instances beyond one per generic function add up to 60 KB raw (20 KB gz on
their own), but most are distinct code (each `collect()` site, each export closure); the
true duplicates (JSON parser x3, number parsing x2, ...) are ~10–15 KB raw.

So the density gap is a sum of small taxes, not one cause: safety checks (~11 KB gz
arena + slices), panic paths (~17 KB gz), inline-by-default containers and error
plumbing (most of the rest). C gets the same work done with out-of-line helpers and int
error codes. The checks are fundamental for safe Rust (no `unsafe` in djs-core); the
container and error idioms are a style that can be changed where it is hot in size.

### 3.2 Builtins as JavaScript in a shipped image (+81 KB gz net)

Design: builtins are written in the builtin language (TypeScript subset), compiled to
bytecode at build time and shipped inside the module as a heap image that `vm_new`
copies (ARCH G2, D1321). QuickJS writes builtins in C and builds the realm at
`JS_NewContext` from `JSCFunctionListEntry` tables: 10 KB of tables for ~640 builtins,
11.6 KB of realm-setup code.

What is in the image (385 KB raw; gzip by kind, total 97 KB when grouped by kind):

| cell kind | raw | gz | what |
| --- | ---: | ---: | --- |
| bytecode | 101,248 | 27,763 | 874 function bodies |
| strings (Str8) | 79,904 | 26,346 | 2,360 strings, only 33,754 bytes of characters |
| function templates | 48,944 | 10,584 | 56 B per function |
| U32Array | 38,808 | 9,204 | atom hash index (16 KB), capture descriptors (14 KB) |
| objects | 40,688 | 7,304 | mostly the 874 function objects |
| ValueArray | 34,728 | 5,718 | constant pools, atom entries, value stack |
| properties (dictionary tables, shape values, shapes, transitions) | 34,792 | 6,354 | |
| rest | 6,328 | 1,369 | boxes, accessor pairs, realm, symbols |

Per builtin function that is ~440 B raw / ~120 B gz, against ~290 B raw / ~80 B gz of C
plus a 16-byte table entry in QuickJS. And it replaces little code: djs still has 31 KB
gz of Rust intrinsic leaves behind the builtins where QuickJS has 51 KB gz of builtin C,
and QuickJS spends 4 KB gz on realm-building code djs doesn't need. Net: 105 − 20 − 4 =
+81 KB gz. (The generated `@native` builtins and fast paths, 21 KB gz more, are counted
under 3.3.)

Three things make the image bigger than it needs to be:

- **Internal names.** 965 of the 2,360 strings are names of internal prelude functions
  (`ArrayBufferPrototypeSlice`, `IsCompatiblePropertyDescriptor`) and 529 are names of
  prelude locals and parameters (`actualStart`, `descriptors`), interned as atoms when
  the prelude is compiled and kept by the atom table: 52 KB of the 80 KB of string
  cells, and two thirds of the atom index and atom entries. A C engine has none of
  these at run time.
- **Cell headers and alignment.** Strings carry a 16-byte header and 8-byte padding for
  14 bytes of characters on average; 52% of the image's bytes are zero. gzip
  removes much of that, but not all: the strings' characters alone gzip to 10.7 KB, the
  string cells to 26.3 KB.
- **Bytecode density.** The same source compiles to 1.36–1.97x QuickJS's bytecode
  (`scripts/size/bytecode-density.py`: typescript.js 1.36x, dquickts's test_builtin.js
  1.70x, microbench.js 1.97x). djs has no short forms (QuickJS has `get_loc0`..`3`,
  `push_0`..`7`, 8-bit jumps), uses 2-byte local indexes and 4-byte jump offsets, and
  each property access carries a 2-byte inline-cache slot. QuickJS runs a peephole
  optimizer over its bytecode (part of its 37 KB gz compiler); djs compiles in one pass.

Measured by perf (0122): xz of the image is 79 KB; ordering it by kind gains 9 KB gz;
shipping the prelude source and compiling it per instance instead would save ~35 KB gz
net but costs ~100M instructions per instance.

Fundamental: shipping a materialized realm for a microsecond `vm_new` (a design goal)
and keeping the builtins' logic in a language the snapshot can capture. Incidental:
the internal names, the cell encoding of a read-only image, bytecode density, and the
duplicated `@native`/`@fast` versions.

### 3.3 Speed and embedding features QuickJS doesn't have (+48 KB gz), and Unicode (+15)

| feature | gz | note |
| --- | ---: | --- |
| generated `@native` builtins (30 builtins as Rust state machines) | 11,863 | duplicates of image builtins, 1.3–2.5x faster (perf) |
| hand-written fast paths, inline caches | 8,971 | |
| second dispatch loop for hot ops (`run_hot`) | ~4,500 | inside the dispatch-loop row |
| JSON: Latin-1/UTF-16/UTF-8 parsers, fast stringify | +9,300 | vs QuickJS's one path |
| Unicode tables, looser encoding (107 vs 43 KB raw), and code | +14,600 | QuickJS packs its tables with a custom run-length scheme; counted separately |
| TypeScript | 13,451 | dquickts has it too (10.6 KB): +2.9; quickjs-wasi does not: +13.5 |
| suspension, heap images, wire format, verification | 6,196 | quickjs-wasi snapshots by copying linear memory |
| host ABI (djs-wasm exports, `Vm` API) | 9,349 | quickjs-wasi's glue: 5.4 KB raw |

Determinism costs next to nothing in code (canonical hashing and collection order,
~3 KB raw). The two collectors together (10.7 KB raw) are smaller than QuickJS's
reference counting and cycle collector.

### 3.4 What QuickJS has that djs doesn't (−9 KB gz)

Modules (6.6 KB gz) and Date (5.0 vs 2.8: djs's Date is mostly prelude bytecode). The
realm-building code (4.1) is netted in 3.2. quickjs-wasi is also 12.8 KB gz smaller than
our dquickts build, 10.6 of it TypeScript.

## 4. The gap, decomposed

Against dquickts (area deltas from section 2):

| cause | KB gz | from |
| --- | ---: | --- |
| Rust density on shared features (3.1) | +90.4 | compiler 27.2, runtime core 31.7, regexp 13.5, std 6.5, BigInt 4.9, lexer 2.8, number 2.7, dispatch loop 1.7, GC 0.3, libm −0.9 |
| builtins in the image (3.2) | +81.4 | image 105.3, builtin leaves −20.1, realm setup −3.9 |
| speed/embedding features (3.3) | +48.0 | `@native` 11.9, fast paths 9.0, JSON 9.3, host ABI 9.0, snapshots/suspension/wire 5.9, TypeScript 2.9 |
| Unicode | +14.6 | tables 13.1, code 1.5 |
| other data | +6.5 | messages, panic locations, tables |
| QuickJS-only | −8.8 | modules 6.6, Date 2.2 |
| **djs − dquickts** | **+232** | (measured: 230.5) |
| dquickts − quickjs-wasi | +12.8 | TypeScript 10.6 |
| **djs − quickjs-wasi** | **+243** | |

Each row is ±10 KB: the areas are sorted by name patterns, and some growth inside an
area is a feature rather than density (inline caches and shapes in the runtime core,
v-flag class sets in regexp).

## 5. Fundamental or incidental

| cause | KB gz | fundamental (~) | incidental (~) |
| --- | ---: | --- | --- |
| Rust density | 90 | ~35: bounds checks (11 for the arena, more for slices), error values instead of a global exception flag | ~55: panic messages and format machinery (17), containers and error plumbing inlined at every site, monomorphized duplicates (5), iterator chains |
| builtins in the image | 81 | ~45: a shipped, materialized realm (fast `vm_new`, whole-VM snapshots) | ~35: internal names (~15), cell encoding and order of a read-only image (~9, perf), bytecode density (~9) |
| speed/embedding | 48 | ~25: suspension, images, wire format, host ABI, inline caches | ~25: `@native`/`@fast` duplicates where they don't pay, per-width JSON copies |
| Unicode | 15 | ~2 | ~13: the encoding |
| other data | 6 | – | ~6 |
| QuickJS-only | −9 | | |
| **total** | **232** | **~105 (45%)** | **~135 (55%)** |

## 6. What 1.5x and 1.0x would take

**~1.5x (430 KB gz, −100 KB)** is reachable inside the current design, at small
performance cost (0122 is on it): a compact image (no internal names, grouped by kind,
denser bytecode for privileged code: −30–40), Unicode tables in a run-length encoding
like QuickJS's (−13), panics without format machinery on stable (a non-formatting panic
handler and fewer `expect`s: up to −17), out-of-line containers and fewer monomorphized
copies in size-hot code (−10–15), `@native`/`@fast` only where benchmarks pay (−10–20),
one JSON input path for cold widths (−5).

**~1.0x (287 KB gz, −243 KB)** is not reachable by trimming; it needs a different engine:

- no image in the module: build the realm at startup from compact tables as QuickJS does
  (losing the microsecond `vm_new`, unless the host caches an image it built once), or
  ship the image outside the module;
- builtins as native code at C density: written in Rust in a C-like style (out-of-line
  buffer and error helpers, int-like errors, few iterator chains), without bytecode
  twins and fast-path duplicates;
- Rust density near C in the compiler and runtime: the ~1.7x gzip ratio would have to
  come down to ~1.1–1.2x, which means giving up much of what makes the code idiomatic
  and safe (`Result`/`?` everywhere, `Vec` growth inline, checked indexing) or proving
  bounds once per cell instead of per field;
- dropping the features QuickJS doesn't have, or accepting them as the remaining
  difference (~48 KB gz).

## 7. Reproducing

- `scripts/size/areas.py djs SHIPPED.wasm NAMES.wasm` and `areas.py c QJS.wasm LINES.txt`:
  the area tables (`--list AREA` for the functions of one area).
- `scripts/size/qjs/build.sh`: QuickJS for wasm with names and line tables, and the
  native `bcsize` for `scripts/size/bytecode-density.py`.
- `scripts/size/density/run.py`: the C vs Rust kernels.
- `scripts/size/variant.sh NAME [PATCH.py] [-- cargo args]`: variant builds;
  `scripts/size/patches/nobounds.py` is the bounds-check experiment.
- `scripts/wasm-dwarf-lines.py`: bytes by source line (needs a line-table build).
- perf's (0122) `scripts/size/sections.py`, `image-kinds.py`, `families.py`.
