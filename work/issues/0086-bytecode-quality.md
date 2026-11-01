# 0086: Bytecode quality and dispatch (0081 proposal 4)

status: in-progress
agent: lexer
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
- [x] Peepholes: `inc_loc`/`dec_loc`, assignment statements without set+drop, the loop
      condition at the bottom (one jump per iteration), cheaper `insert2` uses.
- [x] More simple ops on the fast path (fast_step).
- [ ] The fast opcodes renumbered into one dense range, so wasm gets a single br_table
      (check the wasm code). The OPS_HASH and build hash change.
- [x] Bytecode counts (scripts/perf/instr.py) and instruction counts (wasm primary)
      before/after on the 0081 workloads; ratchets, stress modes and determinism
      checks identical.

## Log
- Peepholes (D1860, docs/compiler.md 5.10): compiler/peephole.rs on phase-1 code (drops of
  expression statements: `name_set; drop`, `insertN ... put_*; drop`, `dup; inc; put;
  drop`; `to_numeric; inc`; `not; jump`; `push_true; jump`; jump threading, jump to
  return, `&&`/`||` conditions with new labels), and fusions in the re-emit
  (resolve.rs: `inc_loc`/`dec_loc`, put+get as `set_loc`, `=== undefined` as
  `is_undefined`). `i += 1` stays an `add` (strings).
- Loop rotation for `while` and C-style `for`: parts emitted in source order, then the
  phase-1 byte ranges move (`CodeBuf::rotate`, `Parser::rotate_code` keeps the line
  entries in effect per block). One conditional jump per iteration.
- `inc_loc`/`dec_loc` ops; an object local goes through `%ToNumeric` into the local and
  re-executes. That exposed a latent bug: re-executing slow paths used `ret_pc - 1` as
  the caller's handler key and line, which is the previous instruction: a throwing
  valueOf in the first instruction of a `try` escaped it. New `Continuation::Retry`
  (D1861) for the re-executing slow paths.
- `fast_more` (out of line, after `fast_step`): constants, push_const, special_object
  this, swap/dup2/nip/insert2, get/put_cap, get/put_box, not, strict_eq/ne,
  is_undefined/is_nullish, int mod and bitops. As arms inside `fast_step` they cost the
  hot loop 3-6% in wasm (A/B: tight +6.4%, prop1 +5.4%), so they live behind it.
- `a.length` of an Array / string in `fast_field` (arrays -11%).
- Bytecodes executed (instr.py, user+prelude, base 5459e97 → now): tight 180.0M → 90.0M,
  loop 153.0M → 90.0M, pi-like 67.4M → 61.0M (slow path 13.1M → 4.9M), props 54.0M →
  39.0M, calls 39.0M → 29.9M, arrays 121.2M → 79.1M, strbuild 9.2M → 5.7M, forof 44.0M →
  38.4M, call1 69.0M → 42.0M, prop1 57.0M → 30.0M, strings 18.5M → 15.7M.
- wasm instructions (wmeasure.py, node, minus empty; base → now; qjs-wasi): tight 14141 →
  8748 (qjs 15657), loop 13316 → 8627 (13228), pi-like 25813 → 24777 (7304), props 10563
  → 9720 (9452), calls 6056 → 5056 (4059), arrays 27658 → 22346 (16809), strbuild 5815 →
  5544 (3204), forof 10039 → 9038 (7140), jsonh 2161 → 2178 (3567), higher-order 18602 →
  18199 (4753), call1 7617 → 5917 (6984), prop1 4761 → 3180 (5510), json 1335 → 1344
  (2182), strings 8500 → 8316 (5982). wasm +9 KB (fast_more 3.3 KB, compiler ~4 KB).
- scripts/perf/instr.py patches the new `fast_step() || fast_more()` line.
- Not done here: the dense opcode renumbering (a separate small merge the coordinator
  schedules, it changes the `ops!` order other branches depend on).

## Follow-ups
- Dense opcode renumbering (acceptance item 3), separate merge.
- `add_loc` (`s += x` with a simple right side: `get_loc x; add_loc s`) would save two
  ops per accumulation; its slow path needs a generic `%Add` call into the local
  (object operands), so it was left out.
- `post_inc_loc` for `a[i++]` in expressions (still `get_loc; to_numeric; dup; inc;
  put_loc`).
- `jump_if_x L; jump L` (both to the same label after threading) could be `drop; jump L`.
- `fast_more` for the remaining slow ops costs every general-path op one extra call; the
  run-loop restructuring (0081 P6, state in locals) is the real fix for D1352's fragility.

## Result
