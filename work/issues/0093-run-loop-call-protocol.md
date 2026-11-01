# 0093: Run-loop state in locals and a compact call protocol (0081 P6 + P9)

status: open
agent: -
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
- [ ] **Design first** (a short note): what lives in locals (pc, sp, fuel, lp, the code and
      stack slices or their base offsets), when it is written back (calls, allocation,
      GC safepoints, suspension: the "suspend anywhere" and image guarantees must hold
      exactly), how the 0062 ICs, native frames (0063, D1631) and @fast entries (0085)
      interact, the frame header layout for a compact call protocol, and the expected
      gains. **Send it to the coordinator before implementing** (heap's 0083 shapes and
      infra's 0090 touch interp at the same time; we'll sequence the merges).
- [ ] Implementation in parts with wasm instruction counts (primary) and native, the run
      loop's wasm function size (it must still tier up: 0081's caveat about one huge
      vm_run slice), and identical ratchets, stress modes, and determinism checks.

## Log

## Follow-ups

## Result
