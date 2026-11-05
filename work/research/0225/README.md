# 0225: where the shipped module's bytes go

Measured at 23b0a946 (before 0225's changes) with the tools of scripts/size/ and the
scratch scripts of work/scratch/0225/ (secs.py: every section with its gzip; fdiff.py:
function-level diff of two named builds; hist.sh: named builds at old commits). All
numbers gzip -9 unless "raw".

## Every byte (shipped module, 1,033,538 raw / 442,576 gzip)

| section | raw | gzip |
| --- | ---: | ---: |
| code | 867,720 | 336,856 |
| data (one active segment) | 115,665 | 56,616 |
| custom `djs_base` (the base image, lzrc-coded) | 45,694 | 45,715 |
| function, type, export, elem, table, memory, global | 4,421 | 2,385 |

Code by area (`scripts/size/areas.py djs`, gzip scaled to the section):

| area | raw | gzip |
| --- | ---: | ---: |
| runtime core (calls, properties, objects, strings, atoms, operators, errors) | 193,810 | 75,295 |
| compiler (parser, scopes, codegen) | 155,887 | 64,069 |
| builtins: Rust intrinsic leaves | 84,452 | 29,307 |
| regexp | 74,980 | 29,081 |
| interpreter dispatch (`step_once`, `run_hot`) | 53,547 | 15,612 |
| JSON | 41,348 | 15,716 |
| host API / ABI (incl. dlmalloc, page allocator, lzrc decoder) | 40,153 | 16,697 |
| TypeScript | 33,692 | 13,246 |
| std: fmt, panic, malloc, out-of-line generics | 28,884 | 10,853 |
| BigInt | 22,927 | 9,367 |
| generated `@native` builtins | 20,874 | 5,807 |
| number parse/format | 19,779 | 9,109 |
| libm | 19,127 | 8,628 |
| unicode code | 16,843 | 7,775 |
| GC | 16,288 | 7,335 |
| `@fast` paths, inline caches | 14,167 | 5,274 |
| lexer | 14,053 | 6,043 |
| snapshots, suspension, wire | 8,375 | 3,632 |
| Date | 8,080 | 3,744 |

Largest functions: `call_intrinsic` 37.6 KB raw, `step_once` 34.1, `run_hot` 18.5,
`emit_function` 14.3, `json_regexp_intrinsic` 13.8, `regexp::exec::resume` 7.8,
`json::parse::parse` 7.4.

By source line (DWARF line table, inlined code charged to its source): Rust library code
inlined into djs functions is the biggest single cause: `Result` 53 KB raw (the `?`
moves: `Result<Value, Fault>` is 16 bytes and returns through memory), `RawVec`/`Vec`
76 KB (1,046 inlined `deallocate` calls in drop glue alone), `Option` 22 KB, slice
indexing 21 KB; the arena accessors (heap/layout/access.rs) 35 KB.

Panic paths: 656 `panic_bounds_check` calls, 427 arena `out_of_bounds`, 265
`slice_index_fail`, 220 `handle_error`, 145 `expect_failed`, 94 `panic_fmt`, plus core's
formatting code they keep reachable.

Data (`datamap.py`, link map): Unicode tables 58,299 raw / 32,332 gzip (properties 15.3
KB, decomposition 18.8 KB, general category runs 5.2 KB, scripts 2.8 KB, ...);
messages and names 30,959 / 12,240; binary lookup tables 21,197 / 7,587 (the opcode table
6.0 KB, the intrinsic table 5.4 KB: 452 entries of name pointer, length and signature);
other statics 2,546 / 1,562.

Base image: 186,376 raw, 3,100 cells, lzrc 45,694 (bytecode and function cells 87 KB raw,
809 strings 26 KB raw).

Monomorphization: 28 KB raw of generic instances beyond the largest instance of each
family, mostly deliberate per-width copies (JSON parser x3, wire transcode x2).

## What grew

Nothing, net, since 0161 (2026-11-03). Named builds of 0148 (de5efbcf), 0161 (c169e192),
0204 (f37ec601) and 23b0a946:

| | 0148 | 0161 | 0204 | 23b0a946 |
| --- | ---: | ---: | ---: | ---: |
| code gzip | 341,245 | 334,610 | 335,962 | 336,835 |
| base (coded) | in data | 47,188 | 45,655 | 45,715 |
| module gzip (ceiling) | 459,164 | 442,685 | 442,831 | 442,576 |

0161 → now by area (gzip): compiler +2.0 (completion values, try/finally, `UnitPool`),
host +1.3 (`open_host_call`, `vm_dump_runtime`, `vm_replace_image`), GC +0.7
(`find_in_level`, mark-compact), runtime core +0.9, JSON +0.5; snapshots −4.0 (0177
removed image verification). The module was never within the budget: the ratchet started
at 510 KB (0122) and came down to ~442 KB by 0161; since then each feature paid for itself
with cuts.

## Experiments (variant builds, module gzip vs 442,576 / 396,336 without base)

| change | Δ gzip | speed |
| --- | ---: | --- |
| build-std core+alloc, `-Cpanic=immediate-abort`, `-Zshare-generics=no` (landed, D3090) | −10.8 KB | −3.1 % … +2.3 % instructions |
| same without `-Zshare-generics=no` | −10.8 KB | +40 % tight/arrays/forof (core's `copy_from_slice_impl` out of line in `run_hot`) |
| build-std alone (panic=abort) | +0.1 KB | +40 % (same cause) |
| build-std, core+alloc+compiler_builtins at opt-level 3 | −6.9 KB | neutral |
| intrinsic run-time tables (landed, D3091) | −0.8 KB | ±0.8 % |
| `unsafe-heap` (unchecked arena) | −7.7 KB | faster; rejected for safety in D2520 |
| `#[optimize(size)]` on cold modules (patches/optsize.py) | −2.0 KB | compile speed: unmeasured |
| lzrc of the whole data segment (estimate: 114,407 → 49,956 coded) | ≈ −6.6 KB | a decode per module |
| lzrc of the Unicode tables alone | ≈ −3.1 KB | |
| wasm-opt -Oz / -Os / -O2 / -O3 (binaryen 123) | +1.1 / +1.4 / +3.5 / +1.7 KB | raw −6 % but worse gzip |
| `-Zfmt-debug=none` | −0.1 KB | |
| core's `optimize_for_size` | −0.06 KB | |

Earlier measurements still valid in kind (0156): no `@native` −10.6, no `@fast` −8.3, no
`run_hot` −6.7 (all perf trades), opt-level z −27 (+30–70 % instructions).

## Ranked waste (as sent to the coordinator)

1. Panic machinery and std built at O3 elsewhere: build-std + immediate-abort, −10.8
   (done, D3090).
2. `Result` returned through memory: a niche in `Value` and a zero-sized `Fault` (the
   pending error in the VM) would return `Result<Value, Fault>` in one register; est.
   −5 to −10 KB, likely faster. Deferred until 0224 lands.
3. Unchecked arena access: −7.7 (owner's call, D2520).
4. Data segment coded with lzrc like the base: ≈ −6.6 (prototype next).
5. Name tables: −0.8 (done, D3091; the estimate of 4–6 was too high: the pointer
   tables compressed well).
6. Small: optimize(size) on cold modules −2.0, Unicode re-packing ~−3, monomorphized
   width copies ≤ 3–5 (perf), dlmalloc → smaller allocator ~2.
