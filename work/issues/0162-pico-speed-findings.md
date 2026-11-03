# 0162: Act on pico's speed findings

status: in-progress
agent: speed
branch: issue/0162-pico-speed
depends-on: 0152, 0154, 0151 L4 (merged first)
guide: docs/guide/interpreter.md / builtins.md as affected
touches: crates/djs-core/src/interp/ (run_hot callback returns, fuel accounting), crates/djs-core/src/builtins/ (reduce and the callback builtins' TS), crates/djs-core/src/vm/ (host result path into heap strings); perf (0159) prunes @native/@fast twins in builtins/, bloat (0151 L3) is in compiler/ heap/: coordinate via the coordinator

## Goal
docs/lessons-from-pico.md and 0154's follow-ups, the speed items:
1. **pico's tree benchmark** (children + map/reduce): 665 ms vs quickjs-wasi 310. 0154 left:
   an argument-count intrinsic so `reduce` needs no rest array and second frame; callback
   returns on short arrays handled inside run_hot (after L4's encoding is on main); the
   collector's cost on long-lived trees (measure what share remains after the first two).
2. **Fuel only at calls and backward jumps** (pico's rule, lesson 7): measure what charging
   fuel per instruction costs today, and whether charging at calls/backward jumps (with a
   per-block bound so determinism and the suspend-anywhere guarantee hold, G5) pays. Fuel
   counts are part of determinism (the same history gives the same counts), so the rule
   change must be global and recorded.
3. **Large results straight into their final heap string** (lesson 8): host results and
   large JSON/string results skip the staging buffer.
Use scripts/perf/instr.py (D2309) for splits; wasm instructions on all workloads, the pi
bench and pico's rewritten cases (work/scratch/0152, 0154/w). No workload slower than
+1%; size ratchet respected or trades recorded.

## Log

## Follow-ups

## Result
