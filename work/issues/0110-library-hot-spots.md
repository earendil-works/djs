# 0110: pi-like's remaining hot spots: string search, allocation, flatten, dense element access in natives

status: done
agent: perf
branch: issue/0110-library-hot-spots
depends-on: 0107
guide: - (tell the coordinator if the builtins or interpreter chapters need updates)
touches: crates/djs-core/src/heap/string.rs (search, flatten), crates/djs-core/src/heap/arena.rs (alloc_init) (heap is idle pending the 0101 decision; coordinate through the coordinator), crates/djs-bgen/src/emit_rust.rs + builtins/gen (fewer states per element, a fused dense HasProperty+Get), crates/djs-core/src/interp/run/

## Goal
0107's findings: pi-like hands under 0.1% of its run to the general interpreter; its time
is in the builtins library: string `index_of`, arena `alloc_init`, JSON, GC, flatten; and in
native builtins each forEach element still costs ~300 native instructions in the state
arms, with HasProperty and Get reading the same dense element twice.

## Acceptance criteria
- [x] A wasm profile of pi-like by function after 0107, and fixes for what dominates, for
      example: a SWAR/word-at-a-time `index_of` for one- and two-byte strings (wasm has no
      SIMD by default, D016/0077), a cheaper `alloc_init` fast path, fewer flattens
      (str_view_flat where read-only, D1610), a fused dense HasProperty+Get helper and
      fewer states per element in bgen's generated loops.
- [x] wasm instruction counts (primary) before/after per change; images, fuel, stress and
      determinism checks unchanged; the size per change.

## Log
- Profiles of pi-like after 0107, by function: the wasm cpu profile (node --cpu-prof,
  callers from the profile tree) and native samply plus lldb traces of micro cases
  (work/scratch/0110/mi.py).
  - `alloc_init` (4%) is almost all the GC's copy (`gc::forward`). With for_each_slot,
    forward and trace, the GC is ~11% of pi-like. That is heap's, pending 0101; not
    touched.
  - `index_of`: 891k calls, an average haystack of 1 KB, but short scans.
    - The cost was per call: the needle copy, flatten, and 4–5 kind checks in
      str_view/str_view_flat/flatten/prefix_view (~300 instructions);
    - and a byte loop plus `memcmp` for the search.
  - `split`: each piece repeated the `index_of` setup, and substrings went through a
    stack buffer.
  - Array `join`: a Vec of units per element, then narrowed again.
  - JSON parse/stringify (~9%) is infra's (0090) and already narrow-buffered; not
    touched.
- Landed, with wasm Minstr of pi-like (min of 3, spread ≤0.3%):

  | change | pi-like before → after | other workloads |
  | --- | --- | --- |
  | word-at-a-time `index_of`, in-place needle, one-pass `split`, flat fast path in heap/string.rs | 10,039 → 9,405 | strings −9.6%; +1.3 KB |
  | `alloc_latin1_pieces` | 9,392 → 9,315 | +263 B |
  | fused dense HasProperty + Get (D2090) | → 9,134 | per element −10 to −14% wasm, −12 to −16% native; +485 B |
  | Array join from pieces | 9,129 → 8,781 | +512 B |
  | one kind read per piece | 8,793 → 8,585 | +61 B |

- Tried and dropped: tail duplication (fewer states per element), +13 KB for ~0%
  (find −6%, sort −1%). State transitions in the wasm loop are cheap.
- Outside the listed areas: builtins/string.rs (index_of, find_byte/find_latin1, join),
  builtins/fast.rs (split) and builtins/dense.rs (join). These are the hot spots the
  issue names, and their callers. heap/string.rs was cleared by the coordinator;
  arena.rs was not needed.
- Tests:
  - `word_at_a_time_search_matches_the_naive_one`, random against naive;
  - the golden `NativeHasGet`;
  - unit, e2e (3 modes), difftest (both backends, stress, determinism) and test262
    active: all unchanged.
- Final vs main 3432a8c2 (wasm, min of 3; full table in work/scratch/0110/wm-d6.txt):
  - pi-like 10,029 → 8,590 (−14.3%); strings −11.0%; higher-order −9.3%;
    foreach −8.3%; strbuild −2.8%; json-date −2.5%;
  - the rest −1.3% to +0.9%, all within their spreads. pushpop's +2.4% re-ran at
    −0.7% (n=5), and native is identical.
  - Size: 1,686,949 → 1,689,933 (+3.0 KB).
- Guide: no builtins chapter yet; nothing affected.

## Follow-ups
- GC is now the largest single cost of pi-like (~11% wasm: copy, slot walk, forward).
  It is for heap, after the 0101 mark-compact decision.
- JSON parse/stringify (~9% of pi-like) for infra: parse's `next`/`write_string` and
  stringify's `quote`. Output goes through a Rust Vec and is copied again into the arena
  (`alloc_latin1`); writing in place would save one copy.
- `flat_fast` stays out of line in wasm (0.9%); the heap's view helpers are called from
  everywhere.

## Result
pi-like −14.3% in wasm (10,029 → 8,590 Minstr), strings −11%, higher-order −9%,
foreach −8%; nothing slower; +3.0 KB.
- Word-at-a-time one-byte `index_of` with the needle read in place, and one-pass
  `split`.
- A flat fast path for string views (heap/string.rs).
- In-arena joins and substrings (`alloc_latin1_pieces`), and Array `join` from pieces.
- A fused dense HasProperty + Get in bgen's natives (D2090).
- Fewer states per element was measured and dropped (+13 KB for ~0%).
- Next: the GC (heap) and JSON (infra).
