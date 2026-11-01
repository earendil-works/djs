# 0086: Bytecode quality and dispatch (0081 proposal 4)

status: open
agent: -
branch: issue/0086-bytecode-quality
depends-on: 0081, 0084
touches: crates/djs-core/src/compiler/ (peepholes, loop layout), crates/djs-core/src/bytecode/ (new ops, renumbering), crates/djs-core/src/interp/ (fast_step arms for the new/common ops; coordinate with heap/infra), docs/compiler.md

## Goal
0081 F2/F7: the compiler emits ~1.5x qjs's bytecode count (`i++` as a statement is 6 ops,
3 jumps per loop iteration, set+drop / insert2+drop for assignment statements), common
prelude ops run on the slow path (`get_cap`, `strict_eq`, `not`, `null`, `push_false`:
90–190 instructions instead of ~45), and the wasm opcode switch is a compare tree plus five
br_tables. Fix the cheap parts.

## Acceptance criteria
- [ ] Peepholes: `inc_loc`/`dec_loc`, assignment statements without set+drop, the loop
      condition at the bottom (one jump per iteration), cheaper `insert2` uses.
- [ ] More simple ops on the fast path (fast_step).
- [ ] The fast opcodes renumbered into one dense range, so wasm gets a single br_table
      (check the wasm code). The OPS_HASH and build hash change.
- [ ] Bytecode counts (scripts/perf/instr.py) and instruction counts (wasm primary)
      before/after on the 0081 workloads; ratchets, stress modes and determinism
      checks identical.

## Log

## Follow-ups

## Result
