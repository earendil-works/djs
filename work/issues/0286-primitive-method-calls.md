# 0286: Builtin method calls on primitives, outside the loop's shared path

status: done
agent: pcall
branch: issue/0286-primitive-method-calls
depends-on: 0282
touches: crates/djs-core/src/interp/run (decline paths, fastpath, the call path for builtin callees; run_hot only with the hotcode.sh check), builtins/fast.rs, crates/djs-core/src/compiler only if a call-site form helps
guide: interpreter.md
decisions: D3550-D3559

## Goal
The ops table's remaining red rows are mostly tiny builtin calls on primitives: trim 1.18,
slice 1.15, padStart 1.27, charCodeAt loop 1.14, String(i) 1.23, toFixed 1.14 (wall, best
QuickJS). A method call on a string/number pays ~150 instructions of prototype lookup plus
~250 of generic leaf dispatch; QuickJS calls a C function directly. In-loop variants
(D3265, D3301) lost to V8's register allocation. Find a path outside run_hot's shared code
that makes `prim.method(args)` with a @fast/@rust builtin callee cheap end to end: e.g. the
method IC hit carrying the builtin's leaf id so the call skips frame construction and the
generic dispatch (arguments passed in place, no Shadow copy), a specialized leaf signature
for 0-2 argument string methods, or a compile-time call form for `x.m(...)`. Measure with
slope.py per op and the hotcode.sh check; report the design with numbers before a big build.

## Acceptance criteria
- [x] per-op instructions vs QuickJS 2026 before/after for the red rows; workloads; run_hot's shared path unchanged
- [x] the every-stop equivalence test covers the new path
- [x] size reported (~1 KB of the owner's allowance left, shared with 0285 and 0287); gate green; decisions recorded
## Log
- First design (prototype while builds and benchmarks were on hold for 0284): (1) the
  method from the slot for string and number receivers; (2) the whole call made from
  `get_method`'s out-of-line path (`field_slow`) for sites `get_method; plain pushes;
  call`. Measured when the hold was lifted: (2) is 400 instructions slower per call than
  leaving the loop (the trip through `field_slow` and `cached_get`, the decoding), and its
  `continue` in the arm put 32 instructions on every arm. Dropped; patch in
  work/scratch/0286/fused-prototype.patch.
- The yardstick: `slope.py --qjs` counts quickjs-wasi, against which main was already at
  0.73–1.00 on the red rows. The ops table's ratios are against Bellard's QuickJS built
  with -O2; `slope.py --qjs2` counts that one, and gives the table's ratios (trim 1.14,
  charCodeAt 1.16, String(i) 1.21, toFixed 1.19).
- Second design, landed: the call arm calls a `@fast` builtin's entry itself
  (`loop_leaf_call`, D3551), −55 per builtin call. D3265 and D3301 had lost to V8's
  register allocation with this; three things found with hotcode.sh made the shared path
  stay as it is (nothing of the arm's live across the call, no opcode after it, the flags
  read again).
- (1) alone cost every property read 6 instructions (V8 spilled three loop values at
  the head of the `get_field` arm; found with the new `v8arm.py --paths`, the summary's
  static path does not show it) and made the workloads slower. With the validity moved
  into the epoch (the first write to a builtin bumps it) and `field_slow` given the pc
  instead of opcode, key and cache index, the own-property hit is 66 instructions as on
  main and the short forms are one shorter (D3550).
- `fast_leaf_call` (the decline path, now only for calls at the stack's end and the
  tests' run without the loop) asks `loop_leaf_call`: one copy of the code, −784 B of
  wasm in `decline_fast`.
- Tests: `hot_loop_paths_are_invisible_at_every_stop` now stops finely through the whole
  run (it had only for the first 600 stops) at fuel 1, 2, 3, 5, 7, 11, with builtin calls
  that succeed, throw, convert an object, meet a replaced and restored builtin, and
  `Math.pow` (a leaf that uses the stack) at every alignment to the stack's end; removing
  the arm's test of the room makes it fail. Corpus case builtins/fast-calls.js (from
  qjs): receivers of nine kinds at one site, builtins replaced, shadowed, deleted and
  turned into getters with warm caches, throwing and converting arguments, deep frames.
- Checked: lib tests (631), e2e in four modes, difftest (whole corpus plain; builtins/,
  codemode/, language/, dquickts/ with suspension, GC stress and determinism; codemode/
  and builtins/fast-* lazy with plain dumps), test262 built-ins and language (39,251
  active, no new failures).
- Outside `touches:`: heap/frozen.rs (`set_prefix_written` bumps the prototype epoch,
  3 lines), vm/compat.rs (COMPAT_VERSION 13), vm/tests.rs, tests/scores/size.tsv,
  scripts/perf (slope.py `--qjs2`, qjs-built-run.mjs, v8arm.py `--paths`), TESTING.md.
- Guide: interpreter.md (the tier-0 loop: builtin calls; inline caches: a primitive's
  builtin method).

## Follow-ups
- What is left above QuickJS 2026 on these rows is the builtins' own work: number
  formatting (`toFixed` 1.13, `toString` 1.21, `String(x)` 1.15), `startsWith` 1.69
  (700 instructions more than QuickJS: the regexp check of its argument and the generic
  leaf), `indexOf` 1.07.
- Measured on the way, against QuickJS 2026 (worse than against quickjs-wasi): a getter
  `o.g` 1.82 and a class getter 1.71 (a frame inside the loop, 0276's follow-up),
  `o.missing` 1.70, a typed array's `length` 1.48, a closure call `f()` 1.18,
  `Map.get` 1.13.
- `String(x)` and other calls of globals pay the global lookup (`get_global`), which
  the known-builtins table does not cover for `String` itself.
- The published ops table and the guide's ratios that came from `--qjs` (D3530's table)
  are against quickjs-wasi; a refresh would use `--qjs2`.

## Result
Merged with this commit; D3550, D3551. Builtin calls run inside the tier-0 loop and a
string's or number's builtin method comes from the cache slot. Against Bellard's
QuickJS 2026 (instructions per operation, steady state): trim 1.14 → 1.04, slice 1.06 →
0.97, padStart 0.98 → 0.89, charCodeAt 1.16 → 1.02, String(i) 1.21 → 1.15, toFixed
1.19 → 1.13; also Math.floor 0.89 → 0.83, push 0.97 → 0.89, Map.get 1.20 → 1.13.
Simple loops −1..−1.7%, all sixteen workloads faster (−0.2..−6.1%). run_hot's shared
path unchanged (dispatch 14, tail 4). Shipped 431,032 → 431,435 B gz (+403), slim +387;
595 B of the owner's allowance remain.

## Shared context (all of 0256-0260)
The owner asked to push both memory and speed as far as they go (2026-11-21). Background,
fresh profiles and the ranked idea list: work/scratch/perfreview/ of the primary checkout
(BRIEF.md, IDEAS.md; prof.ts and cat.py profile the codemode workloads in wasm). Read
IDEAS.md's section for your issue first; its numbers are estimates, not measurements.

Five issues run in parallel (0256 per-VM/per-byte costs, 0257 idle memory, 0258 GC,
0259 builtin calls, 0260 byte kernels). Stay in your `touches:`; announced overlaps are in
each issue's Notes.

Measuring (D016: wasm32 under node is the yardstick):
- run time and memory per workload: build the shipped module in your worktree
  (`scripts/build-wasm.sh`, or `--keep-names` for profiles) and run
  `DJS_WASM=$PWD/target/wasm32-unknown-unknown/wasm/djs_wasm.wasm node js/bench/engines/run.ts --engines djs --reps 25`
  before and after, on main's build and yours, at load below ~4 (`uptime`; say the load).
  Memory claims: scripts/perf/mem-perturb.sh (D2048).
- instructions: scripts/perf/wmeasure.py / wmicro.py (TESTING.md "Instruction ratios").
- Report per workload: ms and MiB, main vs branch, and the module's gzip bytes.

Size: the shipped module is ~1.3 KB gzip under the D020 budget (ceilings, D3266). Report
the bytes of every change; find offsets where you can. If a measured win needs more room
than that, stop and tell the coordinator with the numbers: do not raise ceilings yourself.

Never weaken determinism (collection points a function of the program, D1710), suspension
anywhere, images, or the unsafe rules (AGENTS.md).

## Log

## Result
