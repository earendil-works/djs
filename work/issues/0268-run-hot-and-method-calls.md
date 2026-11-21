# 0268: run_hot's sensitivity, receiver method calls, the general path

status: in-progress
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
- [ ] a written explanation of run_hot's sensitivity with evidence (in the issue and a decision)
- [ ] per-op instructions for method calls; workload instructions (wmeasure tight/calls/prop1/loop/strings/json + the codemode set) with a noise control
- [ ] size reported (room ~2.4 KB under the ceiling, shared); gate green; decisions recorded

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

## Follow-ups

## Result
