# 0081: Why is djs an order of magnitude slower than QuickJS? (disassembly-level investigation)

status: in-progress
agent: perf
branch: issue/0081-perf-anatomy
depends-on: -
touches: work/issues/0081-perf-anatomy.md (the report), work/scratch/0081/, scripts/ or crates/djs-bench/ (new measurement tools only). No engine changes.

## Goal
On real workloads djs is still roughly an order of magnitude slower than QuickJS (pi-like
codemode script ~4–5x, for-of 10–15x, push/pop 6x, indexOf 7x, after 0035/0062/0063). The
owner wants to understand **why**, at the machine-code level. QuickJS is a reference
point, not the goal (it is itself far from optimal), so the question is "what does djs
spend instructions and memory traffic on that a well-designed interpreter would not", not
"how do we copy qjs".

## What to deliver
- [ ] Profiles with attribution down to instructions: macOS `sample`/`xctrace` (no
      installs; use what is on the machine; `otool -tV` / `objdump` from Xcode,
      llvm-tools only if already installed) on a native release build with symbols
      (`debug = 1` is already in the release profile), and the wasm build in node
      (`--cpu-prof`, plus inspecting the wasm code for the hot functions). Workloads: a
      tight loop, calls, property access, a for-of loop, push/pop, string building, a
      pi-like codemode script, a JSON-heavy script.
- [ ] Disassembly of the hot paths: the dispatch loop (`Exec::run`, `fast_step`),
      instruction decode (immediates, bounds checks), the value stack and frame access
      (where `pc`, `sp`, `fuel`, `bp` live: registers or memory, re-derivation after calls
      or GC, D1350 notes), NaN-boxing tag checks, arena reads and writes (bounds checks,
      D1260), call/return (frame push, argument copying, header writes), the
      `call_intrinsic` dispatch, native frames (0063), allocation, and the safepoint and
      fuel checks. Count instructions and memory ops per bytecode for common ops, and per
      JS-level operation (for example one `arr.push(x)`).
- [ ] Root causes, ranked by impact with evidence: for example dispatch design (a match
      on a u8 with bounds checks vs threaded code), state kept in memory instead of
      registers, Result/Fault plumbing on hot paths, heap bounds checks, generic arena
      accessors, allocation per operation (iterator results, arguments arrays,
      closures), the builtin-language layering (how many intrinsic calls per builtin),
      fuel accounting, and so on.
- [ ] Proposals per cause: the expected gain, the effort, and the risk to the
      architecture's guarantees (position-independent heap, suspend anywhere,
      deterministic images, no unsafe without a decision). Separate cheap fixes from
      structural redesigns, and say what the ceiling looks like for this design.

## Notes
- The machine is shared and loaded: profile with care, repeat, and report variance.
  Don't record benchmarks (bench.sh --record) under load.
- Own worktree.

## Log

## Findings

## Proposals

## Result
