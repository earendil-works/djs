# 0127: Size: compact image (internal names, bytecode density), containers, fast paths

status: done
agent: perf
branch: issue/0127-compact-image-code
depends-on: 0122, 0123
guide: docs/guide/bytecode.md (if the encoding changes), docs/guide/snapshots.md / builtins.md as affected
touches: crates/djs-core/src/compiler/ (emit, peephole), crates/djs-core/src/vm/ (bytecode decoding, base image), crates/djs-prelude/, crates/djs-bgen/, collections/containers in djs-core; bloat (0126) owns Unicode tables and panic/fmt

## Goal
Bloat's report (docs/size-vs-quickjs.md) puts ~1.5x within reach without design changes,
via cuts 0122 didn't take:
1. Internal names in the image: 1,494 of 2,360 strings are prelude function / local
   names (52 KB raw, ~15 KB gzip). Keep only observable names (Function.prototype.name,
   stack frames as tested by test262 / corpus / pi suite); locals need no names in the
   shipped image.
2. Bytecode density for the prelude (short forms, narrower operands, a peephole pass,
   IC slots only where they pay): ~−9 KB gzip, and smaller user bytecode too; measure the
   interpreter cost (must not regress pi-like).
3. Out-of-line container growth (Vec push paths inlined everywhere): 10–15 KB.
4. Generated Rust fast paths kept only where benchmarks pay: 10–20 KB (prune with
   numbers, per D2020).

## Acceptance criteria
- [x] Each cut measured (gzip via scripts/size, wasm instructions on all workloads); no
      workload slower than +1%, pi-like not slower, unless a trade is recorded.
- [x] The size ratchet (tests/scores/size.tsv) lowered with each landing.
- [x] Report the new ratio and what remains to 1.5x.

## Log
### Landed (wasm gzip of the shipped module; tests/scores/size.tsv lowered each time)

| cut | decision | gzip | speed |
| --- | --- | ---: | --- |
| internal names out of the base image | D2253 | −22.4 KB | unchanged |
| 13 `@native` builtins back to bytecode | D2254 | −3.3 KB | workloads unchanged (native counts) |
| `-inline-threshold=40` plus hot helpers `#[inline]` | D2255 | −2.6 KB | all 22 workloads at or below main |
| URI functions and `substr` without @fast | D2256 | −0.2 KB | those calls 1.3–3.3x slower (rare) |

- Main after the first landing (34eb668e): 472.0 KB. This branch: **469.2 KB gzip =
  1.637x** quickjs-wasi (286.7 KB). Ceiling 470,209.
- D2253 is vm/image.rs and `Heap::forget_string_atoms` (heap/atom.rs).
- D2255 adds `#[inline]` on three small heap/object.rs accessors (heap's file, attributes
  only) and puts the flag in bloat's wasm-build-env.sh (told bloat first).
- scripts/perf/setup.sh builds the wasm through with-wasm-env.sh, so its bytes match the
  shipped ones.

### Measured and not done
- **Bytecode density (item 2):** every prelude immediate narrowed to 8 bits where it fits
  (99 → 70 KB raw) is only −1.8 KB in the coded image, and 8-bit jumps alone −0.35 KB.
  The image codec (D2252) already removes the zero high bytes. Not worth an encoding
  change across compiler, step, run_hot and disasm.
- **One JSON input path:** the UTF-16 instance is 3.6 KB raw (~1.5 KB gz). It needs a
  WTF-8 mode for lone surrogates; skipped.
- **Containers (item 3):** 443 inlined push/`grow_one` sites, 1,020 inlined Vec drops,
  155 reserve sites: ~15–20 KB raw, ~6 KB gz in all. Only per-function rewrites would
  cut it (about −3 to −5 KB gz). Parked as a follow-up.
- **pi suite:** 215/226, the six process-lifetime tests a known artifact of the fast
  search. Cancellation was checked with a busy loop on node and bun; it is documented in
  docs/host.md.

### Where we are, and what 1.5x takes
**469.2 KB = 1.637x**; 1.5x is 430.0 KB, −39 KB away.
- In my lane: the container rewrite, about −3 to −5 KB.
- bloat (0126): the Unicode table encoding, ~−13 KB per its report. That would land at
  ~452 KB, ≈1.58x.
- The rest takes owner trades (measured in 0122):
  1. **Unicode \p and normalization tables out of the module** (host-loaded on first use):
     about −34 KB, less whatever 0126's encoding takes first.
  2. **Prelude compiled from source per process** instead of the shipped image: the
     image is now 57 KB coded, against about 53 KB of minified source plus 16 KB of
     realm code. With names gone and the codec in, this no longer pays, so drop it as a
     candidate. Cost was 15–25 ms of wasm per process.
  3. **opt-level z for cold code** (compiler, regexp compiler) via a crate split: about
     −9 KB, slower compiles. Whole-module z is −35 KB but +30–70% instructions.

So after 0126: 1 alone (≈ −20 KB beyond its RLE) gets close to 1.5x; 1 and 3 together
reach it.

## Follow-ups
- Containers: out-of-line push/extend helpers and fewer temporary Vecs in the size-hot
  functions (compiler emit, call_intrinsic, json); each with a speed check. About −3 to
  −5 KB gz.
- One JSON input path (UTF-16 through a WTF-8 mode of the UTF-8 lexer): about −1.5 KB gz.

## Result
The shipped module is at 469.2 KB gzip, **1.637x** quickjs-wasi (from 1.777x at the
start of 0127, 498.6 → 469.2 KB on main's tree). The cuts:
- the base image keeps no internal names (D2253);
- 13 natives and two rare @fast paths are gone (D2254, D2256);
- the module inlines less, with hot helpers marked (D2255).

Workloads are equal or faster. Bytecode density was measured and dropped. Containers and
one-JSON-path are follow-ups. Reaching 1.5x needs bloat's Unicode encoding plus an owner
trade (Log).
