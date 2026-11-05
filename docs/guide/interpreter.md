# The interpreter: frames in the heap

The interpreter runs bytecode, and its shape is dictated by the goal to [suspend
anywhere](goals.md#suspend-anywhere): execution must be able to stop between any two
instructions and continue later, possibly in another process. An interpreter that calls
itself recursively for every JavaScript call keeps half of the program's state on the
Rust stack, where no snapshot can reach it. djs's interpreter never does. This page
shows how calls, getters, conversions and exceptions work under that rule, and how the
interpreter stays fast anyway.

> **In short.** The interpreter is one loop. A JavaScript call pushes a frame record
> onto a value stack that lives in the arena and continues the loop; a return pops it.
> An instruction that needs to run user code (a getter, `valueOf`, an iterator) pushes a
> frame for it with a fixed *continuation* that says what to do with the result, and
> finishes; it never resumes half-way. So the Rust stack is the same depth whatever
> JavaScript does, and the arena alone describes where execution is.

## One loop

In outline, `run` is this:

```text
loop {
    if fuel == 0 { save registers to the arena; return OutOfFuel }
    decode the instruction at pc; execute it; fuel -= its cost
}
```

The registers are a handful of indexes: `bp`, `hp` and `sp` into the value stack, and
`pc` into the current function's bytecode. Whenever `run` returns, for whatever reason,
they are written into the `Interp` cell of the arena (D1151). Fuel, and what running out
of it means, is the subject of [the next chapter](suspension.md).

## Frames on the value stack

The value stack is a growable array of values in the arena (`Roots::STACK`), addressed
by index, so growing it moves nothing the interpreter holds. A call frame is a stretch
of it (D1086):

```text
bp      callee            the function object
bp+1    this
bp+2    arg0 .. arg(n-1)  n = max(argc, declared parameters), padded with undefined
hp      prev_bp           int32
hp+1    prev_hp           int32
hp+2    ret_pc            int32: the caller's pc to resume
hp+3    argc              int32: actual argument count
hp+4    flags             int32: continuation | construct bit | native bit
hp+5    new_target        Value
lp      locals            lp = hp + 6, filled with undefined
sb      operand stack     sb = lp + locals
```

A call instruction finds the callee, `this` and the arguments already on the operand
stack in exactly this order, so they *become* the new frame's first slots with no
copying. The call writes the six header words, fills the locals with `undefined`, and
jumps to the callee's first instruction. A return reads the header, puts the result
where the continuation says, and restores the caller's registers.

Every header word is an ordinary `Value` (the indexes are int32 values), so the
collector traces the stack as a plain array of values, and a snapshot needs no frame
serializer: the stack is just a cell.

```js live
function depth(n) { return n === 0 ? 0 : 1 + depth(n - 1); }
console.log(depth(10000));
```
<!-- gen:run -->
```text
10000
```
<!-- /gen -->

Ten thousand nested JavaScript calls use ten thousand frames in the arena and one Rust
stack frame. The limit is the size of the value stack, 2²⁰ slots by default; beyond it a
call throws `RangeError: Maximum call stack size exceeded`, the message pi's QuickJS
build and V8 give (D013).

> **Trade-off.** A stackless interpreter over a recursive one. A recursive interpreter
> gets its frames for free from the native stack and can call back into JavaScript from
> anywhere in Rust. djs writes every frame into the arena instead, a few stores per
> call, and gives up calling JavaScript from Rust entirely (next section). What it gets
> is that any instruction boundary is a complete, copyable state: suspension, snapshots
> and fuel limits cost nothing extra.

## Instructions that call user code

Many instructions can run user code: `get_field` on a getter, `+` on an object with
`valueOf`, `for-of` on a user iterator, `instanceof` with `Symbol.hasInstance`, anything
on a Proxy. A recursive interpreter would call the user function from inside the
instruction and wait. djs's rule is that **an instruction never resumes in the middle**
(D1085):

1. It runs its fast path, in plain Rust that runs no user code.
2. If user code is needed, it pushes a frame for the callee, with a continuation in the
   frame's flags, and is finished.
3. When that frame returns, the continuation completes the instruction's job.

There are only a few continuations:

| continuation | on return |
| --- | --- |
| `PushResult` | push the value on the caller's operand stack (calls, getters) |
| `DiscardResult` | drop it (setters) |
| `ConstructResult` | use it if it is an object, else the frame's `this` (constructors) |
| `Entry` | the bottom frame of the embedder's call (`Vm::call`): `run` returns `Done` |
| `Job` | the bottom frame of a promise job: drop it and run the next job |
| `Replace(slot)` | store it in a stack slot of the caller and continue after the instruction |
| `Retry(slot)` | store it in a stack slot and run the same instruction again |
| `Each` | a callback of `%EachDense`: complete the loop's step, then enter the same frame again for the next element, or give the builtin the index its own loop goes on from |

`Retry` is how conversions work (D1280, D1861). When `+` finds an object operand, it
calls the builtin `%ToPrimitive` on it with `Retry` aimed at the operand's own stack
slot. When `%ToPrimitive` returns, the primitive replaces the object and the `+` runs
again from the start, this time on primitives. The multi-step logic (look up
`Symbol.toPrimitive`, then `valueOf`, then `toString`) lives in a builtin written in
JavaScript-like TypeScript, where suspending is free, and the interpreter has no per-op
state machines.

```js live
const price = { valueOf() { console.log("valueOf runs"); return 40; } };
const order = { get total() { console.log("getter runs"); return price + 2; } };
console.log(order.total);
```
<!-- gen:run -->
```text
getter runs
valueOf runs
42
```
<!-- /gen -->

Here `get_field total` pushes a frame for the getter with `PushResult`; inside it, `add`
pushes a frame for `%ToPrimitive` with `Retry`, which calls `valueOf` with `PushResult`.
Three frames deep, and the Rust stack never grew. Each of those calls is an instruction
boundary at which the VM could have stopped and been snapshotted.

> **Trade-off.** Re-entry through frames over per-op resume states. The original design
> allowed an instruction to keep "a small numbered resume state" so it could pick up in
> the middle. Every such instruction would have needed its state saved somewhere a
> snapshot can see, and a test for every state. Pushing a frame and finishing keeps the
> interpreter free of that. The cost is speed on the slow path: an object operand to `+`
> takes a call into a builtin and a second execution of the instruction, which is fine
> because the fast path covers the common cases.

## Exceptions

Each function template has a **handler table**: `(start, end, handler, depth)` entries,
innermost first. A throw at `pc` looks for the first entry covering `pc`. If there is
one, the operand stack is cut to `depth`, the exception is pushed, and execution jumps
to the handler. If not, the frame is popped and the search continues in the caller, at
the call instruction. An `Entry` frame without a handler makes `run` return
`Threw(value)`.

`finally` blocks are compiled once and entered with a plain jump (D2733). Normal exit,
`return`, `break`, `continue` and a thrown exception all reach the same code after storing
their **completion** in two hidden locals: a kind (0 normal, 1 throw, 2 return, or a
number naming the statement a `break` or `continue` goes to) and a value (the exception
or the return value). The end of the block looks at the kind and goes on: past the
`try`, rethrowing, returning, or jumping, through the next finally block out if there is
one:

```js
let log = [];
try {
  log.push("try");
} finally {
  log.push("finally");
}
return log;
```

<!-- gen:disasm -->
```text
function 0 <anonymous>: params=0 locals=4 stack=3 captures=0
  handler [7, 19) -> 40 depth 0
  handler [3, 49) -> 49 depth 0
    0  async_start loc0 (<state>)
    3  array_from 0
    6  put_loc1 (log)
    7  get_loc1 (log)  ; 3:3
    8  get_method "push" ic0  ; 3:6
   15  push_const8 #0 ("try")
   17  call1  ; 3:11
   18  drop
   19  push0
   20  put_loc2 (<temp>)
   21  get_loc1 (log)  ; 5:3
   22  get_method "push" ic1  ; 5:6
   29  push_const8 #1 ("finally")
   31  call1  ; 5:11
   32  drop
   33  get_loc2 (<temp>)
   34  jump_if_false8 -> 45
   36  get_loc3 (<temp>)
   37  undefined
   38  put_loc3 (<temp>)
   39  throw
   40  put_loc3 (<temp>)
   41  push1
   42  put_loc2 (<temp>)
   43  jump8 -> 21
   45  get_loc1 (log)  ; 7:8
   46  async_return loc0 (<state>)
   49  async_reject loc0 (<state>)
```
<!-- /gen -->

The completion is in locals, not on the operand stack, so a function suspended inside a
`finally` block holds nothing a snapshot or a dump has to interpret: the kind is a
number that means the same in any build. Closing a `for-of` loop's iterator when a jump
or a `return` leaves the loop works the same way.

> **Trade-off.** Handler tables over a stack of catch offsets. QuickJS pushes catch
> offsets onto the value stack and scans for them while unwinding. A table keeps the
> stack plain, so the collector and the snapshot never need to know about `try`. The
> cost is a table lookup per frame while unwinding, which only happens on a throw.

## Native frames

Some builtins (`Array.prototype.push`, `sort`, `JSON.stringify`'s slow path, ...) are
compiled to Rust state machines rather than bytecode ([builtins](builtins.md)). They run in **native frames**:
the same frame records on the same value stack, with a bit in the flags, the state
number where a bytecode frame has its `pc`, and the builtin's locals in the frame's
slots (D1631, D1632). When `sort` calls its comparator, it does not call it from Rust: it
writes the callee's frame on top of its own, saves the state to resume at, and leaves.
The loop runs the callback like any other closure, and its return re-enters the native
frame at the saved state, one native run per callback call (D2060). A native builtin in
the middle of a loop is therefore as suspendable as bytecode.

`map`, `forEach`, `filter` and the other callback loops go one step further (D2303).
Their intrinsic `%EachDense` keeps the loop's state in its own arguments, below the
callback's frame, and pushes that frame once with the `Each` continuation. When the
callback returns, the hot loop completes the step (the store into `map`'s result, say)
and rewrites the same frame for the next element: arguments, undefined locals, `pc` 0.
The builtin itself only runs again when the dense part is over.

Errors carry QuickJS-style stack traces, with the function names and source positions of
the frames, read from the frame records and the templates' line tables (D1282).

## Making it fast

A straightforward loop that decodes an instruction, dispatches through a large `match`
and returns a `Result` from every arm spends more time on its own bookkeeping than on
simple instructions. djs keeps the general loop for the full semantics and puts a faster
one in front of it:

- **The tier-0 loop** (`run_hot`, D1930) is the main loop. It keeps `pc`, `sp`, the fuel
  and the current frame's data in Rust locals and handles the hottest cases itself:
  locals, constants, int32 and float arithmetic, comparisons, jumps, calls of plain
  closures and returns (D1931), inline-cache hits and dense array elements. Anything
  else, and any case it declines (an int overflow, an object operand), it hands to the
  general path (`step_once`), which runs that one instruction in place, with identical
  fuel and safepoints, after the locals have been written back. Nothing the tier-0 loop
  does itself can collect or run user code, which is why it may hold the state in
  locals.
- **Inline caches.** Property and global accesses carry a cache slot number (the `ic0`
  in the disassembly). A slot remembers where a property was found: a shape, a table
  index, or a prototype holder validated by an epoch (D1620, D1621). A hit is a compare
  and a load; a stale slot is a miss and is refilled, so caches never change semantics,
  and a VM restored with warm caches behaves exactly like one with cold caches.
- **Iteration without result objects.** `for-of` and array destructuring over arrays,
  strings, Maps and Sets step the iterator inside the interpreter when `next` is the
  original builtin, without allocating `{value, done}` objects (`iter_next`, D1800); a
  loop keeps its iterator and `next` in two locals, which `iter_next_loc` reads in the
  same instruction (D2732).

Every one of these was measured, in wasm under V8, before it was kept: the register
allocation of a dispatch loop is fragile, and an extra arm can make the hottest
instructions slower (D1352). One rule came out of reading V8's machine code for the
loop: nothing computed for every instruction (the instruction word, an immediate, the
opcode) may stay alive across a heap call in any arm. V8 spills such a value where it is
defined, so every instruction would pay a store. The arms therefore extract their own
immediates and read the opcode again after a call (D2441).

## Where to look

- [`crates/djs-core/src/interp/frame.rs`](../../crates/djs-core/src/interp/frame.rs):
  the frame layout and the continuations.
- [`crates/djs-core/src/interp/run.rs`](../../crates/djs-core/src/interp/run.rs): the
  run loop and `step`; [`run/hot.rs`](../../crates/djs-core/src/interp/run/hot.rs): the
  tier-0 loop; [`run/ic.rs`](../../crates/djs-core/src/interp/run/ic.rs): inline caches.
- [`crates/djs-core/src/bytecode/ops.rs`](../../crates/djs-core/src/bytecode/ops.rs):
  every instruction's cost and stack effect ([reference](ref-opcodes.md)).

Decisions on this page:

<!-- gen:decisions D1151 D1086 D013 D1085 D1280 D1861 D1084 D1250 D1282 D1930 D1931 D1631 D1632 D2060 D1620 D1621 D1800 D1352 D2441 -->
- D1151: Interpreter state in the arena
- D1086: Frame records in the index-addressed value stack
- D013: Stack overflow is a RangeError
- D1085: Re-entrant ops finish immediately, slow paths call privileged JS
- D1280: Slow paths replace an operand and re-execute the instruction
- D1861: The Retry continuation
- D1084: Exception handler tables and gosub/ret for finally
- D1250: try/catch/finally compilation
- D1282: Error stack traces
- D1930: The tier-0 loop keeps the interpreter state in locals
- D1931: Calls and returns in the tier-0 loop: the same frames, block access
- D1631: Native frames are ordinary frame records with a header bit
- D1632: The bgen Rust backend: states are basic blocks, calls go through call_intrinsic
- D2060: The base is decoded or injected straight into the first VM's arena
- D1620: Inline caches: verified table indexes in a per-template array
- D1621: Prototype property caches: a receiver-prototype offset, an epoch, and GC clearing
- D1800: iter_next: iteration steps without result objects
- D1352: more fast paths: leaf intrinsics, floats, dense elements, own-property writes
- D2441: The hot loop's dispatch keeps no per-instruction values across calls
<!-- /gen -->
