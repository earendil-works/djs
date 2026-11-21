# 0268: run_hot's sensitivity, receiver method calls, the general path

status: done
agent: pcall
branch: issue/0268-run-hot-method-calls
depends-on: 0259
touches: crates/djs-core/src/interp/run (run_hot, ic, decline paths, step_once, call_intrinsic, Fault), scripts/perf (tools for the investigation)
guide: interpreter.md
decisions: D3390-D3399

## Goal
1. Understand why any change to run_hot moves V8's code for the whole loop by several
   percent (D1352, D3122, D3128; 0231's follow-up): register pressure, spills, block
   layout, br_table shape, locals count? Compare V8's generated code (--print-wasm-code /
   --trace-turbo) for twins that differ by one arm. The goal is a rule or a structure
   that makes additions safe, since it blocks every interpreter improvement. IDEAS.md
   idea 13 (threaded/tail-call dispatch) is one option; judge it with data, including
   what wasm tail calls (`return_call`) would need from the Rust toolchain on stable.
2. Receiver method calls (`s.trim()`, `a.push(x)`, `s.charCodeAt(k)`): ~300 instructions
   is the prototype inline-cache hit inside run_hot, now the largest part of a builtin
   method call. With 1's understanding, make it cheaper (or move the receiver-kind check
   + builtin id into the IC entry), measured against a build-noise control.
3. The general path (IDEAS.md idea 12): step_once (~30 KB of wasm) and call_intrinsic
   (~39 KB), a boxed Fault; their self time is 5-6% of text/json/heavy. Smaller and
   cheaper where the profiles point.

## Acceptance criteria
- [x] a written explanation of run_hot's sensitivity with evidence (in the issue and a decision)
- [x] per-op instructions for method calls; workload instructions (wmeasure tight/calls/prop1/loop/strings/json + the codemode set) with a noise control
- [x] size reported (room ~2.4 KB under the ceiling, shared); gate green; decisions recorded

## Notes
- pstart (0265 part 2) works in compiler/resolve.rs, not in interp/run.
- pidle (0266) works on promise/async paths in interp (suspend, jobs): coordinate through
  the coordinator if you touch the same functions.

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

### Research, part 1: what every instruction pays in run_hot, and why changes move it (2026-11-21)

Tools (work/scratch/0268 of the primary checkout): `twin.sh NAME` (names build + V8's
TurboFan code of run_hot), `v8arm.py` (reads `--print-wasm-code-function-index`: frame,
the dispatch block, the back-edge block, and per opcode the straight path from its
jump-table target back to the loop header), `slope.py` (steady-state machine instructions
per iteration: a workload at N and 2N iterations, difference / N, `--wasm-sync-tier-up`;
two identical builds agree to 0.02% at N = 20M), `toy/` (threaded dispatch experiment).

**How to measure (three traps).**
- `--no-liftoff` (wmicro's NODE_FLAGS in 0259, D2443's check) does not measure the code
  that ships: without Liftoff's feedback V8 inlines callees into run_hot (32.9 KB of arm64,
  142 calls, 4 moves on the back edge) where the tiered-up code is 21.4 KB (41 calls, 3
  moves): every arm is ~2 instructions longer there, and base vs a twin differ by −0.1%
  under --no-liftoff but −1.5% tiered up.
- wmeasure's totals include Liftoff until the TurboFan code is ready (run_hot compiles in
  ~7 ms; Liftoff code is 1.75x slower: tight 9,114 M liftoff-only vs 5,336 M) and all of
  node's start (2.0 G of the 7.3 G of a `tight` run). `--wasm-sync-tier-up` makes the
  switch deterministic (tight 5,234 M).
- A difference of two runs has ±6 M of noise each: per-statement numbers from 300k
  iterations are ±2 instructions. slope.py with 10–20 M iterations resolves 0.1.

**What a trivial bytecode costs** (tiered-up TurboFan code, main 5f5434e3; `drop` is 33
machine instructions, `get_loc0` 41):
- dispatch block, 21: V8's loop interrupt check 2; `o + 8` overflow check 3 and
  `ip + 8 <= len` 3 (`word()`'s `b.get(o..o+8)`: still checked in the unsafe-heap build);
  the fetch 2; a store of `len` to its spill slot 1; `and`/range check of the br_table 3;
  the table jump 7 (three of them V8's padding).
- shared tail + back edge, 9: `cmp fuel; b.lt`; **load `end` from its stack slot, compare,
  branch, store it back to the same slot**; `mov` for fuel; reload `len`.
- the arm itself, 3 for drop.
So 12 of 33 are the two per-instruction bounds checks (`ip >= end`, the 8-byte fetch) and
the stack traffic of the two values they keep alive (`end`, `len`).

**Prototype P1** (twins/p1-nochecks.patch, 20 lines): the fetch through `arena_read` and
both checks done where the frame changes (run_hot's entry, the call arm, the return arm:
`ip < end`, i.e. not a native frame, and `end + 7 <= len`). `step` already treats
`pc >= code_len` of a bytecode frame as "bad bytecode", so the check only ever caught
native frames. Result: dispatch 14, tail 4 (`drop` 21, `get_loc0` 29). Steady state
(slope.py, instructions per iteration, main → P1): empty loop 293.5 → 220.6 (−24.8%),
`s += i` 503.9 → 385.8 (−23.4%), ten `x;` 1041 → 731 (−29.8%), property get/put 860 → 692
(−19.5%), `s = f(s)` 1009 → 825 (−18.2%), `p.m()` 1392 → 1183 (−15.0%),
`str.charCodeAt(i & 7)` 1589 → 1413 (−11.1%), `Math.floor(i / 3)` 1277 → 1095 (−14.3%).
wmeasure (default tiering, n=3, spread ≤ 0.5%): tight 5340.0 → 4136.4 M (−22.5%), calls
2586.8 → 2206.6 (−14.7%), prop1 2127.3 → 1733.2 (−18.5%), loop 5220.7 → 4065.4 (−22.1%),
strings 1736.6 → 1695.4 (−2.4%), json 1061.9 → 1063.3 (+0.1%). Size +49 B gz (427,465 →
427,514). Unit tests and e2e plain pass.

**Why changes move the whole loop.** Every arm ends in one shared tail (LLVM merges them;
V8 needs a single back edge anyway), so three values (ip, sp, fuel) each have a phi at the
tail and a phi at the loop header, and whatever else is live around the loop competes for
registers there. V8's linear scan decides per value where it lives at the header and at
the tail; when the two differ it emits a move (or a load and a store) that every bytecode
executes: one such instruction is 3% of a trivial bytecode now (5% after P1). Measured
with twins:
- t1 (a `continue` that skips `load_frame!` after `decline_fast`, D3128's case), on main:
  `get_loc` loses a move, `add` trades a reload of a spilled invariant for a `sub`: `s += i`
  503.0 → 495.3 (−1.5%; D3128 saw +4–6% for the same edit on its base). On P1 the same
  edit costs 3 instructions on every bytecode (fuel moved at the header, ip on the back
  edge, two shuffles in the arms): `drop` 21 → 24.
- t2 (a new call-free arm, `neg` of an int) and t3 (a new arm with an out-of-line call,
  `typeof`), on P1: the dispatch, the tail and every hot arm are unchanged (three element
  arms and define_field lose 2 instructions).
So the dangerous edits are those that change what is live around the inner loop (the
outer loop, the exit path, values kept across a call in an arm: D2441), not new arms as
such. The artifacts belong to one V8 version's allocator; what holds on every engine is
the work in the wasm itself (P1 removes wasm instructions).

**Threaded dispatch (IDEAS 13), measured and dropped.** toy/: the same 9-opcode bytecode
(8-byte fetch, fuel per instruction) as a loop with br_table and as one function per
opcode ending in a tail call through a table. Stable rustc 1.99.1 (LLVM 23.1.1) with
`-C target-feature=+tail-call` emits no `return_call` for calls in tail position (0 in
the module; `-C llvm-args=-tailcallopt` neither); `become` (`explicit_tail_calls`,
unstable, RUSTC_BOOTSTRAP=1) emits `return_call_indirect`. In node 26 (TurboFan) the
threaded form needs 339 vs 253 instructions per iteration of `s += i` (+34%) and 446 vs
336 for `x; x; x; x;` (+33%): a `drop` handler is 46 machine instructions (frame setup
and teardown, table bounds check, instance check, signature check, target load) against
~21 in the loop.

### Batch 1: the frame-level checks land (D3390, D3391)

- hot.rs: P1 as described above, with `end` kept for a debug assertion at the fetch
  (`ip < end`; the tests, difftest and the stress modes run it), `fetchable` with a unit
  test, the invariant in `run_hot`'s doc comment. Who guarantees it: the stack checker in
  `alloc_template_in`, the only maker of templates (D3390).
- Tools committed: scripts/perf/slope.py with its cases in scripts/perf/s/, hotcode.sh,
  v8arm.py; README rows; TESTING.md "Measuring the tier-0 loop" (the --no-liftoff trap,
  --wasm-sync-tier-up, steady state, reading V8's code).
- Guide: interpreter.md "Making it fast" has a new item "What every instruction pays"
  and lost the sentence that small additions slow every call.
- Checked: lib tests (625), e2e in the four modes, difftest with `--suspend-every 97
  --gc-stress`, `--lazy --suspend-every 97 --gc-stress --determinism` and `--plain-dumps`
  (also `--lazy`) over language/, codemode/, codemode-harvest/ and dquickts/: no new
  failures, stressed runs identical. All of these are dev builds, so the assertion ran.
- Numbers: D3390 (wmeasure −1.1% pi-like … −23.0% tight; steady state −8.5% … −30.2%;
  +49 B gz).

### Batch 2: arm-local costs, slice reads, string keys (D3392, D3393)

- (a) Arm-local: constants instead of run-time op-table lookups (inc/dec, the
  closure-variable arm, call, return), `get_global` split in two arms, the tier-up exit as
  a fuel mark (`ExecOptions::hot_fuel`, was `hot_backedges`), the wide conditional jumps
  share a copy. Each piece checked alone with hotcode.sh (work/scratch/0268/pieces.py,
  try.sh): one of them (get_global as one arm with the constant) put a move on nineteen
  simple arms and was reshaped.
- Structure experiments, not adopted (D3392): an inner loop that changes only ip, sp and
  fuel (clean inner loop, but each re-entry costs ~40 instructions: prop +29%); six
  stores for the frame header (−36 per call, but V8 swaps two registers on every
  bytecode); branch hints through a post-link section (honoured by V8, no effect on the
  shared path).
- (b) `ic::hot_get`, `hot_length`, `hot_elem`: property, method, `length` and dense
  element hits read from the loop's slice; stores and the rarer hits out of line
  (`field_store`, `field_slow`, `elem_slow`). Debug builds assert every slice hit against
  the heap's code. Own property 138 → 94 instructions, method lookup 206–229 → 143–158,
  `a[1]` 141 → 105.
- pgc's string-key helpers (D3402) wired: `o[k]`, `o[k] = v` in `elem_slow` (only reached
  when the key is no index), `k in o` in `decline_fast`. New corpus case
  language/string-key-elems.js (recorded from qjs), green with suspension, GC stress,
  lazy and plain dumps.
- Outside `touches:`: heap/atom.rs lost the three `#[allow(dead_code)]` on pgc's helpers;
  vm/tests.rs and docs/wasm-jit.md follow the `hot_fuel` rename; tests/scores/size.tsv
  (ceilings, approved by the coordinator).
- Guide: interpreter.md (inline caches item, the decline list, the tier-up exit).
- Checked: lib tests, e2e in four modes, difftest over the corpus plain and with
  `--suspend-every 97 --gc-stress` on codemode/, language/, builtins/, test262
  built-ins/Object, Array, TypedArray, Proxy and language/expressions.
- Size +758 B gz (429,785), ceilings raised to 429,790 / slim 363,630 (D3393).

### Batch 3: the general path (D3394)

- Where declined instructions go and what they cost: scripts/perf/instr.py on the
  workloads (2.7–7% of bytecodes through `step`, 0% on four of eight), and slope.py per
  instruction: the trip through `step_once` is 30–170 of 400–3,000 instructions. So no
  redesign of `Step`/`Fault`.
- Template literals: `to_string` of a string in the loop, `concat` and the other
  `to_string`s from `decline_fast` through `Exec::concat_op` (shared with `step`);
  `+` of two strings without conversions. strbuild −15.4%.
- Bytes: `call_intrinsic` reads arguments 1–3 once (−1,209 B gz) and has one
  bad-argument exit (−119 B gz); 38.8 → 30.8 KB of wasm. Ceilings back at 429,648 and
  363,349. Tools for this in work/scratch/0268: ngram.py (repeated instruction runs in a
  wasm function), arms.py.
- Tried and left out: object literals/`typeof`/`push_atom` from the exit; absent
  properties by a chain lookup (accessors pay for it).
- Guide: interpreter.md's decline list names template literals.
- Checked: lib tests, e2e in four modes, difftest (corpus plain; builtins/, codemode/,
  language/ with suspension and GC stress, codemode/ and language/ with determinism),
  test262 built-ins (18,083 active, no new failures).

## Follow-ups
Ranked by what they are worth per instruction saved and how often scripts do them
(steady-state instructions per operation on main after this issue, slope.py):
1. **String concatenation in the heap** (heap/string.rs, outside this issue): `"a" + s`
   is 865, of which `Heap::concat` of two short flat strings is ~560 (`write_leaves` 250
   to copy three bytes). A direct two-piece copy for flat operands would take ~350 off
   every `+` and every part of a template literal (`` `a${s}b` `` is 2,083).
2. **A cache kind for absent properties**: `o.missing` 1,000 (QuickJS 771 with the
   loop), `o.missing ?? 1` 1,164. The addition slots' machinery (`chain_lacks`,
   `OBJ_IS_PROTO`, the epoch) proves the same thing: receiver shape + prototype + epoch
   → undefined, ~60 instructions. A plain chain lookup on a miss gives 674 but costs
   every accessor read +14–25% (twins/d4-absent-lookup.patch).
3. **`typeof x === "string"`** 1,082: `typeof` leaves the loop and the comparison calls
   `strict_equals` for two strings; `typeof` yields one of eight atom strings, so
   `strict_eq` of two identical string cells can answer in the loop (it does: `a == c`),
   and `typeof` as an arm needs only `atom_string` of a fixed atom (no allocation in a
   VM with a base image).
4. **Superinstructions** (compiler + op table): a dispatch is 18 of a simple
   instruction's ~21–29; `get_loc; push_i8; lt; jump_if_true8` → one instruction.
5. **`Map.get` / `Set.has`** 1,085 / 1,058 and **`a.push(i)`**: `@fast` calls pay the
   failed closure attempt, the exit and `fast_leaf_call`'s dispatch (D3301); the call
   arm could test the template's FAST flag before building a frame.
6. **for-of over a Set/Map** (`iter_next_loc` declines for collection iterators) and
   **`f(...args)`**: not looked at.
7. **The add cache for a key that shadows a writable inherited data property**
   (`e.name = 'X'` on an Error): `chain_lacks` refuses any key on the chain.
- The frame header written by six stores instead of a loop: −36 of a call's 178
  instructions, blocked by a V8 register swap on every bytecode (D3392). Worth another
  attempt whenever run_hot's shape changes.
- The two-loop structure (D3392): needs frame values read from `Exec` at re-entry; the
  inner loop it gives is 3 instructions shorter per simple bytecode.
- Superinstructions are open again: a new arm does not disturb the shared path (D3391),
  and a dispatch is ~18 of a simple instruction's 21 machine instructions. `s += i` runs 9
  bytecodes per iteration; `get_loc; push_i8; lt; jump_if_true8` as one compare-and-branch
  and `get_loc; get_loc; add` would make it 5. Needs the compiler's emitter and the
  bytecode table (outside this issue's `touches:`).

## Result
Three batches on main: 4f134156 (batch 1), d7ff0d88 (batch 2), batch 3 with this commit.
- **Why edits to run_hot moved everything** (D3391): every bytecode runs one shared path
  (dispatch block, tail, back edge); V8's register allocator places each value that is
  live around the loop separately at the head, in the arms and at the tail, and a
  disagreement is a move or a stack load on that path, 5% of a simple bytecode. Edits
  that change what is live around the loop do it; new arms did not. The procedure:
  `scripts/perf/hotcode.sh` before and after, diff the summaries; `slope.py` for the
  steady state; never `--no-liftoff` (TESTING.md "Measuring the tier-0 loop").
  Threaded dispatch measured +33% and needs unstable Rust; a two-loop structure gives a
  move-free inner loop but costs 40 instructions per re-entry (D3392).
- **What every bytecode pays** (D3390): 33 → 21 machine instructions for `drop` by
  checking the end of the code where the frame changes instead of per instruction.
- **Property, method and element hits** read from the loop's slice (D3393): own
  property 138 → 94, method lookup 206–229 → 143–158, `a[1]` 141 → 105; string keys
  through 0269's helpers: `o[k]` 1521 → 943.
- **The general path** (D3394): the trip is not the cost; template literals −20–29%;
  `call_intrinsic` 8 KB smaller.
- Shipped module, wmeasure M instructions, main 5f5434e3 (before) → main after batch 3,
  measured batch by batch against the main of the day (n=3, spread ≤ 0.9%):
  tight 5335 → 4107 → 3903 → 3914; loop 5221 → 4054 → 3851; calls 2578 → 2179 → 2062;
  prop1 2129 → 1736 → 1520 → 1514; arrays 7054 → 6076 → 5234 → 5229; call1 3369 → 2825 →
  2686; pushpop 2478 → 2304 → 2126; strbuild 2478 → 2414 → 2405 → 2038; strings 1735 →
  1682 → 1664 → 1620; json 1065 → 1045 → 1041 → 1031; pi-like −1.1%, −1.5%, −0.3%.
- Size: 427,465 → 428,691 B gz over the issue with other issues' bytes in between;
  0268's share +49, +758, −1,096 = −289 B. Ceilings as before the issue (429,648; slim
  363,349).
- Tools: scripts/perf/slope.py (+ s/), hotcode.sh, v8arm.py.
Known gaps: the follow-ups above; the frame-header stores (−36 per call) blocked by a V8
register swap; wall-clock numbers were taken at load 11–19 only (D3390).
