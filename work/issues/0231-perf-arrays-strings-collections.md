# 0231: Performance: sort with a comparator, Array.from, join/concat/push, Map/Set string keys

status: done
agent: parr
branch: issue/0231-perf-arrays-strings
depends-on: -
guide: builtins.md if a fast-path story changes
touches: builtins (array, string, collections), interp natives for sort/join, heap string hashing for Map/Set

## Goal
Measured by benchpage (0229) against the best QuickJS wasm build (QuickJS 2026-06-04 / QuickJS-NG 0.18, wasi-sdk -O2), node 26; scripts in .worktrees/benchpage/work/scratch/0229/ops.ts, ops2.ts (need vendor/engines from scripts/bench-engines.sh --setup on that branch).
- Array.prototype.sort with a comparator 5.3-6.1x slower (native sort re-entering the VM per
  comparison); default string sort 1.7-1.9x.
- Array.from({length}, fn) 2.3x.
- Array.prototype.join of 100k strings 1.7x (3.2x on 30k lines); string a+b concat 2.2x; push
  1.7-1.9x.
- Map.set / Set.add with string keys 2.5-2.7x (string hashing per op?).
Find each root cause and fix; target at least parity with QuickJS.
Constraints: exact semantics (test262 and the difftest stay green); measure in wasm instructions on the shipped module before/after and report size cost (the shipped module has ~15 KB gzip headroom shared by 0230-0234: keep each change small, report bytes; the coordinator arbitrates); several agents work in djs-core at once (0230 builtin calls, 0231 arrays/strings/collections, 0232 async, 0233 regexp, 0234 memory): stay in your touches, ask the coordinator before changing shared paths (the interpreter's call path, the heap). Batch merges, gate checks locally, niced builds, benchmarks only at load < ~4.

## Log
- Measuring: work/scratch/0231/m.py counts steady-state wasm instructions (`/usr/bin/time -l`
  of node: (K+1 runs - 1 run)/K of a codemode body in fresh VMs, so V8's tier-up mostly
  cancels) for djs builds against benchpage's QuickJS 2026-06-04 and QuickJS-NG 0.18
  (wasi-sdk -O2); workloads in work/scratch/0231/w (the ops.ts cases, as files). Instruction
  counts were steady to ~3% even at load 100-170, wall times were not.
- Root causes found (profiles of a named shipped build, node --cpu-prof):
  - sort with a comparator: the native SortList went through ~12 resume points per comparison
    (Gets, CreateDataPropertyOrThrow, slot traffic, non-inlined n_slot/f64_of helpers in wasm);
    the comparator call itself was <20%. Fixed by D3125 (merge state in a heap array, merge in
    leaves, inline leaf forms in natives): 3.2x -> 1.5-1.7x. The rest is n_call (frame write)
    and the native re-entry per comparison: the call path, 0230's.
  - default string sort: a Vec<u16> per element via the chunk iterator (D3125): sort itself at
    parity (sortstr 1.8x -> 1.3x incl. the setup's string building).
  - Map/Set with string keys: not hashing (~5%) but the bytecode builtin's frames (D3127):
    2.0-2.4x -> 1.1-1.5x.
  - a + b: run_hot declined every string Add to `step`; short concats zeroed two 32-entry
    arrays (D3128): 1.7x -> 1.4x.
  - Array.from({length}, fn): a Construct(%Array%, len) call and a final length Set per call,
    and per-element property lookups on the array-like (D3129): 1.8x -> 1.2-1.3x.
  - join: split lines are slices, which the one-byte join path refused (D3129): 2.0x -> 1.5x
    (lines), 1.4x -> 1.1-1.3x (100k); the rest is the compactor moving the big results.
  - push: run_hot declines every @fast call; fast_leaf_call, try_fast_call's generic steps,
    array_push and set_element_fast's checks were ~40% of a push loop. 0230 tried @fast calls in
    the call arm and dropped it (plain calls -7-10%, D3122); the coordinator handed push back.
    D3128: a dedicated push shortcut in fast_leaf_call and `Heap::array_append_fast`: 1.56x ->
    1.20x.
  - V8 sensitivity (D3128): any change to run_hot's own code, even a `continue` in its exit
    path or a concat call in the Add arm, made plain loops and calls 4-6% slower (bisected over
    builds; alternating A/B runs). The loop's code now differs from main only in calling one
    out-of-line `decline_fast` where it called `fast_leaf_call`: calls and loop ±0 vs main.
- Size, against main at 0211f403 (after 0230, 0230b, 0232, 0233): shipped 422,242 -> 423,167 B
  gzip (+925); both ceilings raised to the measured sizes, 423,167 and 356,911
  (coordinator-approved, D3127). Map/Set entries ~270 B; the sort change made the module smaller.
- Tests: unit test of the sort leaves (same comparisons as crate::sort, stability, an
  inconsistent comparator), corpus case builtins/collections-sort-concat.js (Map/Set fast
  paths and their declines, sorts, joins of slices, string + primitive in loops, Array.from
  with other constructors), bgen behavior stand-ins for the new intrinsics; test262 Array,
  Map, Set, TypedArray sort, addition green on both backends; difftest ratchet OK, stress OK.
- Guide: interpreter.md (the decline path of the tier-0 loop), builtins.md (the native list's "merge step" is now the comparison loop; the @fast
  list names Map/Set). interpreter.md's sort paragraph still holds.
- Merging: build:wasm failed on rust-lld's `undefined symbol: anon.*.llvm.N` (a stale
  incremental cache the gate's D2896 retry did not recognize), then the test leg on a stale
  djs-prelude in the gate's target (the coordinator cleaned it; the 8 tests passed on the
  rebased tree here). D3126 (amends D3022): scripts/gate.py's `link_corruption` treats any
  undefined symbol whose object is one of our crates' codegen units, or LLVM-local, as
  corruption; scripts/gate-test.py (leg `gate self-test`) checks it.
- ops.ts (wall time, load ~170, so only indicative), djs/best before -> after: sort comparator
  6.6 -> 3.4, sort lines 2.1 -> 1.0, Array.from 20 2.4 -> 1.4, Map string keys 2.7 -> 2.5,
  Set 2.5 -> 1.7, join 100k 2.2 -> 1.45, a + b 6.5 -> 2.8 (instructions say 1.1-1.3x: the wall
  time ratios carry tier-up and load noise).

- Final steady-state instructions (work/scratch/0231/final-table3.txt), main -> 0231, ratio to
  QuickJS 2026: sort comparator 3.45 -> 1.62, default sort 0.96 -> 0.74, 30k-line sort (with
  setup) 1.72 -> 1.24, Array.from 20 1.87 -> 1.25, join 100k 1.44 -> 1.17, join lines 2.22 ->
  1.61, a + b 1.69 -> 1.36, Map string keys 2.50 -> 1.70, Set 1.97 -> 1.26, push 1.55 -> 1.20 (1.60 -> 1.18 on 0211f403),
  loop 0.85 -> 0.86, closure calls 1.07 -> 1.06.

## Follow-ups
- Any change to run_hot's own code moves V8's code for the whole loop by several percent
  (D3128, D3122): worth a look at why (register pressure, spills) before more arms are added.
- Comparator sort: the remaining 1.5-1.7x is n_call's frame write plus re-entering the native
  frame per comparison (call path: 0230).
- Bytecode builtins' `Get(o, "length")` and other string-literal keys intern the string on
  every call (find_base_atom in Array.from's LengthOfArrayLike of a non-Array): prelude
  string constants used as keys could carry their atom.
- Joining big strings: the compactor moves the large results (0234's area).
- Wall-time ratios are much worse than instruction ratios for the native sort (1.6x vs ~3x):
  worth checking whether natives run in Liftoff longer than run_hot.

## Result
Landed: D3125, D3127-D3129 (and D3126, the gate's corruption detector). Comparator sort merges
in leaves (`%SortStart/SortStep/SortOperand`, inline in natives); default sort compares strings
in place; Map/Set get/has/set/add are `@fast`; `+` of a string and a primitive and `arr.push(x)`
finish in run_hot's decline path (`decline_fast`, `Heap::array_append_fast`) with the loop's own
code unchanged; short concats of two flat pieces skip the leaf walk; Array.from from %Array%
skips Construct and the final length Set, and its array-like reads skip objects without indexed
properties; join reads one-byte slices in place. Steady-state wasm instructions vs QuickJS 2026
(-O2), main -> 0231: sort comparator 3.45x -> 1.62x, strings sort at or below parity, Array.from
1.87x -> 1.25x, join 1.4-2.2x -> 1.2-1.6x, a + b 1.69x -> 1.36x, Map/Set string keys 2.0-2.5x ->
1.3-1.7x, push 1.55x -> 1.20x, plain loops and calls ±0. +0.9 KB gzip (ceilings raised, D3127).
Known gaps: see Follow-ups.
