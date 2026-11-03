# 0161: Heap access that is safe and as cheap as unsafe; Vec/container hygiene

status: done
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
- Part 2 survey (names build of 0e59af8f): calls into Vec glue at 1,045 drop sites
  (`RawVecInner::deallocate`, the Vec spilled to the stack for each), 369 inlined
  `grow_one` (push), 194 `with_capacity`, 184 `reserve`, 263 allocation-error paths; DWARF
  attributes 101 KB raw to vec/raw_vec, spread over ~2,000 functions (top: emit_function
  2.7 KB, step_once 2.4, json_regexp_intrinsic 2.1). Element types of the drops: u32 300,
  u8/String 255, 8-byte pairs 153, u16 134, Value 80.
- Experiment: every push out of line (patch pushcold.py): −4.8 KB gz, json-text +1.7%,
  pushpop +1.3%, json +1.0%. Landed selectively (D2521): 343 sites, −3.2 KB gz, workloads
  within ±1.3% (mostly lower), compile within noise. scripts/size/vec-sites.py + baseline.
- Merged D2521 (f27b07b9). The debug wasm build's nesting test caught `push_out` of an
  832-byte FuncState (parse_arrow's frame 64 → 864 bytes): push_out now asserts values of
  at most 16 words at compile time.
- `Vec::with_capacity` out of line (D2522): 60 sites, −1.3 KB gz, workloads −2.1..+0.5%.
- Final merge: size ceilings lowered to measured + ~1 KB (full 441,661 → 442,685; slim
  376,465 → 377,489), vec-sites baseline refreshed (grow 174, drop 1,040, with_capacity
  148, reserve 167, alloc_error 214). Guide: docs/guide/arena.md updated (part 1).

## Follow-ups
- push_out / vec_with_capacity in the files 0151 L3 is editing, after L3 lands (redo with
  scripts/size/vec-sites.py --check; est. ~1 KB gz): interp/run.rs (Vec pushes among its
  110 `.push(`, most are Exec::push), vm/mod.rs 7, vm/image.rs 2, heap/walk.rs 2,
  heap/cells.rs, heap/atom.rs, heap/compact.rs and bytecode/template.rs 1 each (+1
  with_capacity in template.rs). Leave hot.rs, json/, builtins/dense.rs,
  bytecode/code.rs and compiler/resolve.rs inline unless a measurement says otherwise.
- Drop sites: 1,040 inlined Vec drops, each spilling the Vec to the shadow stack for
  `RawVecInner::deallocate`. Per-function work: VM-owned scratch buffers instead of
  temporaries in the top functions (resolve::emit_function 70 glue sites,
  Exec::json_regexp_intrinsic 49, Exec::call_intrinsic 30, Parser::finish_unit 26,
  CodeBuf::finish 23), and fewer String temporaries in error paths. Est. a few KB gz.
- The rest of the safe/unsafe gap (~7 KB gz) is one compare per access at ~1,650 sites;
  per-cell views (`&[u8; N]` taken once, fields at constant offsets) at multi-field
  call sites are the remaining lever (D2520).

## Result
- **Part 1 (D2520).** Every arena access goes through heap/layout/access.rs. Safe default:
  one u64 compare (−5.2 KB gz, wasm instructions −0 to −4%, no unsafe). `unsafe-heap`
  feature (djs-core, djs-wasm; off, not shipped): unchecked with debug assertions, the
  only `unsafe` in djs-core (lib.rs forbids it otherwise), sound only for images the
  engine made; −12.4 KB gz vs the old checks, −8 to −15% instructions but +2–3% on
  calls/tight. Not in SOURCE_HASH. Gate: verifier fuzz test (tests/verify_fuzz.rs) + e2e
  with the feature, difftest corpus in release with suspension and GC stress. Masking,
  non-panicking reads, a shared per-cell window, `wasm32::unreachable` and `#[inline]`
  were measured and rejected (none beat the u64 compare).
- **Part 2.** `PushOut::push_out` at 343 non-hot sites (D2521, −3.2 KB gz; values ≤ 16 words,
  a compile-time check) and `buf::vec_with_capacity` at 60 sites (D2522, −1.3 KB gz);
  workloads within noise. scripts/size/vec-sites.py reports inlined push/drop/
  with_capacity/reserve/alloc-error sites per function against scripts/size/vec-sites.txt
  (`--check` fails on >2% growth of a kind).
- Total about −9.7 KB gz; size ceilings lowered (full 442,685, slim 377,489).
