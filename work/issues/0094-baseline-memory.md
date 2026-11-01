# 0094: Baseline wasm memory of a VM (3.9 MiB vs quickjs-wasi's 1.6 MiB)

status: done
agent: num
branch: issue/0094-baseline-memory
depends-on: 0087
touches: crates/djs-wasm/ (stack size, memory setup), crates/djs-core/src/heap/ (initial arena sizing), crates/djs-prelude/, crates/djs-core/src/compiler/ (only if the nesting limits must change), js/src/sandbox.ts, docs/host.md

## Goal
After 0087, a pi `return 1` run still peaks at ~3.9 MiB of wasm memory against 1.6 MiB
for quickjs-wasi. Find out exactly where the baseline goes and cut it. Candidates: the
1 MiB default Rust stack (vs the compiler's nesting limits; `deep_search.py`), the base
image in the data segment and its copy in the arena (both live?), the initial arena
reserve, dlmalloc's growth granularity, the Sandbox's staging buffers, and node's own
minimum.

## Acceptance criteria
- [x] An accounting of the 3.9 MiB (data segment, stack, heap arena, allocator slack,
      buffers), measured.
- [x] Reductions with numbers: for example a smaller stack if the compiler's recursion
      fits (or a deeper-but-iterative parse where needed), not keeping two copies of the
      base image live, initial arena sized to the base image plus a page.
- [x] No behavior change (deep nesting tests still give the same SyntaxError limits or
      documented new ones), the pi bench and memory numbers before/after.

## Log
- Accounting with a new djs-wasm `alloc-stats` feature (`djs_alloc_stats`) and
  work/scratch/0094/account.ts (memory and Rust heap per export call). Warm pi `return 1`
  was: 1 MiB stack + 561 KiB data + 546 KiB staging (moved and left as a hole when the arena
  grew) + 1093 KiB arena + verify transient = 3.5 MiB (D1940).
- Stack: per-level wasm stack measured for 20 constructs (scratch deep.py / perlevel.py);
  nested classes trapped even at 1 MiB. Boxed thunk/init FuncStates, out-of-line
  declaration/for-init parsing, depth weights (class 7, for 1, pattern 1); 256 KiB stack;
  js/test/nesting.test.ts (D1941).
- Arena: reserve the target at restore, base copy into a target-sized buffer,
  `djs_image_staging` export + JS hosts, exact verify list (D1942).
- Base image compression (LZ, 398 → 159 KB) built and rejected: ~0.5 ms / 7.5 M wasm
  instructions per vm_new (D1940).
- Checks: djs-core/prelude/wasm tests, js tests (48), native-vs-wasm, test262 class/for/
  destructuring subsets (ratchet OK).

## Follow-ups
- A cheaper base-image codec (~240 KB of data segment per instance) if one decodes in
  ~0.1 ms in wasm.
- The cold start's peak (3.8 MiB) is the post-prelude collection's from+to space; an
  in-place (sliding) compaction for that one collection would cut it.
- The verify pass still allocates ~80 KiB at restore; a bitmap of cell starts would make it
  ~9 KiB.

## Result
pi `return 1` reusing the prelude image: 3.50 → 2.13 MiB of wasm memory (quickjs-wasi
1.56). The stack is 256 KiB (D1941; parser frames shrunk, class/for/pattern depth weights,
wasm nesting test), restored arenas reserve their GC target once instead of moving
(D1942, `djs_image_staging`), and the accounting is in D1940. Cold starts stay ~3.8 MiB
(the post-prelude collection's two semispaces).
