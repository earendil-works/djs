# 0158: Instance memory: smaller reservation, arena at the end, smaller stack, base outside memory

status: done
agent: heap
branch: issue/0158-instance-memory
depends-on: 0128, 0139, 0151 (L4 landed)
guide: docs/guide/arena.md, gc.md, host.md as affected
touches: crates/djs-wasm/ (allocator.rs, base decode/inject, stack size in the build), crates/djs-core/src/heap/ (initial reservation, capacity), scripts/build-wasm.sh / wasm-build-env.sh (stack size flag), js/src/snapshot.ts (base handling, if the compressed base leaves memory); bloat (0151 L3) works in heap/ templates and objects: coordinate via the coordinator

## Goal
docs/architecture-bloat-audit.md step 5: a fresh Sandbox is 19 pages (1.19 MiB, now 18
after L4): 4 stack, 3 static data, 11 arena (base + 409 KiB reserved and never touched,
because the reservation is 2x the base), 1 Rust heap. Target ~11 pages:
- reserve less up front, **with the page allocator keeping the arena at the end of
  memory** so it can still grow in place (the audit's prototype at 12 pages made growing
  heaps end 1–4 pages higher because the arena got boxed in);
- a 128 KiB wasm stack if the deep-recursion tests and D1941's parser depth limits still
  pass (or make the stack size a build option with 128 KiB for the shipped module);
- keep the compressed base out of wasm memory (the host holds it; D2252/D2060 paths).
Measure with mem-perturb on all workloads, pi warm/cold, an empty Sandbox, and
scripts/size/sandbox-memory.ts; no workload's peak worse beyond noise.

## Log
- Stack, measured on the debug wasm (the gate's nesting test), scratch probes in
  work/scratch/0158/ (nesting-probe.ts, runtime-probe.ts):
  - parser nesting, deepest level without a trap at 128 KiB vs the accepted limit at
    MAX_DEPTH 1000: parens 248/332, arrays 266/333, objects 248/332, calls 398/498, funcs
    164/199, blocks 723/995, trys 721/995, template 248/332, classes 81/110, methods
    101/124; arrows, ifs, fors, switches, whiles, patterns, regexp groups fit. Worst
    ratio 0.72 (trys, blocks): MAX_DEPTH ~650 leaves >= ~10% margin everywhere.
  - runtime recursion (JS calls, JSON both ways, join, proxy/bound/prototype chains,
    getters, a 300k-deep list through the GC, regexps, sort, async recursion, toString,
    flat): identical results at 256 and 128 KiB.

- Stack: MAX_DEPTH 1000 -> 650 (all constructs reach the limit's SyntaxError at 128 KiB
  with 10–12% spare, debug and release identical); test262 language/ and the difftest
  ratchet unchanged. The release module also needed the regexp parser's limit lowered
  255 -> 201 (243 nested capture groups / 236 lookaheads trapped at 128 KiB in release
  only; test262 S15.10.2.8_A3_T15 needs 200 groups). D2540.
- Allocator: a page log (scratch build, work/scratch/0158/allocator-with-log.rs) showed
  every growing workload moving its arena once at 10 pages, blocked by dlmalloc's first
  segment or the 2-page segment a collection's side tables need. Variants tried and
  their numbers are in D2541 (gaps, forced early move, finer growth, dlmalloc release /
  trim, prime sizes, no medium runs). Taken: dlmalloc primed with 2 pages, 16–256 KiB
  blocks as page runs when dlmalloc would have to grow, room behind the top block for a
  growing buffer, lean start capacity (image + 1 page; the size hint keeps the old
  capacity so pi's cold start stays at 24 pages).
- Coded base: custom section `djs_base`; with `#[used]` or `include_bytes!` the bytes
  stayed in the data segment too, a `pub` static array without `#[used]` leaves them out.
  New export `djs_base_coded_ptr`, ABI 2, `provideCodedBase` in js/src/snapshot.ts; raw
  hosts updated (abidrive.mjs, js/bench/wasm.bench.ts, sandbox.bench.ts).
- Coordinator accepted json-date 4.3 -> 5.2 MiB as a trade (D2541).
- Guide pages updated: arena.md (starts small, kept at the end), gc.md (what blocks the
  arena, trade-off), host.md (coded base as a custom section, 11 pages); docs/host.md
  export table, docs/compiler.md 2.4.
- Shared files touched with the coordinator's OK: djs-core vm/mod.rs (two call sites,
  `Heap::start_capacity`), heap/mod.rs, compiler/parser.rs, compiler/tests.rs,
  regexp/parser.rs (MAX_DEPTH only).

## Follow-ups
- JSON.stringify holds its whole result in a Rust `Vec` while the arena grows to copy it
  in (json-date +0.9 MiB, D2541): write large results straight into a heap string.
- `djs-wasm` test `base_equals_a_compiled_realm` (`--features compiled-prelude`, not in
  the gate) fails on main too: the base's bytes differ from a realm compiled natively in
  the test process.
- A stack overflow in wasm (deeper than the parser limits allow, e.g. a future recursive
  path) surfaces as a raw `RuntimeError: memory access out of bounds`, not a
  SandboxError "trap": hosts could map it.
- The page log used for this issue could become a cargo feature of djs-wasm for future
  memory work (kept out to stay small).

## Result
- Fresh Sandbox 18 -> 11 wasm pages (704 KiB): 128 KiB stack (D2540), dlmalloc's 2
  pages below a base-plus-one-page arena, coded base out of linear memory (D2541).
- Peak memory (mem-perturb n=24, mean MiB, main -> branch): small runs 1.10 -> 0.70,
  pi-like 2.40 -> 1.60, push-pop 2.00 -> 1.19, foreach/forof 4.70 -> 3.80, json-indent
  5.40 -> 4.70, json-replacer 5.34 -> 4.66, json 6.20 -> 5.80, jsonh 5.80 -> 5.40,
  arrays 13.7 -> 13.1, higher-order 14.9 -> 14.3, strbuild 15.8 -> 15.3, strings 12.2 ->
  12.0, props/json-text 1.3 -> 1.2; json-date 4.3 -> 5.2 (accepted trade).
- pi bench pages per instance: warm start 23 -> 14, warm tool-heavy 25 -> 24, cold 26 ->
  24. Resident per sandbox ~650 -> ~610 KiB.
- Module +985 B gzip (451,262; ceiling 455,600). Instructions within ±1% except
  json-text +4.4%, json-date +3.9%, indexof +2.6% (spreads 2–4%).
- Hosts must hand over the `djs_base` section (`provideCodedBase`, or `provideBase` /
  `baseReference` which call it); ABI version 2.
- Nesting limits: ~215 nested parentheses/arrays/objects, ~70 classes, 645 blocks; 200
  nested regexp groups.
