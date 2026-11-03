//! Suspendable frames: generators and async functions (D1375, docs/compiler.md 7.7).
//!
//! A suspension copies the current frame's slice of the value stack, `stack[bp..sp)`
//! (callee, `this`, arguments, header, locals, operand stack), into an internal
//! `ValueArray` held by a Generator object (`ObjectClass::Generator`), with the pc to
//! resume at and `hp - bp`. A frame holds no absolute stack positions except its
//! header's links to its caller, so resuming copies the slice back at the top of the
//! stack and rewrites those links for the new caller (the resumer, with `PushResult`).
//!
//! Generator object slots:
//!
//! ```text
//! 0  state    int32: START (suspended before the body), SUSPENDED (at a yield or
//!             await), EXECUTING, COMPLETED
//! 1  frame    internal ValueArray while suspended, else undefined
//! 2  pc       int32: where to resume
//! 3  hp       int32: hp - bp of the frame
//! 4  promise  the result promise of an async function; undefined for generators
//! 5  kind     int32: GENERATOR, ASYNC or ASYNC_GENERATOR
//! 6  queue    an async generator's requests (internal ValueArray of Records), D1421
//! ```
//!
//! An `await` suspends with state AWAITING (resumed by a promise job, pushing the
//! value); a `yield` with SUSPENDED (resumed by a request, pushing `value isReturn`).
//! Async generators use both: their resumer (builtins/asyncgen.ts) tells them apart by
//! the state after the frame hands control back.
//!
//! `await` and the end of an async function replace the frame with a call of a builtin
//! (`%AsyncAwait`, `%AsyncFunctionReturn`, `%AsyncFunctionThrow`) with the frame's own
//! continuation: a tail call. Those return the result promise, so whoever ran the frame
//! (the caller the first time, the job that resumed it later) gets the promise.

use super::*;

pub(crate) const START: i32 = 0;
pub(crate) const SUSPENDED: i32 = 1;
pub(crate) const EXECUTING: i32 = 2;
pub(crate) const COMPLETED: i32 = 3;
pub(crate) const AWAITING: i32 = 4;
/// Suspended at a `yield_star`: the resumer passes the value on unwrapped. (5 is an
/// async generator's awaiting-return, kept by builtins/asyncgen.ts.)
pub(crate) const SUSPENDED_RAW: i32 = 6;

pub(crate) const GENERATOR: i32 = 0;
pub(crate) const ASYNC: i32 = 1;
pub(crate) const ASYNC_GENERATOR: i32 = 2;

const STATE: u32 = 0;
const FRAME: u32 = 1;
const PC: u32 = 2;
const HP: u32 = 3;
const PROMISE: u32 = 4;
const KIND: u32 = 5;
pub(crate) const QUEUE: u32 = 6;

const ASYNC_AWAIT: u16 = intrinsics::id("AsyncAwait");
const ASYNC_FUNCTION_RETURN: u16 = intrinsics::id("AsyncFunctionReturn");
const ASYNC_FUNCTION_THROW: u16 = intrinsics::id("AsyncFunctionThrow");

/// `v` as a Generator object of `kind`.
pub(crate) fn as_generator(heap: &Heap, v: Value, kind: i32) -> Option<Obj> {
    v.as_object().filter(|&o| {
        heap.object_class(o) == ObjectClass::Generator
            && heap.object_slot(o, KIND).as_int() == Some(kind)
    })
}

/// `v` as a Generator object of any of `kinds`.
fn as_generator_of(heap: &Heap, v: Value, kinds: &[i32]) -> Option<Obj> {
    kinds.iter().find_map(|&k| as_generator(heap, v, k))
}

pub(crate) fn state_of(heap: &Heap, g: Obj) -> i32 {
    heap.object_slot(g, STATE).as_int().unwrap_or(COMPLETED)
}

pub(crate) fn set_state(heap: &mut Heap, g: Obj, s: i32) {
    heap.set_object_slot(g, STATE, Value::int(s));
    if s == COMPLETED {
        // A completed activation keeps nothing alive.
        heap.set_object_slot(g, FRAME, Value::UNDEFINED);
    }
}

pub(crate) fn promise_of(heap: &Heap, a: Obj) -> Value {
    heap.object_slot(a, PROMISE)
}

fn alloc(heap: &mut Heap, proto: Option<Obj>, kind: i32) -> Result<Obj, HeapError> {
    let g = heap.alloc_object(
        ObjectClass::Generator,
        proto,
        ObjectClass::Generator.slots(),
    )?;
    for (slot, v) in [
        (STATE, Value::int(EXECUTING)),
        (FRAME, Value::UNDEFINED),
        (PC, Value::int(0)),
        (HP, Value::int(0)),
        (PROMISE, Value::UNDEFINED),
        (KIND, Value::int(kind)),
        (QUEUE, Value::UNDEFINED),
    ] {
        heap.set_object_slot(g, slot, v);
    }
    Ok(g)
}

/// Appends a request Record to async generator `g`'s queue (D1421).
pub(crate) fn enqueue_request(heap: &mut Heap, g: Obj, request: Value) -> Result<(), HeapError> {
    let q = heap.object_slot(g, QUEUE).as_internal().unwrap_or(0);
    let len = if q == 0 { 0 } else { heap.value_array_len(q) };
    let q2 = heap.value_array_reserve(q, len + 1)?;
    heap.value_array_set(q2, len, request);
    heap.arena_mut().set(q2, ValueArray::LEN, len + 1);
    heap.set_object_slot(g, QUEUE, Value::internal(q2));
    Ok(())
}

/// The oldest request of `g`, or undefined.
pub(crate) fn first_request(heap: &Heap, g: Obj) -> Value {
    match heap.object_slot(g, QUEUE).as_internal() {
        Some(q) if heap.value_array_len(q) > 0 => heap.value_array_get(q, 0),
        _ => Value::UNDEFINED,
    }
}

/// Removes and returns the oldest request of `g` (queues stay short: a shift).
pub(crate) fn dequeue_request(heap: &mut Heap, g: Obj) -> Value {
    let Some(q) = heap.object_slot(g, QUEUE).as_internal() else {
        return Value::UNDEFINED;
    };
    let len = heap.value_array_len(q);
    if len == 0 {
        return Value::UNDEFINED;
    }
    let first = heap.value_array_get(q, 0);
    for i in 1..len {
        let v = heap.value_array_get(q, i);
        heap.value_array_set(q, i - 1, v);
    }
    heap.value_array_set(q, len - 1, Value::UNDEFINED);
    heap.arena_mut().set(q, ValueArray::LEN, len - 1);
    first
}

impl Exec<'_> {
    /// The Generator object in local `loc` of the current frame.
    fn local_generator(&self, loc: u32, kinds: &[i32]) -> Result<Obj, Fault> {
        as_generator_of(self.heap, self.get(self.lp + loc), kinds).ok_or_else(|| {
            Fault::error(
                ErrorKind::InternalError,
                String::from("bad bytecode: no generator state"),
            )
        })
    }

    /// Saves the frame `stack[bp..sp)` into `g`, to resume at `pc`.
    fn suspend_frame(&mut self, g: Obj, state: i32, pc: u32) -> Result<(), Fault> {
        let len = self.sp - self.bp;
        let arr = self.heap.alloc_value_array(len)?;
        for i in 0..len {
            let v = self.get(self.bp + i);
            self.heap.value_array_set(arr, i, v);
        }
        self.heap.arena_mut().set(arr, ValueArray::LEN, len);
        self.fuel -= (len / 8) as i64;
        for (slot, v) in [
            (STATE, Value::int(state)),
            (FRAME, Value::internal(arr)),
            (PC, Value::int(pc as i32)),
            (HP, Value::int((self.hp - self.bp) as i32)),
        ] {
            self.heap.set_object_slot(g, slot, v);
        }
        Ok(())
    }

    /// Replaces the current frame by a call of the installed builtin `id` with `args`,
    /// returning to the frame's own continuation.
    fn tail_call_builtin(&mut self, id: u16, args: [Value; 2]) -> Step {
        let f = realm::installed_intrinsic(self.heap, id);
        let cont = Continuation::from_flags(self.header_int(frame::FLAGS) as i32);
        let ret_pc = self.header_int(frame::RET_PC);
        let base = self.bp;
        if !cont.is_bottom() {
            let (prev_bp, prev_hp) = (
                self.header_int(frame::PREV_BP),
                self.header_int(frame::PREV_HP),
            );
            self.bp = prev_bp;
            self.hp = prev_hp;
        }
        self.sp = base;
        self.push(f);
        self.push(Value::UNDEFINED);
        for a in args {
            self.push(a);
        }
        self.call(base, 2, cont, ret_pc)
    }

    /// `generator_start loc`.
    pub(super) fn generator_start(&mut self, next: u32) -> Step {
        let loc = self.imm_u16(1);
        // An async generator if the function is also ASYNC (D1421). The prototype: the
        // function's own `prototype` if it is an object (a data property of generator
        // functions), else %GeneratorPrototype% / %AsyncGeneratorPrototype%.
        let callee = self.get(self.bp).as_object().expect("frame callee");
        let flags = u32::from(
            self.heap
                .arena()
                .get::<u16>(self.template, FuncTemplate::FLAGS),
        );
        let (kind, fallback) = if flags & func_flags::ASYNC != 0 {
            (ASYNC_GENERATOR, Intrinsic::AsyncGeneratorProto)
        } else {
            (GENERATOR, Intrinsic::GeneratorProto)
        };
        let proto = self
            .heap
            .get_own_property(callee, crate::heap::atoms::PROTOTYPE)
            .and_then(|p| p.value())
            .and_then(|v| v.as_object())
            .unwrap_or_else(|| intrinsic_obj(self.heap, fallback));
        let g = alloc(self.heap, Some(proto), kind)?;
        self.set(self.lp + loc, Value::object(g));
        self.suspend_frame(g, START, next)?;
        self.do_return(Value::object(g))
    }

    /// `yield loc`.
    pub(super) fn yield_value(&mut self, next: u32, raw: bool) -> Step {
        let kinds: &[i32] = if raw {
            &[GENERATOR]
        } else {
            &[GENERATOR, ASYNC_GENERATOR]
        };
        let g = self.local_generator(self.imm_u16(1), kinds)?;
        let v = self.pop();
        let state = if raw { SUSPENDED_RAW } else { SUSPENDED };
        self.suspend_frame(g, state, next)?;
        self.do_return(v)
    }

    /// `async_start loc`.
    pub(super) fn async_start(&mut self) -> Result<(), Fault> {
        let loc = self.imm_u16(1);
        let a = alloc(self.heap, None, ASYNC)?;
        let proto = intrinsic_obj(self.heap, Intrinsic::PromiseProto);
        let p = promise::alloc(self.heap, Some(proto))?;
        self.heap.set_object_slot(a, PROMISE, Value::object(p));
        self.set(self.lp + loc, Value::object(a));
        Ok(())
    }

    /// `await loc`.
    pub(super) fn await_value(&mut self, next: u32) -> Step {
        let a = self.local_generator(self.imm_u16(1), &[ASYNC, ASYNC_GENERATOR])?;
        let v = self.pop();
        self.suspend_frame(a, AWAITING, next)?;
        self.tail_call_builtin(ASYNC_AWAIT, [Value::object(a), v])
    }

    /// `async_return loc` / `async_reject loc`.
    pub(super) fn async_finish(&mut self, reject: bool) -> Step {
        let a = self.local_generator(self.imm_u16(1), &[ASYNC])?;
        let v = self.pop();
        let id = if reject {
            ASYNC_FUNCTION_THROW
        } else {
            ASYNC_FUNCTION_RETURN
        };
        self.tail_call_builtin(id, [Value::object(a), v])
    }

    /// `%GeneratorResume(g, value, mode)`: the arguments are popped (`sp` = their base);
    /// pushes the saved frame of `g` with `PushResult` back to `next`.
    pub(super) fn generator_resume(
        &mut self,
        g: Value,
        value: Value,
        mode: Value,
        next: u32,
    ) -> Step {
        let g = g
            .as_object()
            .filter(|&o| self.heap.object_class(o) == ObjectClass::Generator);
        let (Some(g), Some(mode)) = (g, mode.as_int()) else {
            return intrinsics::bad_argument("GeneratorResume");
        };
        let state = state_of(self.heap, g);
        let saved = self.heap.object_slot(g, FRAME).as_internal();
        let (Some(arr), START | SUSPENDED | AWAITING | SUSPENDED_RAW) = (saved, state) else {
            return intrinsics::bad_argument("GeneratorResume");
        };
        let pc = self.heap.object_slot(g, PC).as_int().unwrap_or(0) as u32;
        let f = self
            .heap
            .value_array_get(arr, 0)
            .as_object()
            .expect("callee");
        let t = self.template_of(f);
        if state != START && mode == 1 {
            // Thrown at the suspension point: without a handler there, it leaves the
            // activation at once, as if thrown by this intrinsic.
            let caught = HandlerTable::new(crate::bytecode::template::handlers(self.heap, t))
                .ok()
                .and_then(|h| h.find(pc - 1))
                .is_some();
            if !caught {
                return Err(Fault::throw(value));
            }
        }
        let hp_off = self.heap.object_slot(g, HP).as_int().unwrap_or(0) as u32;
        let len = self.heap.value_array_len(arr);
        let base = self.sp;
        // Room for the frame, its operand stack and the resume values.
        let top = base
            + hp_off
            + HEADER
            + self.heap.arena().get(t, FuncTemplate::LOCALS) as u32
            + self.heap.arena().get(t, FuncTemplate::STACK) as u32;
        self.ensure(top.max(base + len + 2))?;
        for i in 0..len {
            let v = self.heap.value_array_get(arr, i);
            self.set(base + i, v);
        }
        self.fuel -= (len / 8) as i64;
        let hp = base + hp_off;
        let header = [
            (frame::PREV_BP, Value::int(self.bp as i32)),
            (frame::PREV_HP, Value::int(self.hp as i32)),
            (frame::RET_PC, Value::int(next as i32)),
            (
                frame::FLAGS,
                Value::int(Continuation::PushResult.to_flags()),
            ),
        ];
        for (k, v) in header {
            self.set(hp + k, v);
        }
        self.heap.set_object_slot(g, STATE, Value::int(EXECUTING));
        self.heap.set_object_slot(g, FRAME, Value::UNDEFINED);
        self.bp = base;
        self.hp = hp;
        self.sp = base + len;
        self.pc = pc;
        self.enter_template(t);
        match (state, mode) {
            (START, _) => {}
            (_, 1) => {
                let st = self.throw_value(value, pc - 1);
                if st.is_none() {
                    self.safepoint();
                }
                return Ok(st);
            }
            (AWAITING, _) => self.push(value),
            _ => {
                self.push(value);
                self.push(Value::bool(mode == 2));
            }
        }
        self.safepoint();
        Ok(None)
    }
}
