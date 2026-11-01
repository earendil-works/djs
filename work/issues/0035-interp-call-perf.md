# 0035: Interpreter call-path performance

status: in-progress
agent: heap
branch: issue/0035-interp-call-perf
depends-on: 0030 (parts 1-3 merged)
touches: crates/djs-core/src/interp/, crates/djs-core/src/vm/ (internal only), crates/djs-bench/ (benchmarks)

## Goal
Calls are the bottleneck of the whole builtin design (D1085: slow paths and builtins are
JS functions). Measured: 3.4M plain JS calls take 0.6 s vs qjs 0.06 s; `sort` with a
comparator on 200k elements takes 1.8 s vs 0.04 s (0034). djs-bench's engine track shows
calls 9.0x, loop 8.5x and props 4.0x slower than qjs (0016). Profile and cut the per-call
and per-instruction overhead, without changing semantics or the suspension/snapshot
properties.

## Acceptance criteria
- [ ] Profiles (native release, and wasm via the 0026 `wasm/` group) of: the calls,
      loop and props microbenchmarks, a comparator sort, and an Array.prototype.map over
      100k elements. Write the top costs in the Log before changing anything.
- [ ] Fixes for what dominates. Candidates (measure first): frame setup/teardown cost,
      re-deriving offsets, bounds checks in the dispatch loop (hoisting, or caching the
      code slice per frame), fuel accounting per instruction, the debug-only checks
      leaking into release, the safepoint check cost, argument copying, `this`
      handling, and intrinsic dispatch overhead for builtin-language code
      (`call_intrinsic` → Rust).
- [ ] No semantic changes: e2e (three modes), difftest and test262 ratchets unchanged
      (or better), and gc-stress / suspension still identical.
- [ ] Numbers before/after in tests/scores/bench.tsv (`scripts/bench.sh --record`), plus
      the djs/qjs ratios for the engine track. Target: a sizable cut in the calls ratio
      (for example 9x → under 4x); report what was achieved and what is structural
      (things that need inline caches, shapes or superinstructions, i.e. later issues).

## Notes
- Interleave measurements (D1190 noise notes); the machine is shared.
- Keep the code readable. A big rewrite of the dispatch loop needs a decision entry
  first: propose it in the Log and ask.

## Log
- 2026-11-01: Baseline (release, Apple M5 Max; medians of 5 interleaved runs of `djs run
  --body`, scratch script `work/scratch/0035/bench.sh`; qjs single runs):

  | bench | djs native | djs wasm (node 25) | qjs | native ratio |
  | --- | ---: | ---: | ---: | ---: |
  | calls.js | 253 ms | 418 ms | 47 ms | 5.4x |
  | loop.js | 731 ms | 1216 ms | 81 ms | 9.0x |
  | props.js | 395 ms | 853 ms | 96 ms | 4.1x |
  | sort 200k with comparator | 1807 ms | 2897 ms | 78 ms | 23x |
  | map 10x100k | 247 ms | 413 ms | 40 ms | 6.2x |

  Instruction counts (a temporary counter, not committed): loop 153M instructions
  (4.9 ns each), props 54M (7.3 ns), calls 39M with 2.2M calls (13.6 ns average). If a
  plain instruction costs ~5 ns, a call/return pair costs ~150 ns. So there are two
  problems: per-instruction dispatch (~15-20 cycles), and an expensive call path.

  Profiles (`sample` native, `node --cpu-prof` on djs-wasm; self time; calls.js):
  - native: `exec` (the op bodies) 37%, the run/step loop 17%, `load_frame` 6%, `push`
    6%, `call` 4%, `imm_u16` 4%, `do_return` 3%, `safepoint` 3%, `imm_i32` 2%,
    `to_number` 2%, `binary_int` 2%;
  - wasm: `exec` 30%, `Vm::run` (the loop, inlined) 18%, `push` 6.6%, `call` 5.3%,
    `load_frame` 4.7%, `imm_u16` 3.6%, `header_int` 3.1%, `do_return` 2.2%,
    `binary_int` 2.2%, `ensure` 2.0%, `safepoint` 1.9%, `pop` 1.5%.

  Top costs, in order:
  1. The tiny stack and immediate helpers (`push`, `pop`, `imm_u16`, `imm_i32`,
     `header_int`, `ensure`) are *not inlined* in either build: every push is a call.
  2. The per-instruction overhead in `step` / `exec`: bounds checks on the code byte, the
     opcode lookup, the `OPS` info, fuel, and `primitive_operands` (D1280's check runs a
     match for *every* instruction).
  3. The call path: `call` + `load_frame` + `do_return` re-derive everything from the
     heap on every call and return (callee → slot 0 → template → CODE → `bytes_of` →
     CONSTS, plus `object_slot`'s bounds assert computing the slot count from the cell
     size); the header is six writes and the return reads four.
  4. Numeric ops through `to_number` / `binary_int` instead of int32 fast paths.
- 2026-11-01: Step 1 (committed): `inline(always)` on step/exec and the stack/imm
  helpers, the cold `grow_stack`, code reads through `Arena::read` (Vec bounds only), and
  int32 fast paths in binary_num/binary_int/compare/to_numeric (the -0 cases checked
  against qjs). Loop 731 to 555 ms, calls 253 to 205 ms.
  Tried and dropped: a `NEEDS_PRIMITIVE[op]` table in front of `primitive_operands`. It
  was *slower* (loop 605 to 720, reproducibly), probably register allocation.
- Step 2, D1350 (committed): `fast_step` runs the hottest simple ops before `step`.
  - Root cause: every exec arm merges its result through a `Step` held in a stack slot,
    because non-inlined helpers return it via sret. The store/reload stall alone was
    ~12% of loop samples. Loop 555 to 385 ms.
  - Calls: `call` was split into `frame_top`/`push_frame`/`enter_template` (no
    re-derivation through `load_frame`), and `fast_step` takes `call` of a closure with
    room on the stack and a `return` into PushResult frames.
  - `fast_step` matches the raw byte (the `fast::` consts), with constant size and fuel
    per arm (`advance`). This removed the `Op::from_u8` lookup and the `OPS` loads.
- Timings with `work/scratch/0035/ab.sh` (interleaved, min of 7; load ~4.5 from other
  agents, so only same-run comparisons count):

  | build | calls | loop | props | sort | map |
  | --- | ---: | ---: | ---: | ---: | ---: |
  | d1350 | 187 | 364 | 327 | 1367 | 191 |
  | bytes (WIP head) | 113 | 306 | 243 | 800 | 111 |

  The baseline was 253 / 731 / 395 / 1807 / 247.
- Tools in `work/scratch/0035/`:
  - `prof.sh <js>`: samply plus a dSYM; self samples by inlined source line and by
    function.
  - `ab.sh save <label>` / `ab.sh cmp <labels..>`: kept binaries, interleaved runs.
  - `wasm-bench.ts`: times djs-wasm through the Sandbox (`DJS_WASM=…`); with
    `--cpu-prof` it profiles.
- Paused here for 0030 part 4. Next:
  - commit-level verification: difftest stress and the test262 ratchet were not rerun
    after the bytes change;
  - profile props (get_field/define_field) and sort/map (comparator calls through
    `call_intrinsic` / the non-fast call path);
  - wasm numbers;
  - `scripts/bench.sh --record` and the ratios.
- Part 1 merge (after the 0030 interlude; rebased onto 9f35b6f): steps 1 and 2, i.e.
  the inlining, the int32 fast paths, and D1350 `fast_step` with call/return.
  - Full checks: workspace tests, e2e in plain/suspend/gc, difftest stress identical
    with no new failures, test262 active with no new failures and no stale entries.
  - Status stays in-progress: props, sort and map still to do, plus wasm numbers and
    `bench.sh --record`.

## Follow-ups

## Result
