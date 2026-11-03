# 0161: Heap access that is safe and as cheap as unsafe; Vec/container hygiene

status: in-progress
agent: audit
branch: issue/0161-heap-access
depends-on: 0156
guide: docs/guide/arena.md (how heap access is checked)
touches: crates/djs-core/src/heap/arena.rs and the accessor layer, call sites as needed, Cargo features, docs/decisions.md; Vec hygiene: broad but mechanical edits in djs-core; bloat (0151 L3) is in heap/ (templates, function objects) and compiler/: coordinate via the coordinator, land in small merges

## Owner
1. "An optional build that uses unsafe for djs-core on the heap. We should add some markers
   and the code should still work without unsafe as well, so we need to dual-track this.
   Ideally we could guarantee that the safe code has as little overhead/code size as the
   unsafe code. Can creativity get us there?"
2. "Still impressed that Rust adds so much overhead over C. In particular Vec seems crazy.
   Can we do better code hygiene which avoids some of that?"

## Part 1: dual-track heap access
docs/architecture-bloat-audit.md: unchecked heap accessors save 11.8 KB gz and are faster;
checked accessors are 4.5% of code. Design one accessor layer with:
- an `unsafe-heap` (or similar) feature, off by default, using unchecked access with
  debug assertions, every unsafe block in one module with a safety comment, a decision
  recording why;
- **creative safe variants** that approach the unsafe cost with no `unsafe`: e.g. a
  power-of-two arena capacity and index masking (`idx & mask`) so reads can't go out of
  bounds and need no panic path; checking once per cell (a cell-sized slice obtained once,
  then fixed-offset field reads LLVM can prove in range); `get(..).unwrap_or(0)`-style
  non-panicking reads where a wrong value is harmless because the verifier/invariants
  guarantee validity; const-sized field accessors; reducing the number of distinct
  accessor instantiations. Measure each against the unsafe build (code size, wasm
  instructions).
- The goal: the safe default within a small margin of the unsafe build, so the unsafe
  feature is optional, not necessary. Both run in the gate (the unsafe one at least on
  e2e + difftest + a fuzzed heap verifier).

## Part 2: Vec/container hygiene
Vec glue is ~10.5% of code (audit): 443 inlined grow_one, 1,020 inlined deallocate, 155
reserve sites. Find patterns and fix them systematically: out-of-line push/extend helpers
(one non-generic grow path per element size), fewer temporary Vecs (reuse VM-owned scratch
buffers), SmallVec-like fixed buffers on hot small cases, avoiding Vec<Vec<..>>, Result
plumbing that forces drops on error paths, etc. A lint or script that reports new inlined
grow/drop sites so it doesn't regress. Measure bytes per change; no workload slower than +1%.

## Log
- Part 1 kernels (work/scratch/0161/kern, wasm32 opt-level s): one field read is 55 B
  checked today (two compares: `get(off..)` then `first_chunk`), 16 B unchecked, 42 B with
  one u64 compare; three fields of a cell 117/36/103 B, and 62 B with one check per cell
  (`first_chunk::<24>`). Masking against a length-derived mask (68–105 B) and clamping
  (70 B) don't let LLVM drop the check; zeros-on-failure is 53 B.
- Module variants (main 7e1ad6c5+, gz): main 455,502; u64 compare 450,265 (−5.2 KB);
  unchecked 443,089 (−12.4 KB); u64 + a 256-byte window per cell base (merged checks):
  450,528 (the inliner spends the merges); u64 + `wasm32::unreachable` fail path 452,488;
  `#[inline]` accessors 450,484. wmeasure (instructions, n=3) vs main: u64 −0 to −4.2%;
  unchecked −8 to −15%, but calls +1.9%, tight +3.3%.
- Landed (part 1): heap/layout/access.rs with both implementations; `unsafe-heap` feature
  (djs-core, djs-wasm), excluded from SOURCE_HASH so the host-built base loads; lib.rs
  forbids unsafe without it; tests/verify_fuzz.rs (3,000 corrupted images in debug, ~30%
  accepted, never a panic, also with the feature); a gate step (fuzz + e2e with the
  feature, difftest in release with suspension and GC stress: ~1 min); D2520; guide
  arena.md updated.

## Follow-ups

## Result
