# 0015: P0 interpreter and embedding API

status: in-progress
agent: heap
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
- 2026-11-01: Read docs/compiler.md (4, 6, 7, 10), D1080-D1086, D006-D009, D1100, and the
  0013 branch (read-only). 0013 defines cell kinds 12-16 (`FuncTemplate`, `Script`,
  `BoxCell`, `Bytes`, `RefArray`), the `ops!` table with `Op`, `OPS` (cost classes,
  flags), `CodeBuf` and raw `read_*` helpers.

  **Plan.**
  1. *D007 tag* (before 0013): tag 13 `INTERNAL` in `value.rs`, a reference to a non-JS
     cell (templates, Boxes, realm, host calls). The GC already forwards every ref tag.
     `verify` accepts it for any cell kind except the JS value kinds and the roots.
     JSON stringify treats it as `Slow` (never visible to JS).
  2. *Arena state* (additive cells in `cells.rs`, kind ids after 0013's):
     - `Roots::STACK`: the value stack, a `ValueArray` addressed by index. It grows by
       copying, so registers are indexes.
     - `Roots::RESERVED` becomes `INTERP`, an `Interp` cell: state (idle / running /
       parked on a host call / done / threw), `BP`, `SP`, `PC`, the pending host call,
       the completion value.
     - `Roots::REALM` holds an internal ref to a `Realm` cell: the global object, and the
       Object, Array, Function and Error prototypes.
     - `Roots::HOST_CALLS`: a `ValueArray` of pending host calls, each a `HostCall` cell
       (id, fn id, args).
  3. *Frames* exactly as docs/compiler.md 7.1 (D1086). Header fields are int32 Values.
     `flags` holds the continuation (`PushResult`, `DiscardResult`, `ConstructResult`,
     `Entry`). `vm.call(f, this, args)` pushes an `Entry` frame and returns; `run`
     executes.
  4. *Function objects*: two new `ObjectClass`es in `heap/object.rs`, which is outside
     `touches:` but a minimal additive change:
     - `Function`: slot 0 = internal ref to the template, slots 1.. = Box refs;
     - `HostFunction`: slot 0 = int32 `fn_id`.
  5. *Host calls* (7.5): `call` on a `HostFunction` creates a `HostCall` entry, parks
     the frame with `pc` after the call, and `run` returns `HostCall { call_id, fn_id }`.
     The embedder reads `host_call_args(call_id)` and calls
     `resolve_host_call(call_id, Ok(v) | Err(e))`; then `run` continues (push or
     throw). `console.log` is just a host function registered by the embedder
     (`djs run`).
  6. *Embedding API* (`vm/`): `Vm::new(opts)`, `Vm::from_bytes(bytes)` (restore),
     `vm.bytes()` (image), `vm.instantiate(template) -> Handle` (top-level closure),
     `vm.compile(src, opts)` once 0014 lands, `vm.call`, `vm.run(fuel) -> Status`
     (`Done(Value)` / `Threw(Value)` / `OutOfFuel` / `HostCall{..}`),
     `vm.resolve_host_call`, `define_global` and `new_host_function` for the embedder,
     and basic value formatting for `djs run`.
  7. *Dispatch loop* (after 0013): registers in Rust locals; `match Op`; fuel charged
     from `OPS[op].cost`, stop when exhausted at an instruction boundary; safepoints
     (back jumps, calls, returns) collect when `gc_pending` (or always with
     `gc_stress`), then re-derive the code from `stack[bp]`. Runtime errors (TypeError,
     ReferenceError, RangeError) are Error objects from the realm prototypes, thrown
     through the handler table (P0 has no `try`, so they unwind to `Threw`). Operators
     on objects that need ToPrimitive throw `TypeError: not supported yet` until P1-2.
     A debug check after every non-jump instruction: `pc` advanced by `size`, `sp` moved
     by `push - pop`.
  8. *`djs run`*: `--fuel-per-slice`, `--gc-stress`, `--suspend-every N` (on
     `OutOfFuel`, rebuild the VM with `Vm::from_bytes(vm.bytes())` and continue).
  9. *Tests*: hand-assembled bytecode via `CodeBuf` until 0014 lands, then end-to-end
     through the compiler.

## Follow-ups

## Result
