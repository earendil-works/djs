# 0015: P0 interpreter and embedding API

status: done
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
- [x] Every P0 opcode is executed. Operators follow spec semantics for primitives (ToNumber
      and ToString via `num`; string concatenation producing ropes; equality and
      relational comparisons, typeof, …). Objects as operands of operators that would need
      ToPrimitive throw a clear internal "not supported yet" TypeError for now (P1-2
      adds the privileged slow paths).
- [x] Fuel per instruction (cost classes from the table). `run(fuel)` returns `Done`,
      `Threw`, `OutOfFuel` or `HostCall`. Resuming after `OutOfFuel` continues exactly
      where it stopped.
- [x] `console.log(...)` is a sync host call: the frame is parked, the host formats the
      arguments (simple formatting, D1100-ish: strings raw, others in a basic form;
      `djs run` prints), then `vm_resolve_host_call` and `run` continue.
- [x] Safepoints: `collect()` is called at safepoints when `gc_pending` is set, and all live
      values are in the arena (value stack, frames, roots). A `--gc-stress` option (collect
      at every safepoint) runs the tests too.
- [x] Suspension stress mode, trivial version: `djs run --suspend-every N` runs with fuel
      N, and on `OutOfFuel` it copies the heap into a fresh `Heap::from_bytes` and
      continues there. Output is identical. (This exercises the "no host pointers"
      property for real.)
- [x] D007's internal-ref tag, if the frame or template design needs it here.
- [ ] Tests: interpreter tests on hand-assembled bytecode (done), and end-to-end tests through
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

- D007 tag implemented: tag 13, `Value::internal` / `as_internal`, `Kind::Internal`. The
  GC forwards it like every ref tag (test: an internal ref in an object slot moves with
  its cell). `verify` accepts it for non-JS cells only. JSON stringify treats it as `Slow`.
- Rebased onto 0013 (96c4a0a). The cells conflict was additive.
- `libm` added with the coordinator's approval (D1150); `pow` handles the JS special
  cases first.
- Dispatch loop (`interp/run.rs`) done for:
  - every P0 op;
  - most P1 ops that need no builtins: args, Boxes, captures, `fclosure`,
    `define_field`, `array_push`/`array_hole`, `concat`, `gosub`/`ret`, `throw` with
    handler tables;
  - getters and setters through `PushResult` / `DiscardResult` continuations, including
    host functions as accessors.
  Ops still missing (`construct`, `special_object`, spreads, iterators, classes, ...)
  throw `InternalError: not supported yet: <op>`. ToPrimitive on objects throws
  `TypeError: not supported yet: ...` until P1-2.
- Debug check: after each instruction that completes in the same frame, `sp` must have
  moved by the table's stack effect (and `pc` by the size for non-jumps). Verified that it
  fires by breaking `nip` on purpose.
- Tests (`vm/tests.rs`, hand-assembled with `CodeBuf`): arithmetic, an operator table,
  strings and ropes, loops with fuel slices, console.log host calls, host call results
  and errors (Ok, Err caught by a handler, the entry host call), qjs-worded runtime
  errors, objects and globals, JS calls with argument padding, closures sharing Boxes
  across collections, accessors as JS closures, exceptions unwinding frames,
  `finally` via gosub, stack overflow, garbage collection while running, an image taken
  while parked and resolved in a restored VM. Every `run_main` case runs three ways and
  must agree: plenty of fuel; fuel 1 with a heap-image round trip at every suspension and
  host call; GC stress.
- Decisions D1150 (libm), D1151 (interpreter state in the arena, 6-slot frame header),
  D1152 (host call protocol, fuel and safepoints).
- `djs-cli/src/run.rs`: the `djs run` driver (`--gc-stress`, `--suspend-every N`). Wiring
  it into `main.rs` (and `Vm::compile`) waits for 0014.

## Follow-ups
Open parts of this issue, moved to a new issue by the coordinator (to do right after
0014 lands):
- `Vm::compile(src, opts)` on top of 0014's `compiler::compile` (+ `instantiate`).
- Wire `djs run file.js` (`--gc-stress`, `--suspend-every N`) into djs-cli `main.rs`;
  the driver is in `crates/djs-cli/src/run.rs`, not yet a module of the binary.
- End-to-end tests through the compiler, in the same three modes as the hand-assembled
  tests (plain, fuel 1 + image round trip, GC stress), and `djs run` on them.

Other follow-ups:
- `construct`, `special_object`, spreads, iterators, classes, `call_intrinsic`: P1
  issues (they throw `not supported yet` now).
- ToPrimitive on objects (`%ToPrimitive` slow paths) and `get_method` through a getter:
  P1-2.
- JSON stringify should treat callable objects (`Function`, `HostFunction`) as
  `undefined` (Skip) instead of `Slow` now that those classes exist.
- Function objects have no `name`/`length`/`prototype` properties yet (P1-1). Error
  objects have no `stack` (P1-3).
- The pending host call table is a linear list; fine for sync calls, revisit for async
  host calls (P2).

## Result
Landed (everything in the arena, so a VM is fully described by its heap image):
- **D007 tag** (`value.rs`): `Value::internal(cell)`, `as_internal`, `Kind::Internal`,
  traced by the GC, checked by `verify`.
- **Cells** (`heap/cells.rs`, additive): `Interp` (17), `Realm` (18), `HostCall` (19);
  `Roots::INTERP`. Object classes `Function`, `HostFunction`, `Error` (`heap/object.rs`).
- **`interp/`**: frame layout (6-slot header, D1151), realm with global object, Object,
  Function, Array and Error prototypes, Error creation with qjs messages, primitive
  conversions and operators (`conv.rs`, `**` via libm, D1150), and the dispatch loop
  (`run.rs`). The loop covers every P0 op plus closures, Boxes, captures, accessors
  through continuations, throw with handler tables and gosub/ret. It also has fuel per
  cost class, GC safepoints and the debug table-agreement check.
- **`vm/`**: the embedding API.
  - `Vm::new`, `Vm::from_bytes` / `bytes` (resume anywhere, including while parked).
  - `instantiate(template)`, `call`, `run(fuel) -> Status` (Done / Threw / OutOfFuel /
    HostCall / Idle).
  - `host_call(id)`, `resolve_host_call(id, Ok | Err)`.
  - `new_host_function`, `define_global`, `define`, `new_object`, `new_error`,
    `display`.
- **Tests**: 16 interpreter tests on hand-assembled bytecode, each `run_main` case
  checked in three modes (plain, fuel 1 with an image round trip at every suspension and
  host call, GC stress), plus conversion unit tests.
- **`djs run` driver**: `crates/djs-cli/src/run.rs`, not yet wired into the binary.

Still open (see Follow-ups): `Vm::compile`, wiring `djs run` into `main.rs`, and the
end-to-end tests through the compiler.
