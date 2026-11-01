# 0122: wasm size toward the D020 budget (gzip <= 1.5x quickjs-wasi)

status: done
agent: perf
branch: issue/0122-size-budget
depends-on: 0098, 0116
guide: docs/guide/goals.md and any page quoting sizes (generated numbers regenerate)
touches: crates/djs-core/ (broadly: generic code, tables, builtins), crates/djs-bgen/ (generated code size), crates/djs-wasm/, scripts/build-wasm.sh, Cargo profile settings

## Goal
The owner accepted D020: the shipped module's gzip size must stay within 1.5x
quickjs-wasi's. Today ~528 KB vs ~287 KB (~1.8x); the target is <= ~430 KB gzip, ~100 KB
to cut. Earlier size work: 0098 (base image + shapes), the bloat agent's
analysis (twiggy/attribution scripts), wasm-opt rejected (hurts gzip).

## Acceptance criteria
- [x] An up-to-date size attribution (code by crate/module/function family, data: base
      image, tables, strings) and a ranked list of candidate cuts with estimated gzip wins
      and instruction costs.
- [x] Land the cuts that pay, with no workload slower than +1% in wasm instructions and
      pi-like unchanged or better, unless a trade is recorded as a decision. Candidates to
      examine: monomorphization in generic code, bgen-generated code (fast paths that
      don't pay), panics/formatting machinery, duplicate Unicode/regex tables, base image
      content (lazy-materialized builtins?), opt-level per crate/function.
- [x] The size track enforces the budget (gate fails when gzip > 1.5x the recorded
      quickjs-wasi size, or a ratchet toward it), documented in TESTING.md.
- [x] Report where we end up; if 1.5x isn't reachable without real cost, say what it
      would take.

## Log
### Attribution (main 4b30eea2; scripts/size/, README there)
- Shipped module: 1,480 KB raw, **529.9 KB gzip**; quickjs-wasi 3.6.2: 286.7 KB (1.85x).
  Target 1.5x: <= 430.0 KB.
- Code: 910 KB raw, 344.2 KB gzip.
  - djs_core 86% of the raw code; alloc 44K raw, core 30K, libm 18.5K, std 11.8K,
    dlmalloc 7.3K.
  - By path: interp::run 212K (call_intrinsic 37.8K, step_once 35.5K, run_hot 16K,
    json_regexp 14.7K), compiler::parser 145K, heap 80K, natives 49K, builtins::fast
    25K, json::parse 24K, regexp::parser 21K.
  - Generic families (families.py) are a long tail: from_iter 10.6K over 58
    instances, LocalKey::with 9.8K (the export bodies), extend_trusted 8.3K, fmt 6.6K.
- Data: 566 KB raw, 182.6 KB gzip.
  - Base image: 385 KB raw, 105.3 KB gzip (image-kinds.py, image-size example):
    bytecode 99K raw, strings 80K, FuncTemplates 49K, function objects 39K, ...
  - The other 180 KB raw (77.2 KB gzip): Unicode about 43 KB gzip (\p sets 19.4,
    normalization 15, case/ID 9), messages and strings 15 KB gzip, panic paths 4.4K
    raw, the rest tables.

### Ranked cuts (gzip win; cost)

| # | cut | gzip | cost | status |
| --- | --- | ---: | --- | --- |
| 1 | base image ordered by kind, strings by content | −9.3 KB | none | landed (D2251) |
| 2 | base image coded (djs-lzrc), decoded once per module | −13.3 KB | +5 ms per module, +0.15 ms per instance, +64 KB memory per instance | landed (D2252) |
| 3 | prelude compiled from source at startup instead of the image | about −35 KB (with 2: about −20 more) | ~100M native instructions (15–25 ms wasm) per module; realm code +16 KB gz | not done: trade for the owner |
| 4 | prune natives (49 KB raw, about 17 KB gz) | about −5 KB | 1.3–2.5x on Set/Map ctors, forEach, toSorted, iterator next | rejected |
| 5 | code long tail (fmt/Debug impls, panic paths, generic glue, `--remap-path-prefix`) | about −3 to −6 KB | none, but many small edits | partly (decoder size) |
| 6 | opt-level "z" | −35 KB | +30–70% instructions everywhere | rejected |
| 6b | opt-level "z" for cold code only (compiler, regexp compiler) | about −9 KB | crate split, compile speed | not done |
| 7 | Unicode \p and normalization tables out of the module (host-loaded) | about −34 KB | architecture; owner | with the owner |
| 7b | the same tables coded with djs-lzrc, decoded at first use | about −5 KB | lazy decode per use or cache | follow-up |

### Landed
1. **D2251:** `Heap::order_for_image` (heap/compact.rs, new; no change to gc.rs,
   compactor.rs or `canonical_hash`) and `Vm::prepare_base_image` (vm/mod.rs). The
   recipe was copied in djs-prelude's build.rs, its test and djs-wasm's
   `vm_new_compiled`; all three use it now.
   - Image gzip 105.3 → 96.2 KB; module 529.9 → 520.6 KB.
   - Snapshot containers (snapshot-research/wasm.ts) are unchanged; the empty sandbox's
     bytes() went from 33 to 15 KB.
   - The wasm-compiled realm equals the base (js base.test), native vs wasm images are
     identical, and determinism holds.
   - Heap's 0120 constraint (OLD_END = image length): whoever merges second adds
     `set_old_region(len, 0)` after the reorder (`verify` catches it).
2. **D2252:** crates/djs-lzrc, our own LZ + range coder.
   - 79.0 KB coded vs 96.2 KB gzip. Tests: round trips, determinism, header and limit
     checks, a corruption fuzz (flips, truncation, garbage).
   - djs-prelude features `raw`/`packed`; djs-wasm embeds the packed image.
   - Host: `provideBase`/`rememberBase` in js/src/snapshot.ts, used by the Sandbox and
     pi's engine. New exports `djs_base_inject_ptr`/`djs_base_inject`; ABI version
     unchanged.
   - Tests: djs-wasm native (decode, inject, refuse a wrong length or damaged copy) and
     js base.test (the same in wasm, identical VMs).
   - Found and fixed: `baseReference` read `memory.buffer` before `djs_base_ptr()`, which
     can grow memory now.
   - Module 520.6 → 507.3 KB (the decoder is +5.8 KB raw after moving `bit` out of line).
3. **The size gate (TESTING.md "Size budget"):**
   - gate.sh builds the shipped module in target/size beside the other steps, and the
     last step runs `scripts/size/budget.py` against tests/scores/size.tsv;
   - quickjs-wasi 286,655 is recorded there, and `ceiling_gzip` is a ratchet, now
     508,383;
   - `--update` lowers the ceiling, and raising it is a recorded decision. Once the
     module is within 1.5x, the budget itself is the limit.
- Speed (wasm, min of 3, vs main; work/scratch/0122/wm-o5.txt): every workload within
  −0.4%…+0.2% of main except strbuild at +0.9%. pi-like −0.2%. Cycles mostly lower
  (cache locality of the grouped image), but cycles are not load independent.
- Guide: host.md (the coded base, said in its own words); docs/host.md has the exports
  and the startup numbers. goals.md quotes no sizes; status numbers regenerate.

### Where we end up, and what 1.5x would take
**507.3 KB gzip = 1.77x** quickjs-wasi (from 1.85x): −22.6 KB. The rest of the way to
430 KB (−77 KB) can't be done without real costs:
- 3, prelude from source: about −20 KB more on top of 2, at 15–25 ms per process.
- 7, Unicode tables out of the module: about −34 KB.
- 6b, cold code at opt-level z: about −9 KB, with a crate split.
- 5, the code long tail: about −5 KB.

Together that is about −70 KB, which lands around 440 KB (1.53x). Beyond that it would
take cutting features or more drastic size-for-speed trades in the interpreter itself.

## Follow-ups
- For the owner, with the coordinator: Unicode tables out of the module (7), and
  prelude from source (3).
- Unicode tables coded with djs-lzrc, decoded at first use (7b, about −5 KB gz). It
  needs a decoded-table cache per process (djs-core is no_std, so a cache in the Heap or
  passed in by djs-wasm).
- `--remap-path-prefix` for the wasm build: the module embeds absolute source paths
  (`/Users/...`; 4.4 KB raw of panic locations). That is hygiene, and it would change
  the build hash inputs.
- Shrink the decoder further: one flat probability array instead of the Model struct
  (about −0.8 KB raw).
- The gate's size step adds a release LTO build (a few minutes, mostly one core) beside
  the other steps: watch the gate's wall time (target/gate-steps.tsv).

## Result
Shipped module 529.9 → **507.3 KB gzip** (1.85x → 1.77x quickjs-wasi), with workload
speed unchanged:
- the base image is ordered by kind (D2251);
- it is shipped coded with djs-lzrc and decoded once per module by the host (D2252);
- the gate enforces the budget with a ratchet (TESTING.md, scripts/size/budget.py,
  tests/scores/size.tsv).

1.5x needs owner-level trades (Log, "what 1.5x would take"): the best cheap path lands
near 1.53x. Attribution tools are in scripts/size/.
