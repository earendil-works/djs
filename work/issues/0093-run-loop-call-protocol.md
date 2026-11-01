# 0093: Run-loop state in locals and a compact call protocol (0081 P6 + P9)

status: done
agent: perf
branch: issue/0093-run-loop-call-protocol
depends-on: 0085
touches: crates/djs-core/src/interp/ (run loop, frames, call/return), docs/compiler.md 7 (frame contract), docs/ (a design note first)

## Goal
After 0085, the remaining per-call gap to quickjs-wasi (1.2–2x) is the call path itself
(~1k wasm instructions before a builtin entry runs), and plain bytecode pays for
interpreter state kept in `Exec` fields (load–modify–store per op; 0081 F2: one `get_loc`
is 52 native instructions, ~5 of them useful). 0081 P6 modeled −29% instructions per op
with the run-loop state in locals (safe Rust); P9 estimated call+return from ~420 to
~150–200 instructions.

## Acceptance criteria
- [x] **Design first** (a short note): what lives in locals (pc, sp, fuel, lp, the code and
      stack slices or their base offsets), when it is written back (calls, allocation,
      GC safepoints, suspension: the "suspend anywhere" and image guarantees must hold
      exactly), how the 0062 ICs, native frames (0063, D1631) and @fast entries (0085)
      interact, the frame header layout for a compact call protocol, and the expected
      gains. **Send it to the coordinator before implementing** (heap's 0083 shapes and
      infra's 0090 touch interp at the same time; we'll sequence the merges).
- [x] Implementation in parts with wasm instruction counts (primary) and native, the run
      loop's wasm function size (it must still tier up: 0081's caveat about one huge
      vm_run slice), and identical ratchets, stress modes, and determinism checks.

## Design (for review before implementing)

### 1. A tier-0 loop with its state in locals

A new function `Exec::run_hot(&mut self) -> Hot`, called by `run_loop` before
`fast_step`, runs the hottest instructions with the interpreter state in locals and
returns when it meets anything else. Everything outside it stays as it is: `Exec`'s
fields are the canonical state whenever `run_hot` is not running.

- **Locals** (loaded from `Exec` on entry): `pc` (as the absolute arena offset of the
  instruction, `code + pc`), `sp`, `fuel`, `bp`, `hp`, `lp`, the current template, the
  code base, the constant pool, the stack cell's value base (`stack + 16`), and the arena
  as a local `&mut [u8]` (a new `Arena` accessor over the Vec, the same range the
  accessors check against today, D1261). Plus two flags read once on entry: whether a
  collection is wanted at the next safepoint (`gc_stress || gc_pending`) and
  `slow_call_flags`.
- **Why it is safe to cache them:** `run_hot` executes only instructions that never
  allocate, never run user code and never call out of `run_hot`. So nothing can move a
  cell (no collection), grow or reallocate the arena or the stack, or change
  `gc_pending` while it runs, and the borrow checker enforces that the slice is dead
  before any `self` method runs. Every offset is re-read from `Exec` at the next entry,
  after whatever collected or grew in between (`load_frame` already re-derives the
  frame data after every safepoint).
- **Written back on every exit, and only there** (pc, sp, fuel, and bp/hp/lp/template/
  code/consts/code_len when a call or return changed the frame). Exits:
  1. the next instruction is not a tier-0 instruction, or a tier-0 instruction needs
     its general path (a non-int operand, an overflow, a non-Bool condition, a callee
     that is not a plain closure, a frame that does not fit the stack);
  2. fuel ≤ 0 at an instruction boundary (the outer loop then flushes and returns
     `OutOfFuel` exactly as today);
  3. a safepoint (frame entry, return, backward jump) with a collection wanted: the
     instruction completes, `run_hot` exits with "safepoint due", and the outer loop
     runs `safepoint()` at that same point;
  4. a bounded number of backward jumps (e.g. 2^16) without any other exit, so that a
     long tier-0 loop returns to `run_loop` and V8 can tier `run_hot` up (no OSR in
     wasm, 0081 F0). An exit changes no state, so this is unobservable.
- **Suspend anywhere and images:** fuel is charged per instruction with the same costs
  and checked at every instruction boundary, safepoints happen at the same instructions
  with the same stack contents (exit 3 runs the existing `safepoint()`), and every arena
  write is the same write the current code makes. So a run suspends at the same
  boundaries, collects at the same points, and leaves byte-identical images; the stress
  modes, the determinism gate (D1720) and `warm_and_cold_inline_caches_behave_the_same`
  check it. D1700 retries concern only allocating instructions, which are never tier-0.
- **The set, part 1:** `get_loc`/`put_loc`/`set_loc`, `get_arg`, `get_cap`/`put_cap` (a box
  read through the closure's slot), `push_i8/i32`, `push_const`, `undefined`/`null`/
  `push_true/false`, `drop`/`dup`/`swap`/`nip`/`insert2`, `to_numeric`/`inc`/`dec` on
  numbers, int and float `add`/`sub`/`mul`/`div`, the relational ops, `strict_eq/ne` on
  primitives, `not` on Bool, `jump`/`jump_if_*`. Their `fast_step`/`fast_more` arms are
  then removed (code size, and `fast_step` gets smaller).
- **Decoding:** one bounds check per instruction for the opcode and its immediates: an
  8-byte little-endian load at `ip` (the longest instruction is 7 bytes; a fallback for the last 8
  bytes of the arena), immediates by shifts. Stack slots: one-sided checks (`get(off..
  off + 8)`, one compare) instead of today's two-sided `first_chunk` checks.

The model (0081, `scripts/perf/model`) puts this at −29% instructions per op in safe Rust
(37.6 → 26.9); the real `get_loc` is 52 native / ~80 wasm instructions today, expected
~20–25 / ~40.

### 2. A compact call protocol (part 2)

`call` of a plain closure (no SLOPPY_THIS/NATIVE/FAST flags, frame fits the stack
capacity) and `return`/`return_undef` into a `PushResult` frame of a bytecode caller run
in `run_hot`:

- **Same frame layout** (docs/compiler.md 7.1: callee, this, args, the 6-word header,
  locals, operands). Heap images, `Vm::backtrace`, native frames and the GC's stack
  tracing do not change. The cost today is not the layout but how it is accessed: each of
  the six header stores reloads the arena Vec and is checked on its own (0081 F4: 72 of
  call's 234 instructions are bounds checks).
- **Block access:** the template's call data is read from one checked 28-byte window
  (FLAGS, PARAMS, LOCALS, STACK, CODE, CONSTS are at offsets 8–35 of the cell); the header
  is written as one checked 48-byte block (a `&mut [u8; 48]` from one range check, then six
  unchecked-by-construction stores), the locals filled through one checked range, and a
  return reads the header as one block. One check per block, all in safe Rust.
- **No `code_len` on the hot path:** the code length is only needed to send native
  frames to `native_step` and to report corrupt bytecode. `run_hot` keeps a frame kind
  instead (a return into a native frame, or into anything but `PushResult`, exits), and
  the slow path keeps today's check. So a call no longer follows CODE → Bytes::LEN, and a
  return re-derives the caller with stack[prev_bp] → slot 0 → template → CODE/CONSTS
  (four loads, each one compare).
- Rejected: adding a seventh header word with the caller's template (saves those loads,
  but every call pays a store, every frame 8 B, and the image format changes); packing
  the header into fewer words (they are int32 Values so that the GC traces the stack
  without frame knowledge, docs/compiler.md 7.1). Revisit if return dominates.

Expected: call+return ~420 → ~120–150 native instructions (0081 P9).

### 3. ICs, native frames, @fast, retries

- **ICs (0062, D1620/D1621):** in part 1 the field ops exit to `fast_field` as today (one
  exit/entry, ~10 instructions). After heap's 0083 shapes lands, part 3 moves the IC hit
  check (a shape compare) into `run_hot`, reading the IC slots and the object through the
  slice; the miss path stays where it is. That ordering avoids colliding with 0083's IC
  rewrite.
- **Native frames (D1631, infra 0090):** native code keeps using `Exec`'s fields through
  the `n_*` helpers; `run_hot` never runs a native frame (its frame kind exits) and never
  calls into one. A bytecode call of a NATIVE template exits (slow_call_flags), as
  `fast_call_at` declines it today. Infra's native-frame retry is in the outer loop and
  unaffected.
- **@fast (D1850):** a `call` whose callee has FAST exits; the run loop's existing
  `fast_leaf_call` handles it next.
- **Fuel debt and `fuel_start` (D1740):** set by `run_loop` before the first entry; `run_hot`
  only decrements `fuel` like the other paths.
- **Retries (D1700):** in the outer loop only (tier-0 instructions never allocate).

### 4. Code size and wasm

`run_hot` is a separate function of ~10–15 KB of wasm, re-entered at every exit, so V8
tiers it up within a slice; `run_loop` loses the arms that move. Measured per part:
wasm instructions (wmeasure n=3, wmicro), native, the wasm sizes of `run_hot` and
`run_loop`, and the stress, determinism and ratchet checks unchanged. If part 1 makes
D1862's dense opcode range pay off for `run_hot`'s own dispatch, that is a separate
measurement.

### 5. Parts and conflicts

1. `run_hot` with the stack/arith/jump set, the arena slice accessor (a one-line
   addition to heap/arena.rs), removal of the moved `fast_step` arms. Touches run.rs's
   `run_loop` in one place (the call before `fast_step`).
2. Calls and returns in `run_hot` (block header access).
3. After 0083: IC hits in `run_hot`; then `get_elem`/`put_elem` on dense arrays.

Decisions: D1930 (the tier-0 loop and its exits), D1931 (the call protocol), recorded
with the parts that land them.

## Log

- Design reviewed by the coordinator (above). Part 1 alone (stack/arith/jump ops in
  `run_hot`, exiting to `fast_step` for the rest) measured *worse* on mixed code: every
  exit/entry cost ~60 native instructions (write-back, an outlined `fast_step`, reload),
  a tier-0 op saved ~25 (wasm: tight −20%, props +32%, arrays +20%). With the
  coordinator's agreement parts 1–3 were combined: `run_hot` now runs everything
  `fast_step`/`fast_more` did (calls, returns, IC hits through ic.rs's hit functions,
  global caches, dense elements, the Array Iterator step, the leaf `call_intrinsic`
  cases) and calls `step` in place (`step_once`) instead of returning, so leaving the
  loop costs only a write-back and a reload around an instruction that costs hundreds
  anyway. The leaf heap calls end the slice borrow and re-take it (`heap_call!`); none
  collects. The old fast paths are `#[cfg(test)]`: the reference of
  `hot_loop_is_invisible` (step alone is not: its slow paths refill IC slots on every
  execution and grow elements differently, so images differ in caches).
- Outside `touches:` (small): heap/arena.rs `Arena::hot_bytes`; iter.rs
  `iter_next_hit`; `#[cfg(test)]` on `fast_field` (ic.rs, heap's 0083 part 3 is there:
  whoever lands second ports) and `fast_intrinsic`.
- Condition (a): `hot_loop_is_invisible` compares image hashes and output at every stop
  against the old fast paths: fuel slices 1/3/13/61/97 with and without restoring the
  image, GC stress (every frame entry, return and back-edge collects), garbage that makes
  a collection pending at tier-0 back-edges, the periodic exit at every backward jump. It
  fails when a fuel charge, the back-edge, frame-entry or return safepoint is removed
  (checked by mutation).
- Rebased onto main after heap's 0083 part 3 (shared IC hit checks `ic_get`/`ic_put`,
  addition caches, `fast_define`): `run_hot` calls `ic_get`/`ic_put` and runs
  `define_field` hits too; a cached addition may allocate (never collect), handled like
  `set_element_fast`'s append (the slice and `gc_pending` are re-read); `fast_define` is
  test-only with the other old fast paths.
- Condition (b): wasm (`wasm` profile): `run_hot` 16,463 bytes, `step_once` 35,543
  (`step` + `exec` inlined; main's `run_loop` 52,982), `Exec::run` 926; code section
  924,028 → 922,840 bytes. `--trace-wasm-compilation-times` on pi-like: `run_hot`
  (#1135) and `step_once` (#1141) are TurboFan-compiled with FUEL=10000 (pi) and
  1000000 (Sandbox). A single 10^12-fuel slice of tight: 8.6G instructions (main 16.8G,
  stuck in Liftoff): the periodic exit fixes 0081 F0's caveat.
- Measurements, against main bf7eb626, wmeasure n=3, wasm instructions; native:
  measure.py exact:

  | workload | wasm | native |
  | --- | ---: | ---: |
  | tight | −21.3% | −28.0% |
  | loop | −22.2% | −27.7% |
  | call1 | −27.1% | −28.6% |
  | calls | −23.4% | −25.8% |
  | prop1 | −19.3% | −19.3% |
  | indexof | −25.8% | −0.6% |
  | forof | −13.7% | −18.6% |
  | pushpop | −10.1% | −8.7% |
  | arrays | −9.4% | −15.4% |
  | strings | −5.6% | −4.8% |
  | props | −4.6% | −7.7% |
  | push-pop | −4.2% | |
  | strbuild | −2.6% | −4.9% |
  | higher-order | −1.4% | −3.5% |
  | pi-like | −1.0% | −1.0% |
  | jsonh / json | 0.0 / +1.0% (spreads 0.8–1.4%) | – / −0.2% |

  Native per op (micro.py, before the rebase): `get_loc`+`drop` 77 → 61, call+return
  631 → 458, `o.c` 214 → 185, `a[i & 7]` 392 → 299, `o.m(i)` 741 → 556; step-path ops
  pay ~+30 (object literal 457 → 492).
- Checks: lib tests, e2e plain/suspend/gc, difftest ratchet on both backends, difftest
  stress and `--determinism` on builtins/codemode/language/dquickts subsets, test262
  active group (no new failures), clippy native and wasm32.

## Follow-ups

- heap: port the IC hit decoding to its shared function once 0083 part 3 lands
  (`run_hot` calls `ic_own_get`/`ic_proto_get`/`ic_own_put`/`ic_global_*`, so it follows
  automatically; `fast_field` is test-only now).
- Step-path instructions pay ~30 native instructions for the write-back and reload; the
  next candidates for `run_hot` by frequency: `define_field` of a new key (needs
  shapes' transition cache), `object`, `get_global` misses, `concat`.
- D1862's dense opcode range could be re-measured for `run_hot`'s own dispatch (two
  `br_table`s in wasm now).
- `run_hot` could take the IC slot array once per frame instead of per access.

## Result

Landed: the tier-0 loop (D1930) and its call protocol (D1931). `Exec::run_hot` runs the
interpreter with `pc`, `sp`, `fuel` and the frame data in locals and the arena as a
local slice, executing the fast cases of the common instructions (calls and returns
included) and calling `step` in place for the rest; nothing in it collects, and fuel,
safepoints, suspension points and images are those of the old fast paths
(`hot_loop_is_invisible`). wasm instructions −19 to −27% on plain bytecode and calls,
−5 to −26% on arrays/strings/for-of/push-pop/indexOf, everything else within ±1%; native
−19 to −29% on plain bytecode. wasm code size unchanged (−1.2 KB). V8 tiers `run_hot` up within a slice, including one huge slice.
