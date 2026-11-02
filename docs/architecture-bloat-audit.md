# Architecture bloat audit: what would half the module take?

Issue 0156, measured at main 40ad9290 (2026-11-09). Research only: nothing in the engine
changes. Experiments are variant builds in scratch copies (`scripts/size/variant.sh`),
never merged. Sizes are bytes of the shipped module (stripped); "gz" is gzip -9; KB is
1000 bytes, KiB 1024.

The owner asked:

> I'm generally quite puzzled about the overall memory and binary size. A meaningful part
> comes from Rust compared to C, but I would have assumed you can get it to half the binary
> size trivially, considering the generally reduced footprint of this implementation
> compared to a full JS one. I wonder if we're missing some meaningful bloat from
> architectural missteps. Investigate just that.

## The answer in short

- **djs is not a reduced engine.** It implements the same language and library as
  QuickJS (99.9% of 38,109 active test262 tests; ES2025 plus most stage-3 proposals),
  and it adds TypeScript, suspension anywhere, snapshots, fuel, determinism, the wire
  format and two speed tiers. What it leaves out (sloppy mode, `with`, direct `eval`,
  modules, Annex B; Intl and Temporal, which QuickJS lacks too) is worth under 10 KB gz
  of QuickJS's code. What it adds is about 45 KB gz, before its speed tiers (27 KB). So "half of a full engine" would
  have had to come from a smaller language, and the language isn't smaller.
- **No big architectural misstep is hiding.** Every duplicate path was removed in a
  variant build and measured:
  - the second interpreter loop: −6.7 KB gz;
  - the `@native` builtin backend: −10.6 KB;
  - the `@fast` entries: −8.3 KB;
  - all three together: −26.8 KB (5.9% of the module).

  Each one buys measured speed. The other duplicates are smaller still: the JSON
  parser's three input widths (~3.5 KB), the second collector (~1.3 KB), duplicate
  monomorphized copies (≤ 5 KB), panic and format code (~3 KB). Removing every
  duplicate would leave the module at ~94% of today's size.
- **Where the bytes are.** Today: **454.8 KB gz**. That is code 343.5 KB, the coded
  realm image 51.5 KB, Unicode tables 32.3 KB and other data 21 KB. Code is 75% of the
  module and 80% of the gap to quickjs-wasi (286.6 KB). Against QuickJS (the dquickts
  build of `docs/size-vs-quickjs.md`), the gap of ~155 KB gz breaks down as:
  - **~90 KB:** Rust code density on the features both engines have. This is unchanged
    since 0123: the compiler, runtime core, regexp, BigInt and number code are 1.4–2.1x
    QuickJS's gzip.
  - **~42 KB:** builtins in two languages (the bytecode image plus Rust leaves plus
    their fast twins), against QuickJS's C. The coded image halved this since 0123.
  - **~34 KB:** djs-only features (the host ABI, JSON's input widths, snapshots and the
    wire format, TypeScript's extra), less ~8 KB of QuickJS-only ones (modules).
- **Half (227 KB gz) is 0.79x quickjs-wasi.** No amount of removing bloat gets there. Every
  structural cut measured here, stacked:
  - the slim codemode language set;
  - one execution path;
  - unchecked arena access;
  - opt-level z.

  Together they give **342.2 KB (−24.7%)**. The remaining ~115 KB would need the realm
  image out of the module, TypeScript on the host, C-style Rust throughout, and a
  language cut beyond the slim set. That is a different engine. A realistic floor for
  this design is ~0.6–0.65x of today.
- **Memory is not bloated relative to QuickJS:**

  | | wasm memory | resident (RSS) |
  | --- | ---: | ---: |
  | fresh djs Sandbox | 1.19 MiB (19 pages) | ~700–720 KiB |
  | fresh quickjs-wasi VM | 1.31 MiB (21 pages) | ~725 KiB |

  The 19 pages are:
  - the 256 KiB shadow stack;
  - 3 pages of static data;
  - **11 pages of arena**: the 295 KiB base and **409 KiB of reserved, untouched GC
    headroom**;
  - 1 page of Rust heap.

  Two small changes take a fresh sandbox to 0.75 MiB (12 pages, prototyped), a third to
  ~0.69 MiB. One of them needs care so growing heaps don't end higher (section 6).
- **Plan (section 8)**, by bytes:
  1. a codemode module: −52 KB gz, no speed cost;
  2. unchecked arena accessors: −12 KB, faster;
  3. keep the speed twins only where benchmarks pay: up to −19 KB, keeping `run_hot`;
  4. errors that don't travel through memory: `Fault` as a zero-sized type plus a niche
     in `Value`, est. −5–10 KB;
  5. the memory changes: −8 pages per VM.

  Together, ~365–375 KB gz (1.28–1.31x quickjs-wasi) at little speed loss. Global opt-level z is not
  worth its 30–70% more instructions.

## 1. Where the bytes are

| part | raw | gz | share of gz |
| --- | ---: | ---: | ---: |
| code | 893,325 | 343,526 | 75.5% |
| base image, coded with djs-lzrc (incompressible to gzip) | 51,450 | 51,476 | 11.3% |
| Unicode tables | 58,299 | 32,323 | 7.1% |
| text constants: error and panic messages, layout field names, DOMException names | 30,504 | 12,023 | 2.6% |
| binary constants: switch and number tables, compactor plans | 20,760 | 7,537 | 1.7% |
| other named statics | 2,466 | 1,507 | 0.3% |
| **module** | **1,063,668** | **454,771** | |
| quickjs-wasi 3.6.2 | 637,405 | 286,630 | code 208,918, data 70,459 |

Data is attributed from a link map (`scripts/size/datamap.py`). Code by area
(`scripts/size/areas.py`; QuickJS columns from `docs/size-vs-quickjs.md`, dquickts at -Os):

| area | djs raw | djs gz | QuickJS gz | ratio |
| --- | ---: | ---: | ---: | ---: |
| runtime core (calls, properties, objects, strings, atoms, operators, errors) | 195,222 | 75,483 | 41,152 | 1.83 |
| compiler (parser, scopes, codegen) | 150,742 | 61,557 | 37,085 | 1.66 |
| regexp | 78,493 | 30,457 | 17,631 | 1.73 |
| builtins: intrinsic leaves | 86,186 | 30,066 | 51,480 (all builtin C) | 0.58 |
| interpreter dispatch (`step_once` + `run_hot`) | 53,893 | 15,937 | 13,646 | 1.17 |
| JSON | 40,762 | 15,406 | 5,562 | 2.77 |
| host API / ABI, allocator | 33,152 | 13,898 | 321 | |
| TypeScript | 33,413 | 13,117 | 10,558 (quickjs-wasi: 0) | 1.24 |
| std: fmt, panic, out-of-line generics | 29,204 | 10,914 | 9,674 | 1.13 |
| BigInt | 23,264 | 9,510 | 4,628 | 2.05 |
| builtins: hand-written fast paths, inline caches | 29,328 | 9,258 | – | |
| number parse/format | 19,779 | 9,107 | 6,091 | 1.50 |
| unicode code | 19,383 | 8,733 | 5,547 | 1.57 |
| libm | 19,127 | 8,617 | 9,535 | 0.90 |
| snapshots, suspension, wire format | 18,830 | 7,797 | 284 | |
| builtins: generated Rust (`@native`) | 24,859 | 6,860 | – | |
| GC | 14,972 | 6,702 | 4,956 | 1.35 |
| lexer | 14,072 | 6,072 | 3,265 | 1.86 |
| Date | 8,189 | 3,779 | 4,992 | 0.76 |
| modules | – | – | 6,611 | |
| realm setup | 357 | 225 | 4,091 | |
| **code** | **893,239** | **343,523** | **237,105** | **1.45** |

What changed since 0123 (529.9 KB): the image went from 105 to 51.5 KB gz (0122 coded it,
0151 shrank it), the `@native` set from 30 to 10 builtins, and panic/format code from
~17 to ~3 KB gz. The code ratio to QuickJS is still 1.45, and the density on shared
features is still ~+90 KB. The ~155 KB gz gap to dquickts (299 KB) decomposes (each row
±10 KB) as:

| cause | KB gz | how |
| --- | ---: | --- |
| Rust density on shared features | ~+90 | compiler +24.5, runtime core +34.3, regexp +12.8, BigInt +4.9, number +3.0, unicode +3.2, lexer +2.8, loop +2.3, GC +1.7, std +1.2, libm −0.9 |
| builtins in two languages | ~+42 | djs: image 51.5 + leaves 30.1 + `@native` 6.9 + `@fast` 9.3 = 97.8; QuickJS: builtin C 51.5 + realm setup 4.1 = 55.6 |
| djs-only features | ~+34 | host ABI and allocator 13.6, JSON's three widths and fast stringify 9.8, snapshots/suspension/wire 7.5, TypeScript +2.6 (+13.1 against quickjs-wasi, which has none) |
| QuickJS-only features | −8 | modules 6.6, Date 1.2 |
| data other than the image | ~−6 | djs 53.4 (Unicode 32.3) vs QuickJS 59.6 |

## 2. Is djs actually reduced?

No. By feature surface it is a full engine, and then some:

| | djs | QuickJS / quickjs-wasi | in codemode scripts (0149) |
| --- | --- | --- | --- |
| ES2025 language and library | yes, strict only; 38,069 of 38,109 active test262 tests | yes, plus sloppy mode, `with`, Annex B | the core: Promise, JSON, String, Object, Error, Date, Math, Set, Map, Array, RegExp |
| stage 3/4 proposals: decorators, `using`, iterator helpers, Float16Array, `RegExp.escape`, `Math.sumPrecise`, `Promise.allKeyed`, `JSON.rawJSON`, `Error.isError` | yes | partly | none in 2,687 scripts |
| BigInt, typed arrays, ArrayBuffer/DataView, Proxy/Reflect, WeakRef, generators | yes | yes | only Proxy/Reflect (pi's `tools`), generators rarely |
| Unicode property escapes, the v flag, `normalize` | yes (34.5 KB gz) | yes | none |
| TypeScript stripping | yes (13.1 KB gz) | dquickts yes, quickjs-wasi no | the scripts are TS |
| modules, dynamic import | no | yes (6.6 KB gz) | no |
| Intl, Temporal, SharedArrayBuffer | no | no | no |
| suspend anywhere, fuel, snapshots, determinism, wire format, frozen builtins, host calls | yes (~22 KB gz incl. the ABI) | snapshots by copying linear memory | all of it |
| speed tiers: the tier-0 loop, `@native`, `@fast`, inline caches, `iter_next`, `%EachDense` | yes (~27 KB gz for the first three) | inline caches only | |

The non-goals that ARCH lists cost QuickJS little code: sloppy-mode and `with` handling are
scattered through its compiler and runtime, a few KB; direct eval rides on the same
compiler; modules are 6.6 KB gz. The 310 corpus runs touch 135 of ~960 builtins
(0149), but the engine must implement all of them for test262 and for whatever a model
writes next. So the "reduced footprint" exists in usage, not in the engine. A smaller
module requires a smaller language: the slim set below.

## 3. Duplicated machinery, measured

Variant builds of main (`scripts/size/variant.sh`; patches in `scripts/size/patches/`):

| variant | module gz | Δ | what it removes | what it costs |
| --- | ---: | ---: | --- | --- |
| shipped | 454,771 | | | |
| no `@native` (`onepath.py`, `ONEPATH=native`) | 444,216 | −10,555 | 10 generated state machines (sort, JSON slow path, promise jobs, push, GetPrototypeFromConstructor) and their frames | those builtins 1.3–2.5x slower (0123) |
| no `@fast` (`ONEPATH=fast`) | 446,437 | −8,334 | 73 Rust entries for everyday builtins | `includes`, `split`, `slice`, ... back to bytecode: 0081 found codemode time dominated by them |
| no `run_hot` (`ONEPATH=hot`) | 448,037 | −6,734 | the tier-0 loop | +19–27% wasm instructions on plain bytecode, up to +26% on arrays/strings (D1930) |
| one path (all three) | 427,952 | −26,819 | | all of the above |

Duplicates not worth a variant, by function size (name-section build):

| duplicate | code raw | est. gz | note |
| --- | ---: | ---: | --- |
| JSON parser per input width (Latin-1, UTF-16, UTF-8) | 10,825 | ~3.5 | `next`, `key_colon`, `scan_rest` instances beyond one, plus `parse_utf8` |
| Cheney collector for canonical images, next to mark-compact | 3,249 | ~1.3 | `collect_canonical`, `forward`, `trace` |
| image verification | 9,991 | ~4 | needed for untrusted snapshots, not a duplicate |
| wire decoder (the encoder is in JS) | 7,174 | ~3 | one path in the module |
| string representations (Latin-1/UTF-16/ropes/append buffers): functions generic over or named for a representation | 23,383 | ~8 | mostly one implementation each; the generic `Chunks`/`Units` iterator instances are the duplication, a few KB |
| monomorphized copies beyond one per generic function | ≤ 78,231 | ≤ 5 | an upper bound: most are distinct `collect`/`extend` sites, as 0123 found |

The heap readers in `js/src/reader.ts` cost the module nothing (host code); the boundary
has three inbound paths (wire, JSON, handles) and two outbound ones (direct reads, JSON).
JSON and the wire format share the parser and the string code.

**Why the builtins exist several times** is the one structural pattern behind several
rows. Suspend-anywhere (G5) forbids hand-written Rust that calls back into JS (the leaf
rule). Every builtin that might run user code is therefore TypeScript compiled to
bytecode in the image, over Rust leaf intrinsics (`call_intrinsic`, 38 KB raw, and
`json_regexp_intrinsic`, 15 KB, dispatching 309 intrinsics). Speed then needs twins for
the hot ones: a generated Rust state machine (`@native`) or a Rust entry that runs when
nothing observable can happen (`@fast`). In QuickJS a builtin is one C function. This
design is the price of G5, and the twins are the price of G5 being fast. Removing all
twins buys 19 KB gz; the image is 51.5 KB; together they are the ~42 KB row in section 1
(net of the C that QuickJS needs instead).

## 4. Rust multipliers

**By source line** (a DWARF line-table build, 957,570 bytes covered; inlined code charged
to the line it comes from; `scripts/wasm-dwarf-lines.py`). 34.7% of the code is Rust
library code inlined into djs functions (0123: 38%):

| source | raw | share | what |
| --- | ---: | ---: | --- |
| `Vec`/`RawVec` | 100,589 | 10.5% | buffer pointer and length loads (`as_ptr`, deref: would exist in C), `push`'s capacity check and `grow_one` call, **drop glue (raw_vec.rs:425, 14.5 KB)**, `with_capacity` |
| `Result` | 56,143 | 5.9% | `?`: `branch` 30.5 KB and `from_residual` 17.9 KB: results returned through memory and re-tested at each level |
| slices | 41,771 | 4.4% | bounds checks and range indexing |
| `ptr`/`mem` | 33,026 | 3.4% | |
| integer helpers (checked, saturating) | 25,226 | 2.6% | |
| `Option` | 24,594 | 2.6% | |
| iterators | 23,948 | 2.5% | |
| djs's arena accessors (`heap/layout.rs`) | 43,271 | 4.5% | the bounds-checked field reads and writes, ~1,300 inlined sites |
| NaN-box encode/decode (`value.rs`) | 35,184 | 3.7% | |

Experiments (module gz):

| variant | Δ | note |
| --- | ---: | --- |
| arena accessors without bounds checks (`patches/nobounds.py`, needs unsafe) | −11,752 | 0123: −11.2 KB |
| opt-level z, whole module | −27,410 | +30–70% instructions (0123) |
| `#[optimize(size)]` on every djs-core function (`patches/optsize.py`, `OPTSIZE=all`) | −25,763 | the attribute reproduces opt-level z |
| `#[optimize(size)]` on the cold modules only: compiler, lexer, regexp parser/compiler, BigInt, wire, unicode (748 functions) | **−4,219** | **the z gain is in the hot code**: runtime, interpreter, heap |

So:
- Monomorphization is not a cause (≤ 5 KB; confirms 0123).
- Inlining is already tuned (D2255's inline threshold).
- Panic and format code is down to ~8 KB raw.
- The cold-code trick (size-optimize what is not hot) gives only 4 KB.

What remains is per-call-site overhead: checked accessors (12 KB gz), `Result`
plumbing (~48 KB raw), drop glue and `push` (~20 KB raw), and slice bounds checks. In
wasm a `Result<Value, Fault>` (16 bytes; `Fault` is a `Box`) is returned through a stack
slot. The caller loads the tag, branches, and copies the payload. QuickJS returns a
`JSValue` with an exception tag in a register and keeps the exception in the context.
djs could get most of that without giving up `?`: make `Fault` zero-sized (the pending
error lives in the heap or the `Exec`) and give `Value` and the reference types a niche,
so that `Result<Value, Fault>` and `Result<Obj, Fault>` are one scalar. Estimated −15–30
KB raw (−5–10 KB gz); not measured, it is a refactor of every fallible function.

## 5. Data tables

- **The realm image** is 296 KB raw, 4,848 cells, coded to 51.5 KB:
  - bytecode 102 KB raw (28.7 KB gz on its own), 932 cells;
  - function templates 49 KB (875 cells at 56 B);
  - constant pools and atom entries (ValueArray) 38 KB;
  - objects 30 KB;
  - Latin-1 strings 26 KB (809 cells, down from 2,360 before 0151 dropped the internal
    names);
  - property values and tables 32 KB;
  - U32Arrays 13 KB.

  The coding (D2252) already removed the cell-header waste that 0123 counted. What is
  left is mostly the builtins' logic as bytecode, which C would have as code.
- **Unicode** (58 KB raw, 32.3 KB gz, plus 8.7 KB gz of code; −34.5 KB gz with both
  features off): decomposition tables 21 KB, property data 15 KB, general category runs
  5 KB, identifier and case tables 12 KB. QuickJS packs the same data tighter (0123: 43
  vs 107 KB raw at the time). Nothing in 2,687 codemode scripts uses `\p{}` or
  `normalize`.
- **Text constants** (30.5 KB raw, 12 KB gz): JS error messages, panic messages that
  survive `-Zlocation-detail=none`, the heap layout's field names (`PropTable`,
  `COUNT`, `DELETED`, ...: used by image verification and the layout export), and
  DOMException's name table. Small; there's no structural issue here.

## 6. Memory per VM

A fresh Sandbox (second and later instances; the first decodes the base itself),
measured with `scripts/size/sandbox-memory.ts` on an `alloc-stats` build:

| part | wasm memory | resident |
| --- | ---: | --- |
| shadow stack (D1941) | 256 KiB, pages 0–3 | only the touched part |
| static data: Unicode 57 KiB, the coded base 50 KiB, messages and tables 50 KiB | 161 KB, pages 4–6 | all of it (copied in at instantiation) |
| arena: the base | 295 KiB of 704 | all of it |
| arena: reserved headroom | **409 KiB** | none until used |
| Rust heap (dlmalloc: the VM struct, handles, side tables) | 1 page, ~17 KB live | |
| **total** | **1,216 KiB = 19 pages** | **~700–720 KiB** (RSS over 200 sandboxes) |
| quickjs-wasi, fresh VM | 1,344 KiB = 21 pages (1 MiB stack, data, heap) | ~725 KiB |

The arena's capacity is the base's first GC threshold plus slack:
- `next_threshold(301,704)` is 2x live, page-rounded: 655,360;
- `capacity_target` adds 64 KiB: 720,896 = 11 pages.

The arena reserves this up front so that its growth doesn't move it (D2053). The
threshold counts the base, although minor collections never trace or move the base (the
old prefix).

Prototype (`work/scratch/0156/patches/memlean.py`: reserve the base plus one page; a
128 KiB stack via `DJS_WASM_STACK`) starts a sandbox at **12 pages (0.75 MiB)**. But
growing heaps then end 1–4 pages *higher*:

| workload | main | prototype |
| --- | ---: | ---: |
| 1,000 objects | 19 | 22 |
| 400k short-lived arrays | 21 | 25 |
| 1 MB JSON | 78 | 77 |

The cause: dlmalloc takes pages after the arena, so the arena can no longer grow in
place and moves, leaving a hole. A real change needs the page allocator to keep the
arena on top: dlmalloc's pages below it, or a page reserved for dlmalloc before
`vm_new`.

Cheap memory changes, then:
1. Reserve the base plus a page instead of the first GC threshold's capacity, and grow on
   demand with the arena kept at the end of memory: −5 pages at start. The threshold,
   and so every collection point, stays the same; only the reservation changes.
2. A 128 KiB stack: −2 pages. It needs a lower parser nesting limit; the deepest
   accepted nesting needs ~170 KiB today.
3. The coded base out of linear memory: −1 page. Hosts inject the decoded base anyway.
   Use a passive segment dropped after the decode, or a custom section the host hands
   over.

Together that is ~11 pages (0.69 MiB). The lazy realm of `docs/empty-heap.md` would take
the base's 5 pages down to ~1. The headroom only counts against wasm memory, not RSS.
Resident memory is already on par with quickjs-wasi.

## 7. What half would take

Stacked variants (each row includes the ones above it):

| variant | module gz | Δ vs shipped | × quickjs-wasi |
| --- | ---: | ---: | ---: |
| shipped | 454,771 | | 1.59 |
| slim A: 0149's drop set, Unicode off, decorator/`using` syntax cut | 402,559 | −52,212 | 1.40 |
| + one path, + unchecked arena (`lean`) | 363,260 | −91,511 | 1.27 |
| + opt-level z (`leanz`) | 342,235 | −112,536 | 1.19 |
| half of shipped | 227,386 | −227,386 | 0.79 |

From `leanz` (code 268.5 KB gz, data 71.2 KB) to half is another −115 KB gz. The
candidates, none of them a trim:

| change | est. KB gz | what it means |
| --- | ---: | --- |
| realm image out of the module | −30–45 | the host ships and caches the base (the download does not shrink), or the realm is built at startup from tables with builtins as Rust code (QuickJS's way), which gives up the microsecond `vm_new` and brings back Rust code for the logic |
| TypeScript stripped on the host | −13 | node's `module.stripTypeScriptTypes`; browser hosts need their own; error positions must map back |
| C-style Rust in compiler and runtime | −20–40 | out-of-line buffer and error helpers, the niche-`Result` change, no iterator chains: a rewrite of ~350 KB of raw code |
| a smaller language than slim A | −10–30 | e.g. drop regexp lookbehind/named groups/v flag, async generators, class private methods, the second JSON width |

Even the optimistic ends add up to only ~230–250 KB, and only at full opt-level z (−30–70%
speed) and with features gone. Half the module means a smaller engine for a smaller
language. **The realistic floor for this design and feature set is ~340–365 KB
(0.75–0.8x of today)**, most of it reachable without a speed loss.

## 8. Plan: the structural changes that cut the most

Ranked by bytes per effort; the speed column is for the default (native) backend.

| # | change | module | per-VM memory | speed | effort | risk |
| --- | --- | ---: | ---: | --- | --- | --- |
| 1 | **A codemode module** (0149's slim A as a cargo feature; a second module for pi) | −52 KB gz (measured today) | base −85 KB (−1–2 pages) | none | 2–4 weeks (0149's phases) | pi must drop its lockdown for frozen builtins; a second module to gate |
| 2 | **Unchecked arena accessors** behind the image verifier (one audited `unsafe` pair in `heap/layout.rs`; ARCH lists hoisted checks as open, 0081) | −11.8 KB (measured) | – | a few % faster | days, plus a decision | a VM bug corrupts the instance's own memory instead of trapping; still inside the wasm sandbox |
| 3 | **One path where speed doesn't pay**: re-benchmark each `@native` and `@fast` twin and drop the ones under ~1.3x on the codemode corpus. Keep `run_hot`: 6.7 KB for −19–27% instructions is the best speed per byte in the module | up to −18.9 KB (both twins measured); realistic −8–12 | – | per twin, chosen | 1–2 weeks | slower everyday builtins if cut too far |
| 4 | **Errors as a zero-sized `Fault` plus a niche in `Value`/refs**, the pending error in the VM (QuickJS's `JS_EXCEPTION` model with `?` kept) | est. −5–10 KB | – | slightly faster (fewer memory round trips) | 3–5 weeks, mechanical | every error path; tests catch most; pending-error state must survive suspension |
| 5 | **Memory**: young-generation reservation with the arena kept on top, a 128 KiB stack, the coded base out of linear memory | ~0 | 19 → ~11 pages (1.19 → ~0.69 MiB) | none measurable (GC timing unchanged: only the capacity moves) | 1 week | growth must not move the arena (the prototype's lesson); the parser's nesting cap |

1–4 together: ~455 − 52 − 12 − 10 − 7 ≈ **365–375 KB gz (1.28–1.31x quickjs-wasi)**,
inside the D020 budget, at little speed loss (only the twins that don't pay go). Not
recommended:
- global opt-level z (−27 KB at +30–70% instructions);
- size-optimizing the cold modules (−4 KB);
- dropping the second collector (−1.3) or unifying the JSON widths (−3.5): their speed
  is worth more.

The lazy realm (`docs/empty-heap.md`) is the memory step after 5; it doesn't change
the module.

## 9. Re-checking earlier conclusions

- *"~half Rust density, ~half the bytecode image"* (0123): density, yes, unchanged at ~90
  KB gz. The image is now a quarter of the gap, not a half: it went 105 → 51.5 KB gz, and
  the twins went 21 → 16 KB.
- *"Monomorphization and the optimization level are not causes"*: confirmed. Opt-level z
  still saves 27 KB, but only in hot code (the cold-module experiment), so the speed cost
  is intrinsic.
- *"Panic and format paths 17 KB gz"*: no longer true. They are ~3 KB gz after
  `-Zlocation-detail=none` and the panic handler without messages.
- *"Two collectors cost nothing"*: confirmed (the Cheney-only code is 3.2 KB raw).
- *Empty heap* (0143): the per-VM base is 295 KiB now (322 then). Its wasm-memory cost is
  dominated by the GC headroom reserved on top of it (409 KiB), which 0143 didn't count.

## 10. Reproduce

```sh
scripts/build-wasm.sh                                     # shipped
CARGO_PROFILE_WASM_STRIP=debuginfo scripts/with-wasm-env.sh cargo build -p djs-wasm \
  --target wasm32-unknown-unknown --profile wasm          # names, same code
python3 scripts/size/areas.py djs NAMES.wasm NAMES.wasm   # section 1 areas
# data by symbol: a link map of the same build
OUT=work/scratch/x RUSTFLAGS="-Clink-arg=--Map=$PWD/work/scratch/x/map.txt" \
  CARGO_PROFILE_WASM_STRIP=debuginfo scripts/size/variant.sh mapped
python3 scripts/size/datamap.py work/scratch/x/map.txt work/scratch/x/mapped.wasm
# section 3/4 variants
scripts/size/variant.sh onepath scripts/size/patches/onepath.py          # ONEPATH=native|fast|hot
scripts/size/variant.sh nobounds scripts/size/patches/nobounds.py
OPTSIZE=all scripts/size/variant.sh allz scripts/size/patches/optsize.py # default: cold modules
CARGO_PROFILE_WASM_OPT_LEVEL=z scripts/size/variant.sh optz
# line attribution
CARGO_PROFILE_WASM_DEBUG=1 CARGO_PROFILE_WASM_STRIP=false scripts/size/variant.sh dwarf
llvm-dwarfdump --debug-line dwarf.wasm > lines.txt
python3 scripts/wasm-dwarf-lines.py dwarf.wasm lines.txt [--lines]
# memory: pages, arena, RSS per sandbox
scripts/size/variant.sh allocstats -- --features alloc-stats
node --expose-gc scripts/size/sandbox-memory.ts allocstats.wasm 200
```

The slim A and stacked variants applied 0149's prototype patch
(`work/scratch/0149/p-a-arms.py`, from the primary checkout) before `onepath.py` and
`nobounds.py`; the memory prototype is `work/scratch/0156/patches/memlean.py`. Raw
results: `work/scratch/0156/results.txt`.
