# 0155: Fix the instruction profiler; no stale stack values in images

status: done
agent: perf
branch: issue/0155-profiler-stack
depends-on: 0153
guide: docs/guide/snapshots.md / determinism.md if image content rules change
touches: scripts/perf/instr.py (+ its patches), scripts/perf/README.md, crates/djs-core/src/vm/ (image/session writers: session.rs, bytes, canonical copy), crates/djs-core/src/heap/ (only if the clearing belongs in the collector), tests; bloat (0151 L4) works in compiler/ and interp/ run_hot: coordinate via the coordinator before touching interp/

## Goal
jit's 0153 follow-ups:
1. **scripts/perf/instr.py is broken**: its `gc.rs` patch no longer applies (the collector
   changed in 0120–0148) and its fast-path counters read zero. Make it work on current main
   and robust against the next refactor (patch by anchors that are checked, fail loudly if a
   patch doesn't apply, a self-test in the script), and split interpreter time between user
   code and bytecode builtins (the split 0153 couldn't get).
2. **Stale values above `sp` in images**: images (and so session images, containers,
   canonical_hash input) include dead value-stack slots above the stack pointer. Equal
   histories still give equal bytes, but the dead values keep cells alive or at least
   cost bytes and make images depend on garbage. Clear them so images contain no data
   above `sp` (and other dead per-VM scratch, if the analyzer shows any: check `djs heap`
   on a REPL and a pi idle snapshot). Prefer doing it where the image is written (no
   runtime cost) if that's sound for the collector (dead slots must not be traced as
   roots); otherwise at suspension. Measure session-image and container sizes before and
   after, and check determinism/canonical_hash and the prefix barrier.

## Acceptance criteria
- [x] instr.py runs on main, with the user-code vs builtin split, documented; a check
      that it applies (in its own self-test, not the gate unless cheap).
- [x] Images have nothing above sp; a test asserts it; sizes reported.
- [x] No workload slower than +1%; size ratchet respected.

## Log
- instr.py (D2309): rewritten patcher (checked anchors, every mismatch listed with the
  nearest lines, non-zero exit), counters in a generated `prof.rs`, hooks at the tier-0
  loop's opcode decode, `step_once`, frame pushes, native arms, `call_intrinsic`,
  `alloc_cell`, `after_collect` and the collectors' entries. User/builtin split by the
  template's PRIVILEGED flag, per path, opcode and function (builtins named by registry
  path); time split by a 50 µs sampler. `--self-test` builds and checks every counter
  moves (passes on this branch). Native split on the workloads:
  pi-like: bytecodes 72.5% user / 27.5% builtins; time 47.0% user bytecode, 38.8%
  bytecode builtins (with the leaf intrinsics they call), 2.3% @native, 11.9% collections.
  higher-order: 74.9 / 0.3 / 19.5 (sort) / 5.3. json: 5.3 / 74.1 / 0 / 20.6.
- Stale slots (D2308): canonical_hash and every collection already cleared the slots
  above sp (collect_heap); JS hosts' images are always taken after vm_collect/vm_seal.
  The dead data reached images through `Vm::bytes()` round trips and session_bytes
  without a collection, and through the garbage old stack a grow leaves behind
  (found by the new test: uninterrupted vs restored-every-997-fuel uncollected images
  differed in 36 bytes of that cell). Fixed: `Vm::image()` (zeroes; image writers use it),
  embedder collections zero, `state::move_stack` (grow) zeroes the old stack (interp/
  edit agreed with the coordinator; bloat informed).
- `djs heap` on a REPL snapshot and pi idle snapshots: the only dead per-VM scratch is
  the value stack's capacity after a deep recursion (32 KB in the REPL after deep(300),
  256 KB in pi after deep(2000)). Embedder full collections now shrink a grown stack
  (not seals: D2057's order). Sizes:
  | | session image before | after | container before | after |
  | --- | ---: | ---: | ---: | ---: |
  | pi idle after deep(2000) | 459,976 | 199,880 | 41,070 | 41,076 |
  | pi idle, plain | 200,024 | 200,024 | 41,077 | 41,072 |
  | REPL after deep(300) | 309,560 | 278,840 | 37,329 | 37,405 |
  (REPL raw image 597,600 → 566,880.) Containers are unchanged within ±0.2%: zstd had
  already compressed the dead slots.
- Size: +91 B gzip (454,862, ceiling 454,863): the first version was +386; a shared
  `move_stack` for grow and tidy, zeroing with a memset, and no shrink in seals got it
  under.
- Workloads (wasm instructions, n=3; strings/props rechecked at n=5): −2.0% to +0.01%
  (props 0.0%, strings −0.2% at n=5).
- Tests: vm::tests::images_have_nothing_above_sp (dead slots in image(), session images,
  equal images across slicing, canonical_hash, shrink and regrow); slices.rs compares
  `image()`s; difftest --determinism hashes `image()`. Gate-equivalents run locally:
  difftest --ratchet --determinism, js tests, native-vs-wasm, e2e modes.
- Guide: snapshots.md ("An image is the arena": dead slots, the stack shrink).

## Follow-ups
- A stack that grew before a seal is old; the first full collection after it frees it and
  shifts the sealed image once (D2308's known gap). Shrinking in the seal itself, before
  `move_to_end`, cost ~70 B gzip more than the ratchet allowed.
- The size ceiling has 1 byte of headroom.

## Result
scripts/perf/instr.py works on main again. Its patches are anchored and checked, it has a
self-test, and it splits interpreter work and time between user code and bytecode
builtins (D2309). Images carry no dead value-stack slots (`Vm::image()`, embedder
collections, zeroing on stack growth), and embedder collections shrink a grown stack
(D2308): pi's session image after deep recursion drops from 460 to 200 KB. No runtime
cost; +91 B gzip.
