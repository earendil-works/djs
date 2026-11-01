# 0015: P0 interpreter and embedding API

status: open
agent: -
branch: issue/0015-p0-interpreter
depends-on: 0013
touches: crates/djs-core/src/{interp,vm}/, crates/djs-core/src/value.rs + heap/gc + heap/walk (only for the D007 tag), crates/djs-cli/

## Goal
docs/compiler.md section 10, row 0006c, and section 7: the value stack and frames in the
arena, the dispatch loop with fuel, the `Interp` cell, globals, `console.log` as a sync
`HostCall`, and the embedding API (`vm_new`, `vm_compile`, `vm_call`, `vm_run` per
ARCH "Execution API"), plus `djs run file.js`. Can start in parallel with 0014 using
hand-assembled bytecode from 0013's emitter.

## Acceptance criteria
- [ ] Every P0 opcode is executed. Operators follow spec semantics for primitives (ToNumber
      and ToString via `num`; string concatenation producing ropes; equality and
      relational comparisons, typeof, …). Objects as operands of operators that would need
      ToPrimitive throw a clear internal "not supported yet" TypeError for now (P1-2
      adds the privileged slow paths).
- [ ] Fuel per instruction (cost classes from the table). `run(fuel)` returns `Done`,
      `Threw`, `OutOfFuel` or `HostCall`. Resuming after `OutOfFuel` continues exactly
      where it stopped.
- [ ] `console.log(...)` is a sync host call: the frame is parked, the host formats the
      arguments (simple formatting, D1100-ish: strings raw, others in a basic form;
      `djs run` prints), then `vm_resolve_host_call` and `run` continue.
- [ ] Safepoints: `collect()` is called at safepoints when `gc_pending` is set, and all live
      values are in the arena (value stack, frames, roots). A `--gc-stress` option (collect
      at every safepoint) runs the tests too.
- [ ] Suspension stress mode, trivial version: `djs run --suspend-every N` runs with fuel
      N, and on `OutOfFuel` it copies the heap into a fresh `Heap::from_bytes` and
      continues there. Output is identical. (This exercises the "no host pointers"
      property for real.)
- [ ] D007's internal-ref tag, if the frame or template design needs it here.
- [ ] Tests: interpreter tests on hand-assembled bytecode, and end-to-end tests through
      the compiler once 0014 has landed (rebase onto it then).

## Log

## Follow-ups

## Result
