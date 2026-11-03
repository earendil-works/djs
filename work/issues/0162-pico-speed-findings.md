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
- Scratch: work/scratch/0162 (wbuild.sh builds the perf wasm into a file; hoststr.ts times
  host string results; fuel-safepoint.diff the item 2 experiment; fuel2.txt its numbers).
- Item 2 (D2550): experiment build charging fuel only at calls, returns and taken backward
  jumps in run_hot. ≤7% fewer wasm instructions on interpreter-bound loops (loop, tight,
  prop1), 2–4% on calls/arrays/foreach, ~0 on pi-like, JSON, strings. Coordinator: not now
  (it would change G5 and every fuel count); recorded with the full table.
- Item 3 (D2551): `vm_string_reserve` + `stringInPlace` (js/src/wire.ts) for host strings
  ≥ 1024 units whose wire encoding shows ASCII: copied from the wire bytes straight into
  the reserved Str8, no staging, no transcode. 10 MiB result: 21.4 → 11.4 MiB wasm memory,
  ~1.5–2x faster; non-ASCII unchanged (encoding into the reservation first cost them
  +0.7 ms/MiB and +1.3 MiB, dropped). Size +69 B gzip (455,571, ceiling 455,600). Tests:
  djs-wasm `strings_written_in_place`, js sandbox test for the in-place and wire cases.
  Guide: host.md ("Values in") updated; docs/host.md export table.

## Follow-ups

## Result
