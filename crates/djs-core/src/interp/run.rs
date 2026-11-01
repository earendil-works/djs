//! The dispatch loop (docs/compiler.md 7).
//!
//! [`Exec`] holds the registers in Rust locals while it runs and flushes them to the
//! `Interp` cell when it stops. It never recurses on the Rust stack: a JS call pushes a
//! frame record on the arena value stack and continues the loop.
//!
//! Fuel: every instruction charges its cost class. `run` stops before the next instruction
//! once the fuel is used up, so resuming continues exactly there.
//!
//! Safepoints (back jumps, calls, returns, and the start of `run`): when the heap asks for
//! a collection (or always, with `gc_stress`), the registers are flushed, the stack above
//! `sp` is cleared, the heap is collected, and the cached code and stack offsets are
//! re-derived from the roots and `stack[bp]`. Everything live is in the arena, so the
//! collector needs nothing else.

use alloc::format;
use alloc::string::String;

use super::conv::{self, Fault, type_error};
use super::frame::{self, Continuation, HEADER};
use super::realm::{self, ErrorKind, Intrinsic, intrinsic, intrinsic_obj};
use super::state::{self, PARK_ENTRY, RunState, park_flags};
use crate::bytecode::decode::{read_i8, read_i32, read_u16, read_u32};
use crate::bytecode::global_decls::GlobalDecls;
use crate::bytecode::template::func_flags;
use crate::bytecode::template::{bytes_of, ref_array_get};
use crate::bytecode::{HandlerTable, OPS, Op, special};
use crate::heap::cells::{BoxCell, FuncTemplate, HostCall, Interp, Roots, U32Array, ValueArray};
use crate::heap::{
    Atom, CellKind, Heap, HeapError, ObjectClass, PropFlags, Property, PropertyDescriptor, Ref,
};
use crate::value::{Kind, Obj, Value};

/// Why `run` returned.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    /// The call returned this value.
    Done(Value),
    /// The call threw this value.
    Threw(Value),
    /// The fuel ran out; `run` again to continue.
    OutOfFuel,
    /// Parked on a sync host call: resolve it, then `run` again.
    HostCall { call_id: u32, fn_id: u32 },
    /// Nothing to run (no call in progress).
    Idle,
}

/// Interpreter options that are not VM state.
#[derive(Clone, Copy, Debug)]
pub struct ExecOptions {
    /// Collect at every safepoint (a stress mode for tests).
    pub gc_stress: bool,
    /// Maximum value stack size in slots (`InternalError: stack overflow` beyond).
    pub max_stack: u32,
}

impl Default for ExecOptions {
    fn default() -> Self {
        ExecOptions {
            gc_stress: false,
            max_stack: 1 << 20,
        }
    }
}

mod intrinsics;

type Step = Result<Option<Status>, Fault>;

/// Frames in a stack trace (QuickJS's default `Error.stackTraceLimit`).
pub const STACK_TRACE_LIMIT: u32 = 10;

pub(crate) struct Exec<'h> {
    pub heap: &'h mut Heap,
    opts: ExecOptions,
    fuel: i64,
    /// The value stack cell.
    stack: u32,
    bp: u32,
    hp: u32,
    sp: u32,
    pc: u32,
    // Cached from the current frame's template; re-derived after every safepoint.
    template: u32,
    code: u32,
    code_len: u32,
    consts: u32,
    lp: u32,
}

impl<'h> Exec<'h> {
    /// Loads the registers from the `Interp` cell.
    pub fn new(heap: &'h mut Heap, opts: ExecOptions) -> Exec<'h> {
        let i = state::interp(heap);
        let a = heap.arena();
        let (bp, hp, sp, pc) = (
            a.get(i, Interp::BP),
            a.get(i, Interp::HP),
            a.get(i, Interp::SP),
            a.get(i, Interp::PC),
        );
        let stack = state::stack(heap);
        Exec {
            heap,
            opts,
            fuel: 0,
            stack,
            bp,
            hp,
            sp,
            pc,
            template: 0,
            code: 0,
            code_len: 0,
            consts: 0,
            lp: 0,
        }
    }

    /// Writes the registers to the `Interp` cell.
    pub(crate) fn flush(&mut self) {
        let i = state::interp(self.heap);
        let a = self.heap.arena_mut();
        a.set(i, Interp::BP, self.bp);
        a.set(i, Interp::HP, self.hp);
        a.set(i, Interp::SP, self.sp);
        a.set(i, Interp::PC, self.pc);
    }

    // ---------------------------------------------------------------- stack

    pub(crate) fn set_slot(&mut self, i: u32, v: Value) {
        self.set(i, v);
    }

    pub(crate) fn set_sp(&mut self, sp: u32) {
        self.sp = sp;
    }

    #[inline]
    fn get(&self, i: u32) -> Value {
        self.heap.arena().read(ValueArray::VALUES.at(self.stack, i))
    }

    #[inline]
    fn set(&mut self, i: u32, v: Value) {
        let at = ValueArray::VALUES.at(self.stack, i);
        self.heap.arena_mut().write(at, v)
    }

    #[inline]
    pub(crate) fn push(&mut self, v: Value) {
        self.set(self.sp, v);
        self.sp += 1;
    }

    #[inline]
    fn pop(&mut self) -> Value {
        self.sp -= 1;
        self.get(self.sp)
    }

    #[inline]
    fn top(&self, k: u32) -> Value {
        self.get(self.sp - 1 - k)
    }

    #[inline]
    fn header(&self, f: u32) -> Value {
        self.get(self.hp + f)
    }

    fn header_int(&self, f: u32) -> u32 {
        self.header(f).as_int().unwrap_or(0) as u32
    }

    /// Makes room for `top` slots, growing the stack by copying.
    pub(crate) fn ensure(&mut self, top: u32) -> Result<(), Fault> {
        let cap = self.heap.value_array_capacity(self.stack);
        if top <= cap {
            return Ok(());
        }
        if top > self.opts.max_stack {
            return Err(Fault::Error(
                ErrorKind::InternalError,
                String::from("stack overflow"),
            ));
        }
        let new_cap = top
            .max(cap.saturating_mul(2))
            .min(self.opts.max_stack.max(top));
        let new = state::alloc_stack(self.heap, new_cap)?;
        let a = self.heap.arena_mut();
        a.copy_within(
            ValueArray::VALUES.at(self.stack, 0),
            self.sp * 8,
            ValueArray::VALUES.at(new, 0),
        );
        a.set(0, Roots::STACK, Ref(new));
        self.stack = new;
        Ok(())
    }

    // ---------------------------------------------------------------- frames

    /// The template of the closure at `stack[bp]`.
    fn frame_template(&self) -> u32 {
        let f = self.get(self.bp).as_object().expect("frame callee");
        self.heap
            .object_slot(f, 0)
            .as_internal()
            .expect("closure template")
    }

    /// Re-derives the cached frame data from `stack[bp]`.
    fn load_frame(&mut self) {
        let t = self.frame_template();
        let a = self.heap.arena();
        let code: Ref = a.get(t, FuncTemplate::CODE);
        self.template = t;
        self.code = crate::heap::cells::Bytes::DATA.at(code.0, 0);
        self.code_len = bytes_of(self.heap, code).len() as u32;
        self.consts = a.get(t, FuncTemplate::CONSTS).0;
        self.lp = self.hp + HEADER;
    }

    fn locals(&self) -> u32 {
        self.heap.arena().get(self.template, FuncTemplate::LOCALS) as u32
    }

    /// Calls `stack[base]` with `this = stack[base + 1]` and the `argc` arguments above
    /// it (`base + 2 + argc == sp`). JS closures get a frame; host functions park the VM.
    pub(crate) fn call(&mut self, base: u32, argc: u32, cont: Continuation, ret_pc: u32) -> Step {
        let f = self.get(base);
        let Some(o) = f.as_object() else {
            return type_error("not a function");
        };
        match self.heap.object_class(o) {
            ObjectClass::Function => {
                let t = self.heap.object_slot(o, 0).as_internal().expect("template");
                let a = self.heap.arena();
                let params = a.get(t, FuncTemplate::PARAMS) as u32;
                let locals = a.get(t, FuncTemplate::LOCALS) as u32;
                let stack = a.get(t, FuncTemplate::STACK) as u32;
                let n = argc.max(params);
                let hp = base + 2 + n;
                let lp = hp + HEADER;
                self.ensure(lp + locals + stack)?;
                for i in argc..n {
                    self.set(base + 2 + i, Value::UNDEFINED);
                }
                let entry = cont == Continuation::Entry;
                let header = [
                    Value::int(if entry { 0 } else { self.bp as i32 }),
                    Value::int(if entry { 0 } else { self.hp as i32 }),
                    Value::int(ret_pc as i32),
                    Value::int(argc as i32),
                    Value::int(cont.to_flags()),
                    Value::UNDEFINED,
                ];
                for (k, v) in header.into_iter().enumerate() {
                    self.set(hp + k as u32, v);
                }
                for i in 0..locals {
                    self.set(lp + i, Value::UNDEFINED);
                }
                self.bp = base;
                self.hp = hp;
                self.sp = lp + locals;
                self.pc = 0;
                self.load_frame();
                self.safepoint();
                Ok(None)
            }
            ObjectClass::HostFunction => {
                let fn_id = self.heap.object_slot(o, 0).as_int().unwrap_or(0) as u32;
                let args = self.heap.alloc_value_array(argc)?;
                for i in 0..argc {
                    let v = self.get(base + 2 + i);
                    self.heap.value_array_set(args, i, v);
                }
                self.heap.arena_mut().set(args, ValueArray::LEN, argc);
                let this = self.get(base + 1);
                let call_id = add_host_call(self.heap, fn_id, args, this)?;
                self.sp = base;
                self.pc = ret_pc;
                let i = state::interp(self.heap);
                let a = self.heap.arena_mut();
                a.set(i, Interp::HOST_CALL, call_id);
                a.set(
                    i,
                    Interp::FLAGS,
                    park_flags(cont, cont == Continuation::Entry),
                );
                state::set_state(self.heap, RunState::HostCall);
                self.flush();
                Ok(Some(Status::HostCall { call_id, fn_id }))
            }
            _ => type_error("not a function"),
        }
    }

    /// Returns `v` from the current frame.
    fn do_return(&mut self, v: Value) -> Step {
        let cont = Continuation::from_flags(self.header_int(frame::FLAGS) as i32);
        let this = self.get(self.bp + 1);
        if cont == Continuation::Entry {
            self.sp = self.bp;
            state::set_result(self.heap, v);
            state::set_state(self.heap, RunState::Done);
            self.flush();
            return Ok(Some(Status::Done(v)));
        }
        let (prev_bp, prev_hp, ret_pc) = (
            self.header_int(frame::PREV_BP),
            self.header_int(frame::PREV_HP),
            self.header_int(frame::RET_PC),
        );
        self.sp = self.bp;
        self.bp = prev_bp;
        self.hp = prev_hp;
        self.pc = ret_pc;
        self.load_frame();
        match cont {
            Continuation::PushResult => self.push(v),
            Continuation::DiscardResult => {}
            Continuation::ConstructResult => {
                self.push(if v.as_object().is_some() { v } else { this })
            }
            Continuation::Replace(slot) => self.set(slot, v),
            Continuation::Entry => unreachable!(),
        }
        self.safepoint();
        Ok(None)
    }

    /// Throws `exc`: finds the innermost handler, unwinding frames. `key` is the pc
    /// whose try regions apply in the current frame.
    fn throw_value(&mut self, exc: Value, mut key: u32) -> Option<Status> {
        loop {
            let handlers = self.heap.arena().get(self.template, FuncTemplate::HANDLERS);
            let found = HandlerTable::new(bytes_of(self.heap, handlers))
                .ok()
                .and_then(|t| t.find(key));
            if let Some(h) = found {
                self.sp = self.lp + self.locals() + h.depth;
                self.push(exc);
                self.pc = h.target;
                return None;
            }
            let cont = Continuation::from_flags(self.header_int(frame::FLAGS) as i32);
            if cont == Continuation::Entry {
                self.sp = self.bp;
                state::set_result(self.heap, exc);
                state::set_state(self.heap, RunState::Threw);
                self.flush();
                return Some(Status::Threw(exc));
            }
            let ret_pc = self.header_int(frame::RET_PC);
            self.sp = self.bp;
            self.bp = self.header_int(frame::PREV_BP);
            self.hp = self.header_int(frame::PREV_HP);
            self.load_frame();
            key = ret_pc - 1;
        }
    }

    /// Turns a fault into the thrown value.
    /// Turns a fault at `pc` of the current frame (`None`: outside any frame) into the
    /// thrown value. New errors get a `stack`.
    pub(crate) fn fault_value(&mut self, f: Fault, pc: Option<u32>) -> Value {
        let (kind, msg) = match f {
            Fault::Throw(v) => return v,
            Fault::Error(k, m) => (k, m),
            Fault::Heap(HeapError::OutOfMemory) => {
                return intrinsic(self.heap, Intrinsic::OutOfMemoryError);
            }
            Fault::Heap(HeapError::StringTooLong) => {
                (ErrorKind::RangeError, String::from("invalid string length"))
            }
            Fault::Heap(HeapError::InvalidArrayLength) => {
                (ErrorKind::RangeError, String::from("invalid array length"))
            }
        };
        let e = match realm::new_error(self.heap, kind, &msg) {
            Ok(e) => e,
            Err(_) => return intrinsic(self.heap, Intrinsic::OutOfMemoryError),
        };
        if let Some(pc) = pc {
            let _ = self.set_stack(e, pc);
        }
        e
    }

    /// Defines `e.stack` from the current frames, the innermost at `pc`.
    pub(crate) fn set_stack(&mut self, e: Value, pc: u32) -> Result<(), HeapError> {
        let Some(o) = e.as_object() else {
            return Ok(());
        };
        let trace = self.stack_trace(pc);
        let s = self.heap.alloc_str(&trace)?;
        realm::define_hidden(self.heap, o, crate::heap::atoms::STACK, Value::string(s))
    }

    /// The stack trace in QuickJS's format: one `    at name (file:line:col)` line per
    /// frame, innermost first, at most [`STACK_TRACE_LIMIT`] lines. Callers are
    /// positioned at their call (`ret_pc - 1`). Prelude frames are hidden, except
    /// installed builtin methods, which show as `at name (native)`.
    pub(crate) fn stack_trace(&self, pc: u32) -> String {
        use crate::heap::cells::{Object, Script, flags::OBJ_BUILTIN};
        let prelude = intrinsic(self.heap, Intrinsic::PreludeScript).as_internal();
        let mut out = String::new();
        let (mut bp, mut hp, mut pc) = (self.bp, self.hp, pc);
        let mut lines = 0;
        if !self.has_frame() {
            return out;
        }
        loop {
            let f = self.get(bp).as_object().expect("frame callee");
            let t = self.heap.object_slot(f, 0).as_internal().expect("template");
            let a = self.heap.arena();
            let script: Ref = a.get(t, FuncTemplate::SCRIPT);
            let builtin = self.heap.cell_flags(f.offset()) & OBJ_BUILTIN != 0;
            let name_of = |fallback: &str| {
                let name = Atom::from_id(a.get(t, FuncTemplate::NAME));
                if name.is_null() {
                    String::from(fallback)
                } else {
                    self.heap.atom_name(name)
                }
            };
            if builtin {
                // The installed name, not the prelude's internal function name.
                let name = self
                    .heap
                    .get_own_property(f, crate::heap::atoms::NAME)
                    .and_then(|p| p.value())
                    .and_then(|v| v.as_string())
                    .map(|s| self.heap.str_to_string(s))
                    .unwrap_or_default();
                out += &format!("    at {name} (native)\n");
                lines += 1;
            } else if prelude != script.get() {
                let kind = func_flags::kind(a.get(t, FuncTemplate::FLAGS));
                let name = if matches!(kind, func_flags::KIND_SCRIPT | func_flags::KIND_ASYNC_BODY)
                {
                    String::from("<eval>")
                } else {
                    name_of("<anonymous>")
                };
                let file = script
                    .get()
                    .and_then(|s| a.get(s, Script::NAME).get())
                    .map(|n| self.heap.str_to_string(crate::value::Str(n)))
                    .unwrap_or_default();
                let lines_ref: Ref = a.get(t, FuncTemplate::LINES);
                match crate::bytecode::lookup_line(bytes_of(self.heap, lines_ref), pc) {
                    Some((l, c)) => out += &format!("    at {name} ({file}:{l}:{c})\n"),
                    None => out += &format!("    at {name} ({file})\n"),
                }
                lines += 1;
            }
            let _ = Object::CLASS;
            let cont = Continuation::from_flags(self.get(hp + frame::FLAGS).as_int().unwrap_or(0));
            if cont == Continuation::Entry || lines >= STACK_TRACE_LIMIT {
                break;
            }
            let ret_pc = self.get(hp + frame::RET_PC).as_int().unwrap_or(1) as u32;
            pc = ret_pc.saturating_sub(1);
            let prev_bp = self.get(hp + frame::PREV_BP).as_int().unwrap_or(0) as u32;
            hp = self.get(hp + frame::PREV_HP).as_int().unwrap_or(0) as u32;
            bp = prev_bp;
        }
        out
    }

    /// Collects if the heap asks for it (or always under `gc_stress`).
    fn safepoint(&mut self) {
        if !(self.opts.gc_stress || self.heap.gc_pending()) {
            return;
        }
        self.flush();
        // Popped values above `sp` must not keep garbage alive.
        let cap = self.heap.value_array_capacity(self.stack);
        for i in self.sp..cap {
            self.set(i, Value::UNDEFINED);
        }
        if self.heap.collect().is_ok() {
            self.stack = state::stack(self.heap);
            if self.has_frame() {
                self.load_frame();
            }
        }
    }

    fn has_frame(&self) -> bool {
        self.sp > 0
            && self
                .get(self.bp)
                .as_object()
                .is_some_and(|o| self.heap.object_class(o) == ObjectClass::Function)
    }

    // ---------------------------------------------------------------- the loop

    /// Runs with `fuel`.
    pub fn run(&mut self, fuel: u64) -> Status {
        self.fuel = fuel.min(i64::MAX as u64) as i64;
        match state::state(self.heap) {
            RunState::Running | RunState::ThrowPending => {}
            RunState::Idle => return Status::Idle,
            RunState::Done => return Status::Done(state::result(self.heap)),
            RunState::Threw => return Status::Threw(state::result(self.heap)),
            RunState::HostCall => {
                let i = state::interp(self.heap);
                let call_id = self.heap.arena().get(i, Interp::HOST_CALL);
                let fn_id = host_call_cell(self.heap, call_id)
                    .map_or(0, |c| self.heap.arena().get(c, HostCall::FN));
                return Status::HostCall { call_id, fn_id };
            }
        }
        self.load_frame();
        if state::state(self.heap) == RunState::ThrowPending {
            state::set_state(self.heap, RunState::Running);
            let exc = state::result(self.heap);
            if let Some(st) = self.throw_value(exc, self.pc.saturating_sub(1)) {
                return st;
            }
        }
        self.safepoint();
        loop {
            if self.fuel <= 0 {
                self.flush();
                return Status::OutOfFuel;
            }
            let pc0 = self.pc;
            match self.step() {
                Ok(None) => {}
                Ok(Some(st)) => return st,
                Err(f) => {
                    let exc = self.fault_value(f, Some(pc0));
                    if let Some(st) = self.throw_value(exc, pc0) {
                        return st;
                    }
                }
            }
        }
    }

    #[inline]
    fn byte(&self, at: u32) -> u8 {
        self.heap.arena().bytes()[(self.code + at) as usize]
    }

    fn imm_u16(&self, off: u32) -> u32 {
        read_u16(
            self.heap.arena().bytes(),
            (self.code + self.pc + off) as usize,
        ) as u32
    }

    fn imm_u32(&self, off: u32) -> u32 {
        read_u32(
            self.heap.arena().bytes(),
            (self.code + self.pc + off) as usize,
        )
    }

    fn imm_i32(&self, off: u32) -> i32 {
        read_i32(
            self.heap.arena().bytes(),
            (self.code + self.pc + off) as usize,
        )
    }

    fn imm_atom(&self) -> Atom {
        Atom::from_id(self.imm_u32(1))
    }

    /// Executes one instruction.
    fn step(&mut self) -> Step {
        if self.pc >= self.code_len {
            return Err(Fault::Error(
                ErrorKind::InternalError,
                String::from("bad bytecode: pc out of range"),
            ));
        }
        let b = self.byte(self.pc);
        let Some(op) = Op::from_u8(b) else {
            return Err(Fault::Error(
                ErrorKind::InternalError,
                format!("bad bytecode: opcode {b}"),
            ));
        };
        let info = &OPS[op as usize];
        self.fuel -= info.cost.fuel() as i64;
        let next = self.pc + info.size as u32;
        #[cfg(debug_assertions)]
        let (pc0, sp0, bp0) = (self.pc, self.sp, self.bp);
        let r = self.exec(op, next);
        // The table and the interpreter must agree (docs/compiler.md 4.2): when an
        // instruction completes in the same frame, it moved `sp` by its stack effect, and
        // fell through unless it jumps.
        #[cfg(debug_assertions)]
        if matches!(r, Ok(None)) && self.bp == bp0 {
            use crate::bytecode::flags as opflags;
            let pops = info.pop as u32
                + info.pop_imm.map_or(0, |k| {
                    // Count immediates are u16; skip the immediates before them.
                    let skip: u32 = info.imms[..k as usize]
                        .iter()
                        .map(|i| i.width() as u32)
                        .sum();
                    let at = (self.code + pc0 + 1 + skip) as usize;
                    read_u16(self.heap.arena().bytes(), at) as u32
                });
            if info.flags & (opflags::JUMP | opflags::BRANCH | opflags::TERM | opflags::GOSUB) == 0
            {
                debug_assert_eq!(
                    self.pc, next,
                    "{} did not advance pc by its size",
                    info.name
                );
            }
            debug_assert_eq!(
                self.sp + pops,
                sp0 + info.push as u32,
                "{} stack effect disagrees with the table",
                info.name
            );
        }
        r
    }

    fn jump_target(&self, next: u32) -> u32 {
        (next as i64 + self.imm_i32(1) as i64) as u32
    }

    /// The local or capture name `i` from the template's `NAMES` (for TDZ messages).
    fn name_of(&self, i: u32) -> String {
        let names: Ref = self.heap.arena().get(self.template, FuncTemplate::NAMES);
        match names.get() {
            Some(n) if i < self.heap.arena().get(n, U32Array::LEN) => {
                let id = self.heap.arena().read::<u32>(U32Array::DATA.at(n, i));
                self.heap.atom_name(Atom::from_id(id))
            }
            _ => String::from("variable"),
        }
    }

    fn uninitialized<T>(&self, name_index: u32) -> Result<T, Fault> {
        Err(Fault::Error(
            ErrorKind::ReferenceError,
            format!("{} is not initialized", self.name_of(name_index)),
        ))
    }

    fn box_value(&self, b: Value) -> Value {
        let cell = b.as_internal().expect("a Box");
        self.heap.arena().get(cell, BoxCell::VALUE)
    }

    fn set_box_value(&mut self, b: Value, v: Value) {
        let cell = b.as_internal().expect("a Box");
        self.heap.arena_mut().set(cell, BoxCell::VALUE, v)
    }

    fn new_box(&mut self, v: Value) -> Result<Value, Fault> {
        let c = self
            .heap
            .alloc_cell(CellKind::BoxCell, BoxCell::LAYOUT.min_size)?;
        self.heap.arena_mut().set(c, BoxCell::VALUE, v);
        Ok(Value::internal(c))
    }

    /// The Box of capture `i` of the current closure.
    fn capture(&self, i: u32) -> Value {
        let f = self.get(self.bp).as_object().unwrap();
        self.heap.object_slot(f, 1 + i)
    }

    fn constant(&self, i: u32) -> Value {
        assert!(self.consts != 0, "no constant pool");
        self.heap.value_array_get(self.consts, i)
    }

    fn binary_num(&mut self, f: impl Fn(f64, f64) -> f64) -> Result<(), Fault> {
        let b = self.pop();
        let a = self.pop();
        let x = conv::to_number(self.heap, a)?;
        let y = conv::to_number(self.heap, b)?;
        self.push(conv::number(f(x, y)));
        Ok(())
    }

    fn binary_int(&mut self, f: impl Fn(i32, i32) -> Value) -> Result<(), Fault> {
        let b = self.pop();
        let a = self.pop();
        let x = conv::to_int32(conv::to_number(self.heap, a)?);
        let y = conv::to_int32(conv::to_number(self.heap, b)?);
        self.push(f(x, y));
        Ok(())
    }

    fn compare(&mut self, swap: bool, invert: bool) -> Result<(), Fault> {
        // lt: a < b; gt: b < a; le: !(b < a); ge: !(a < b). Undefined (NaN) is false.
        let b = self.pop();
        let a = self.pop();
        let (x, y) = if swap { (b, a) } else { (a, b) };
        let r = conv::less_than(self.heap, x, y)?;
        let v = match r {
            None => false,
            Some(t) => t != invert,
        };
        self.push(Value::bool(v));
        Ok(())
    }

    fn exec(&mut self, op: Op, next: u32) -> Step {
        if let Some(step) = self.primitive_operands(op) {
            return step;
        }
        match op {
            // ---- constants ----
            Op::Undefined => self.push(Value::UNDEFINED),
            Op::Null => self.push(Value::NULL),
            Op::PushTrue => self.push(Value::TRUE),
            Op::PushFalse => self.push(Value::FALSE),
            Op::PushI8 => {
                let v = read_i8(
                    self.heap.arena().bytes(),
                    (self.code + self.pc + 1) as usize,
                );
                self.push(Value::int(v as i32));
            }
            Op::PushI32 => {
                let v = self.imm_i32(1);
                self.push(Value::int(v));
            }
            Op::PushConst => {
                let v = self.constant(self.imm_u32(1));
                self.push(v);
            }
            Op::PushAtom => {
                let s = self.heap.atom_to_value(self.imm_atom())?;
                self.push(s);
            }
            Op::Object => {
                let proto = intrinsic_obj(self.heap, Intrinsic::ObjectProto);
                let o = self.heap.new_object(Some(proto))?;
                self.push(Value::object(o));
            }
            Op::ArrayFrom => {
                let n = self.imm_u16(1);
                let proto = intrinsic_obj(self.heap, Intrinsic::ArrayProto);
                let mut vals = alloc::vec::Vec::with_capacity(n as usize);
                for i in 0..n {
                    vals.push(self.get(self.sp - n + i));
                }
                let a = self.heap.new_array_from_values(Some(proto), &vals)?;
                self.sp -= n;
                self.push(Value::object(a));
            }
            Op::ArrayPush => {
                let v = self.pop();
                let a = self.top(0).as_object().expect("array");
                let len = self.heap.array_length(a);
                if !self.heap.set_element_fast(a, len, v)? {
                    let k = self.heap.atom_from_index(len)?;
                    self.heap.create_data_property(a, k, v)?;
                }
            }
            Op::ArrayHole => {
                let a = self.top(0).as_object().expect("array");
                let len = self.heap.array_length(a);
                let d = PropertyDescriptor {
                    value: Some(conv::number(len as f64 + 1.0)),
                    ..Default::default()
                };
                self.heap
                    .define_own_property(a, crate::heap::atoms::LENGTH, &d)?;
            }
            Op::FClosure => {
                let v = self.closure(self.imm_u32(1))?;
                self.push(v);
            }

            // ---- shuffles ----
            Op::Drop => {
                self.sp -= 1;
            }
            Op::Dup => {
                let v = self.top(0);
                self.push(v);
            }
            Op::Dup2 => {
                let (a, b) = (self.top(1), self.top(0));
                self.push(a);
                self.push(b);
            }
            Op::Swap => {
                let (a, b) = (self.top(1), self.top(0));
                self.set(self.sp - 2, b);
                self.set(self.sp - 1, a);
            }
            Op::Nip => {
                let b = self.pop();
                self.set(self.sp - 1, b);
            }
            Op::Insert2 => {
                // a b -> b a b
                let (a, b) = (self.top(1), self.top(0));
                self.set(self.sp - 2, b);
                self.set(self.sp - 1, a);
                self.push(b);
            }
            Op::Insert3 => {
                // a b c -> c a b c
                let (a, b, c) = (self.top(2), self.top(1), self.top(0));
                self.set(self.sp - 3, c);
                self.set(self.sp - 2, a);
                self.set(self.sp - 1, b);
                self.push(c);
            }
            Op::Insert4 => {
                // a b c d -> d a b c d
                let (a, b, c, d) = (self.top(3), self.top(2), self.top(1), self.top(0));
                self.set(self.sp - 4, d);
                self.set(self.sp - 3, a);
                self.set(self.sp - 2, b);
                self.set(self.sp - 1, c);
                self.push(d);
            }
            Op::Rot3L => {
                // x a b -> a b x
                let (x, a, b) = (self.top(2), self.top(1), self.top(0));
                self.set(self.sp - 3, a);
                self.set(self.sp - 2, b);
                self.set(self.sp - 1, x);
            }
            Op::Rot3R => {
                // a b x -> x a b
                let (a, b, x) = (self.top(2), self.top(1), self.top(0));
                self.set(self.sp - 3, x);
                self.set(self.sp - 2, a);
                self.set(self.sp - 1, b);
            }

            // ---- locals, arguments, Boxes, captures ----
            Op::GetLoc => {
                let v = self.get(self.lp + self.imm_u16(1));
                self.push(v);
            }
            Op::PutLoc => {
                let v = self.pop();
                self.set(self.lp + self.imm_u16(1), v);
            }
            Op::SetLoc => {
                let v = self.top(0);
                self.set(self.lp + self.imm_u16(1), v);
            }
            Op::GetLocChk => {
                let i = self.imm_u16(1);
                let v = self.get(self.lp + i);
                if v.is_uninitialized() {
                    return self.uninitialized(i);
                }
                self.push(v);
            }
            Op::PutLocChk | Op::SetLocChk => {
                let i = self.imm_u16(1);
                if self.get(self.lp + i).is_uninitialized() {
                    return self.uninitialized(i);
                }
                let v = if op == Op::PutLocChk {
                    self.pop()
                } else {
                    self.top(0)
                };
                self.set(self.lp + i, v);
            }
            Op::SetLocUninit => {
                self.set(self.lp + self.imm_u16(1), Value::UNINITIALIZED);
            }
            Op::GetArg => {
                let v = self.get(self.bp + 2 + self.imm_u16(1));
                self.push(v);
            }
            Op::PutArg => {
                let v = self.pop();
                self.set(self.bp + 2 + self.imm_u16(1), v);
            }
            Op::SetArg => {
                let v = self.top(0);
                self.set(self.bp + 2 + self.imm_u16(1), v);
            }
            Op::GetBox | Op::GetBoxChk => {
                let i = self.imm_u16(1);
                let v = self.box_value(self.get(self.lp + i));
                if op == Op::GetBoxChk && v.is_uninitialized() {
                    return self.uninitialized(i);
                }
                self.push(v);
            }
            Op::PutBox | Op::SetBox | Op::PutBoxChk | Op::SetBoxChk => {
                let i = self.imm_u16(1);
                let b = self.get(self.lp + i);
                if matches!(op, Op::PutBoxChk | Op::SetBoxChk)
                    && self.box_value(b).is_uninitialized()
                {
                    return self.uninitialized(i);
                }
                let v = if matches!(op, Op::PutBox | Op::PutBoxChk) {
                    self.pop()
                } else {
                    self.top(0)
                };
                self.set_box_value(b, v);
            }
            Op::NewBox => {
                let b = self.new_box(Value::UNINITIALIZED)?;
                self.set(self.lp + self.imm_u16(1), b);
            }
            Op::NewBoxInit => {
                let v = self.top(0);
                let b = self.new_box(v)?;
                self.sp -= 1;
                self.set(self.lp + self.imm_u16(1), b);
            }
            Op::Rebox => {
                let i = self.imm_u16(1);
                let v = self.box_value(self.get(self.lp + i));
                let b = self.new_box(v)?;
                self.set(self.lp + i, b);
            }
            Op::GetCap | Op::GetCapChk => {
                let i = self.imm_u16(1);
                let v = self.box_value(self.capture(i));
                if op == Op::GetCapChk && v.is_uninitialized() {
                    return self.uninitialized(self.locals() + i);
                }
                self.push(v);
            }
            Op::PutCap | Op::SetCap | Op::PutCapChk | Op::SetCapChk => {
                let i = self.imm_u16(1);
                let b = self.capture(i);
                if matches!(op, Op::PutCapChk | Op::SetCapChk)
                    && self.box_value(b).is_uninitialized()
                {
                    return self.uninitialized(self.locals() + i);
                }
                let v = if matches!(op, Op::PutCap | Op::PutCapChk) {
                    self.pop()
                } else {
                    self.top(0)
                };
                self.set_box_value(b, v);
            }

            // ---- globals ----
            Op::GetGlobal | Op::GetGlobalTypeof => {
                let key = self.imm_atom();
                if let Some(v) = self.global_lexical(key)? {
                    self.push(v);
                    self.pc = next;
                    return Ok(None);
                }
                let g = intrinsic_obj(self.heap, Intrinsic::Global);
                match self.heap.find_property(g, key) {
                    None if op == Op::GetGlobal => {
                        return Err(Fault::Error(
                            ErrorKind::ReferenceError,
                            format!("'{}' is not defined", self.heap.atom_name(key)),
                        ));
                    }
                    None => self.push(Value::UNDEFINED),
                    Some((_, Property::Data { value, .. })) => self.push(value),
                    Some((_, Property::Accessor { getter, .. })) => {
                        self.pc = next;
                        return self.call_accessor(getter, Value::object(g), None);
                    }
                }
            }
            Op::PutGlobal => {
                let key = self.imm_atom();
                let lex = realm::global_lex(self.heap);
                if let Some(Property::Data { flags, .. }) = self.heap.get_own_property(lex, key) {
                    self.global_lexical(key)?;
                    if !flags.writable() {
                        return Err(Fault::Error(
                            ErrorKind::TypeError,
                            format!("'{}' is read-only", self.heap.atom_name(key)),
                        ));
                    }
                    let v = self.pop();
                    realm::set_global_lexical(self.heap, key, v)?;
                    self.pc = next;
                    return Ok(None);
                }
                let g = intrinsic_obj(self.heap, Intrinsic::Global);
                if !self.heap.has_property(g, key) {
                    return Err(Fault::Error(
                        ErrorKind::ReferenceError,
                        format!("'{}' is not defined", self.heap.atom_name(key)),
                    ));
                }
                let v = self.top(0);
                self.pc = next;
                return self.set_property(g, key, v, 1);
            }
            Op::InitGlobalLex => {
                let key = self.imm_atom();
                let v = self.pop();
                realm::set_global_lexical(self.heap, key, v)?;
            }
            Op::GlobalDecls => {
                let idx = self.imm_u32(1);
                self.global_decls(idx)?;
            }
            Op::Rest => {
                let index = self.imm_u16(1);
                let argc = self.header_int(frame::ARGC);
                let mut vals = alloc::vec::Vec::new();
                for i in index..argc.max(index) {
                    vals.push(self.get(self.bp + 2 + i));
                }
                let proto = intrinsic_obj(self.heap, Intrinsic::ArrayProto);
                let a = self.heap.new_array_from_values(Some(proto), &vals)?;
                self.push(Value::object(a));
            }
            Op::SpecialObject => {
                let kind = self.heap.arena().bytes()[(self.code + self.pc + 1) as usize];
                let v = self.special_object(kind)?;
                self.push(v);
            }
            Op::ThrowConstAssign => {
                return Err(Fault::Error(
                    ErrorKind::TypeError,
                    format!("'{}' is read-only", self.heap.atom_name(self.imm_atom())),
                ));
            }

            // ---- properties ----
            Op::GetField => {
                let key = self.imm_atom();
                let o = self.top(0);
                self.pc = next;
                return self.get_property(o, key);
            }
            Op::GetMethod => {
                let key = self.imm_atom();
                let o = self.top(0);
                match self.lookup(o, key)? {
                    Ok(f) => {
                        self.set(self.sp - 1, f);
                        self.push(o);
                    }
                    Err(getter) => {
                        // `o -> f o`: the getter's result fills the placeholder.
                        self.set(self.sp - 1, Value::UNDEFINED);
                        self.push(o);
                        self.pc = next;
                        return self.call_getter_into(getter, o, self.sp - 2);
                    }
                }
            }
            Op::PutField => {
                let key = self.imm_atom();
                let (o, v) = (self.top(1), self.top(0));
                let Some(obj) = o.as_object() else {
                    return self.put_on_primitive(o, key);
                };
                self.pc = next;
                return self.set_property(obj, key, v, 2);
            }
            Op::GetElem => {
                let (o, k) = (self.top(1), self.top(0));
                if let (Some(obj), Some(i)) = (o.as_object(), k.as_int())
                    && i >= 0
                    && let Some(v) = self.heap.get_element_fast(obj, i as u32)
                {
                    self.sp -= 2;
                    self.push(v);
                } else {
                    let key = self.property_key(o, k)?;
                    self.sp -= 1;
                    self.pc = next;
                    return self.get_property(o, key);
                }
            }
            Op::GetElemMethod => {
                let (o, k) = (self.top(1), self.top(0));
                let key = self.property_key(o, k)?;
                match self.lookup(o, key)? {
                    Ok(f) => {
                        self.set(self.sp - 2, f);
                        self.set(self.sp - 1, o);
                    }
                    Err(getter) => {
                        self.set(self.sp - 2, Value::UNDEFINED);
                        self.set(self.sp - 1, o);
                        self.pc = next;
                        return self.call_getter_into(getter, o, self.sp - 2);
                    }
                }
            }
            Op::PutElem => {
                let (o, k, v) = (self.top(2), self.top(1), self.top(0));
                let Some(obj) = o.as_object() else {
                    let key = conv::to_property_key(self.heap, k)?;
                    return self.put_on_primitive(o, key);
                };
                if let Some(i) = k.as_int()
                    && i >= 0
                    && self.heap.set_element_fast(obj, i as u32, v)?
                {
                    self.sp -= 3;
                } else {
                    let key = self.property_key(o, k)?;
                    // Rearrange `o k v` to `o v` for set_property.
                    self.set(self.sp - 2, v);
                    self.sp -= 1;
                    self.pc = next;
                    return self.set_property(obj, key, v, 2);
                }
            }
            Op::DefineField => {
                let key = self.imm_atom();
                let v = self.pop();
                let o = self.top(0).as_object().expect("object literal");
                self.heap.create_data_property(o, key, v)?;
            }
            Op::In => {
                let (k, o) = (self.top(1), self.top(0));
                let Some(obj) = o.as_object() else {
                    return type_error("invalid 'in' operand");
                };
                let key = self.property_key(o, k)?;
                let r = self.heap.has_property(obj, key);
                self.sp -= 2;
                self.push(Value::bool(r));
            }
            Op::DefineMethod | Op::DefineGetter | Op::DefineSetter => {
                let key = self.imm_atom();
                let f = self.pop();
                let o = self.top(0).as_object().expect("object literal");
                self.define_literal_member(o, key, f, op)?;
            }
            Op::DefineMethodElem | Op::DefineGetterElem | Op::DefineSetterElem => {
                let (o, k, f) = (self.top(2), self.top(1), self.top(0));
                let key = self.property_key(o, k)?;
                let obj = o.as_object().expect("object literal");
                self.define_literal_member(obj, key, f, op)?;
                self.sp -= 2;
            }
            Op::ArraySpread => {
                let (a, src) = (self.top(1), self.top(0));
                let arr = a.as_object().expect("array literal");
                self.array_spread(arr, src)?;
                self.sp -= 1;
            }
            Op::DefineElem => {
                let (o, k, v) = (self.top(2), self.top(1), self.top(0));
                let key = self.property_key(o, k)?;
                let obj = o.as_object().expect("object literal");
                self.heap.create_data_property(obj, key, v)?;
                self.sp -= 2;
            }
            Op::CopyDataProps => {
                let mode = self.heap.arena().bytes()[(self.code + self.pc + 1) as usize];
                if mode != 0 {
                    return Err(Fault::Error(
                        ErrorKind::InternalError,
                        format!("not supported yet: copy_data_props {mode}"),
                    ));
                }
                let (o, src) = (self.top(1), self.top(0));
                self.copy_data_props(o.as_object().expect("object literal"), src)?;
                self.sp -= 1;
            }
            Op::Delete => {
                let (o, k) = (self.top(1), self.top(0));
                let r = self.delete(o, k)?;
                self.sp -= 2;
                self.push(Value::bool(r));
            }
            Op::Construct => {
                let argc = self.imm_u16(1);
                return self.construct(argc, next);
            }
            Op::SetProto => {
                let v = self.pop();
                let o = self.top(0).as_object().expect("object literal");
                if v.is_null() || v.as_object().is_some() {
                    // A fresh literal is extensible and cannot be on v's chain.
                    self.heap.set_prototype_of(o, v.as_object());
                }
            }
            Op::SetFunctionName => {
                let prefix = self.heap.arena().bytes()[(self.code + self.pc + 1) as usize];
                let (k, f) = (self.top(1), self.top(0));
                let key = self.property_key(f, k)?;
                self.set_function_name(f, key, prefix)?;
            }
            Op::CallSpread => {
                // `f this args -> r`
                let args = self.top(0);
                let argc = self.spread_args(args)?;
                let base = self.sp - 2 - argc;
                return self.call(base, argc, Continuation::PushResult, next);
            }
            Op::ConstructSpread => {
                // `f args -> r`
                let args = self.top(0);
                let argc = self.spread_args(args)?;
                return self.construct(argc, next);
            }
            Op::CallIntrinsic => {
                let id = self.imm_u16(1) as u16;
                let argc = self.imm_u16(3);
                return self.call_intrinsic(id, argc, next);
            }
            Op::RequireObjectCoercible => {
                if self.top(0).is_nullish() {
                    return type_error("cannot convert undefined or null to object");
                }
            }

            // ---- operators ----
            Op::Neg => {
                let a = self.pop();
                let v = match a.as_int() {
                    Some(i) if i != 0 && i != i32::MIN => Value::int(-i),
                    _ => Value::number(-conv::to_number(self.heap, a)?),
                };
                self.push(v);
            }
            Op::Plus | Op::ToNumeric => {
                let a = self.pop();
                let n = conv::to_number(self.heap, a)?;
                self.push(conv::number(n));
            }
            Op::BNot => {
                let a = self.pop();
                let n = conv::to_int32(conv::to_number(self.heap, a)?);
                self.push(Value::int(!n));
            }
            Op::Not => {
                let a = self.pop();
                let b = conv::to_boolean(self.heap, a);
                self.push(Value::bool(!b));
            }
            Op::Typeof => {
                let a = self.pop();
                let s = self.heap.atom_string(conv::typeof_atom(self.heap, a))?;
                self.push(Value::string(s));
            }
            Op::Inc | Op::Dec => {
                let a = self.pop();
                let d = if op == Op::Inc { 1 } else { -1 };
                let v = match a.as_int().and_then(|i| i.checked_add(d)) {
                    Some(i) => Value::int(i),
                    None => conv::number(conv::to_number(self.heap, a)? + d as f64),
                };
                self.push(v);
            }
            Op::ToString => {
                let a = self.pop();
                let s = conv::to_string(self.heap, a)?;
                self.push(Value::string(s));
            }
            Op::Add => {
                let (a, b) = (self.top(1), self.top(0));
                let v = conv::add(self.heap, a, b)?;
                self.sp -= 2;
                self.push(v);
            }
            Op::Sub => self.binary_num(|x, y| x - y)?,
            Op::Mul => self.binary_num(|x, y| x * y)?,
            Op::Div => self.binary_num(|x, y| x / y)?,
            Op::Mod => self.binary_num(|x, y| x % y)?,
            Op::Pow => self.binary_num(conv::pow)?,
            Op::Shl => self.binary_int(|x, y| Value::int(x.wrapping_shl(y as u32 & 31)))?,
            Op::Sar => self.binary_int(|x, y| Value::int(x >> (y as u32 & 31)))?,
            Op::Shr => {
                self.binary_int(|x, y| conv::number(((x as u32) >> (y as u32 & 31)) as f64))?
            }
            Op::BAnd => self.binary_int(|x, y| Value::int(x & y))?,
            Op::BOr => self.binary_int(|x, y| Value::int(x | y))?,
            Op::BXor => self.binary_int(|x, y| Value::int(x ^ y))?,
            Op::Lt => self.compare(false, false)?,
            Op::Gt => self.compare(true, false)?,
            Op::Le => self.compare(true, true)?,
            Op::Ge => self.compare(false, true)?,
            Op::Eq | Op::Ne => {
                let (a, b) = (self.top(1), self.top(0));
                let r = conv::loose_equals(self.heap, a, b)?;
                self.sp -= 2;
                self.push(Value::bool(r == (op == Op::Eq)));
            }
            Op::StrictEq | Op::StrictNe => {
                let b = self.pop();
                let a = self.pop();
                let r = conv::strict_equals(self.heap, a, b);
                self.push(Value::bool(r == (op == Op::StrictEq)));
            }
            Op::Instanceof => {
                let (v, f) = (self.top(1), self.top(0));
                let r = self.instance_of(v, f)?;
                self.sp -= 2;
                self.push(Value::bool(r));
            }
            Op::IsNullish => {
                let a = self.pop();
                self.push(Value::bool(a.is_nullish()));
            }
            Op::IsUndefined => {
                let a = self.pop();
                self.push(Value::bool(a.is_undefined()));
            }
            Op::Concat => {
                let n = self.imm_u16(1);
                let mut acc = self.heap.atom_string(crate::heap::atoms::EMPTY)?;
                for i in 0..n {
                    let v = self.get(self.sp - n + i);
                    let s = conv::to_string(self.heap, v)?;
                    acc = self.heap.concat(acc, s)?;
                }
                self.sp -= n;
                self.push(Value::string(acc));
            }

            // ---- control flow ----
            Op::Jump => {
                self.pc = self.jump_target(next);
                if self.pc < next {
                    self.safepoint();
                }
                return Ok(None);
            }
            Op::JumpIfFalse | Op::JumpIfTrue => {
                let v = self.pop();
                let t = conv::to_boolean(self.heap, v);
                if t == (op == Op::JumpIfTrue) {
                    self.pc = self.jump_target(next);
                    if self.pc < next {
                        self.safepoint();
                    }
                } else {
                    self.pc = next;
                }
                return Ok(None);
            }
            Op::Gosub => {
                self.push(Value::int(next as i32));
                self.pc = self.jump_target(next);
                return Ok(None);
            }
            Op::Ret => {
                let r = self.pop();
                self.pc = r.as_int().expect("gosub return address") as u32;
                return Ok(None);
            }
            Op::Throw => {
                let v = self.pop();
                return Err(Fault::Throw(v));
            }
            Op::Return => {
                let v = self.pop();
                return self.do_return(v);
            }
            Op::ReturnUndef => return self.do_return(Value::UNDEFINED),
            Op::Call => {
                let argc = self.imm_u16(1);
                let base = self.sp - 2 - argc;
                return self.call(base, argc, Continuation::PushResult, next);
            }
            Op::Debugger | Op::Nop => {}
            Op::NameGet
            | Op::NamePut
            | Op::NameSet
            | Op::NameInit
            | Op::NameTypeof
            | Op::EnterScope
            | Op::LeaveScope
            | Op::Label => {
                return Err(Fault::Error(
                    ErrorKind::InternalError,
                    format!("bad bytecode: phase-1 op {}", op.name()),
                ));
            }
            _ => {
                return Err(Fault::Error(
                    ErrorKind::InternalError,
                    format!("not supported yet: {}", op.name()),
                ));
            }
        }
        self.pc = next;
        Ok(None)
    }

    // ---------------------------------------------------------------- helpers

    /// Creates the closure for child template `child` of the current function.
    fn closure(&mut self, child: u32) -> Result<Value, Fault> {
        let children = self.heap.arena().get(self.template, FuncTemplate::CHILDREN);
        let t = ref_array_get(self.heap, children, child).expect("child template");
        let a = self.heap.arena();
        let ncap = a.get(t.0, FuncTemplate::CAPTURES) as u32;
        let desc: Ref = a.get(t.0, FuncTemplate::CAPTURE_DESC);
        let proto = intrinsic_obj(self.heap, Intrinsic::FunctionProto);
        let f = self
            .heap
            .alloc_object(ObjectClass::Function, Some(proto), 1 + ncap)?;
        self.heap.set_object_slot(f, 0, Value::internal(t.0));
        for i in 0..ncap {
            let d = self.heap.arena().read::<u32>(U32Array::DATA.at(desc.0, i));
            let b = if d & 0x8000_0000 != 0 {
                self.capture(d & 0x7FFF_FFFF)
            } else {
                self.get(self.lp + d)
            };
            self.heap.set_object_slot(f, 1 + i, b);
        }
        self.function_properties(f, t.0)?;
        Ok(Value::object(f))
    }

    /// `length`, `name` and (for ordinary functions) `prototype` of a new closure.
    fn function_properties(&mut self, f: Obj, t: u32) -> Result<(), Fault> {
        use crate::heap::atoms;
        let a = self.heap.arena();
        let length = a.get(t, FuncTemplate::LENGTH);
        let name = Atom::from_id(a.get(t, FuncTemplate::NAME));
        let flags = a.get(t, FuncTemplate::FLAGS);
        let ro = PropFlags::CONFIGURABLE;
        let d = PropertyDescriptor::data(Value::int(length as i32), ro);
        self.heap.define_own_property(f, atoms::LENGTH, &d)?;
        let name = if name.is_null() {
            self.heap.atom_string(atoms::EMPTY)?
        } else {
            self.heap.atom_string(name)?
        };
        let d = PropertyDescriptor::data(Value::string(name), ro);
        self.heap.define_own_property(f, atoms::NAME, &d)?;
        if func_flags::kind(flags) == func_flags::KIND_NORMAL
            && flags & (func_flags::ASYNC | func_flags::GENERATOR) == 0
        {
            let op = intrinsic_obj(self.heap, Intrinsic::ObjectProto);
            let proto = self.heap.new_object(Some(op))?;
            realm::define_hidden(self.heap, proto, atoms::CONSTRUCTOR, Value::object(f))?;
            let d = PropertyDescriptor::data(Value::object(proto), PropFlags::WRITABLE);
            self.heap.define_own_property(f, atoms::PROTOTYPE, &d)?;
        }
        Ok(())
    }

    /// ToPropertyKey of a primitive key. Object keys never get here: the
    /// `primitive_operands` slow path converts them first.
    fn property_key(&mut self, _o: Value, k: Value) -> Result<Atom, Fault> {
        debug_assert!(k.as_object().is_none());
        conv::to_property_key(self.heap, k)
    }

    /// The slow path for object operands (D1280): if an operand of `op` that must be a
    /// primitive is an object, call `%ToPrimitive(operand, hint)` with a `Replace`
    /// continuation that writes the result into the operand's slot and re-executes this
    /// instruction. Operands are converted left to right, as the spec evaluates them.
    fn primitive_operands(&mut self, op: Op) -> Option<Step> {
        use crate::heap::atoms::{DEFAULT, NUMBER, STRING};
        let checks: &[(u32, Atom)] = match op {
            Op::Add => &[(1, DEFAULT), (0, DEFAULT)],
            Op::Sub
            | Op::Mul
            | Op::Div
            | Op::Mod
            | Op::Pow
            | Op::Shl
            | Op::Sar
            | Op::Shr
            | Op::BAnd
            | Op::BOr
            | Op::BXor
            | Op::Lt
            | Op::Gt
            | Op::Le
            | Op::Ge => &[(1, NUMBER), (0, NUMBER)],
            Op::Neg | Op::Plus | Op::BNot | Op::Inc | Op::Dec | Op::ToNumeric => &[(0, NUMBER)],
            Op::ToString | Op::GetElem | Op::GetElemMethod | Op::Delete => &[(0, STRING)],
            Op::PutElem
            | Op::DefineElem
            | Op::DefineMethodElem
            | Op::DefineGetterElem
            | Op::DefineSetterElem
            | Op::In
            | Op::SetFunctionName => &[(1, STRING)],
            Op::Eq | Op::Ne => {
                // Only an object compared with a non-nullish primitive is converted.
                let (a, b) = (self.top(1), self.top(0));
                let prim = |v: Value| v.as_object().is_none() && !v.is_nullish();
                let k = if a.as_object().is_some() && prim(b) {
                    1
                } else if b.as_object().is_some() && prim(a) {
                    0
                } else {
                    return None;
                };
                return Some(self.call_to_primitive(self.sp - 1 - k, DEFAULT));
            }
            Op::Concat => {
                let n = self.imm_u16(1);
                return (0..n)
                    .map(|i| self.sp - n + i)
                    .find(|&slot| self.get(slot).as_object().is_some())
                    .map(|slot| self.call_to_primitive(slot, STRING));
            }
            _ => return None,
        };
        for &(k, hint) in checks {
            let slot = self.sp - 1 - k;
            if self.get(slot).as_object().is_some() {
                return Some(self.call_to_primitive(slot, hint));
            }
        }
        None
    }

    /// Calls `%ToPrimitive(stack[slot], hint)`; its result replaces `stack[slot]` and the
    /// current instruction runs again.
    fn call_to_primitive(&mut self, slot: u32, hint: Atom) -> Step {
        let f = realm::installed_intrinsic(self.heap, intrinsics::TO_PRIMITIVE);
        if f.is_undefined() {
            return type_error("cannot convert an object to a primitive (no prelude)");
        }
        let v = self.get(slot);
        let hint = Value::string(self.heap.atom_string(hint)?);
        let base = self.sp;
        self.ensure(base + 4)?;
        self.push(f);
        self.push(Value::UNDEFINED);
        self.push(v);
        self.push(hint);
        let pc = self.pc;
        self.call(base, 2, Continuation::Replace(slot), pc)
            .inspect_err(|_| self.sp = base)
    }

    /// Calls a getter whose result goes into `stack[slot]`; resumes at `self.pc`.
    fn call_getter_into(&mut self, getter: Value, this: Value, slot: u32) -> Step {
        let base = self.sp;
        self.ensure(base + 2)?;
        self.push(getter);
        self.push(this);
        let pc = self.pc;
        self.call(base, 0, Continuation::Replace(slot), pc)
            .inspect_err(|_| self.sp = base)
    }

    fn nullish_base<T>(&self, o: Value, key: Atom, verb: &str) -> Result<T, Fault> {
        let what = if o.is_null() { "null" } else { "undefined" };
        type_error(&format!(
            "cannot {verb} property '{}' of {what}",
            conv::describe_key(self.heap, key)
        ))
    }

    /// The value of `o[key]`, or `Err(getter)` if a getter must be called.
    fn lookup(&mut self, o: Value, key: Atom) -> Result<Result<Value, Value>, Fault> {
        match o.kind() {
            Kind::Object => {
                let obj = o.as_object().unwrap();
                Ok(match self.heap.find_property(obj, key) {
                    None => Ok(Value::UNDEFINED),
                    Some((_, Property::Data { value, .. })) => Ok(value),
                    Some((_, Property::Accessor { getter, .. })) if getter.is_undefined() => {
                        Ok(Value::UNDEFINED)
                    }
                    Some((_, Property::Accessor { getter, .. })) => Err(getter),
                })
            }
            Kind::String => {
                let s = o.as_string().unwrap();
                let len = self.heap.str_len(s);
                if key == crate::heap::atoms::LENGTH {
                    return Ok(Ok(Value::int(len as i32)));
                }
                if let Some(i) = key.as_int()
                    && i < len
                {
                    let u = self.heap.char_code_at(s, i).unwrap();
                    let c = self.heap.alloc_utf16(&[u])?;
                    return Ok(Ok(Value::string(c)));
                }
                // String.prototype does not exist yet.
                Ok(Ok(Value::UNDEFINED))
            }
            Kind::Undefined | Kind::Null => self.nullish_base(o, key, "read"),
            _ => Ok(Ok(Value::UNDEFINED)),
        }
    }

    /// Replaces the object at `sp - 1` with `o[key]`, calling a getter if needed. `pc` is
    /// already the next instruction.
    fn get_property(&mut self, o: Value, key: Atom) -> Step {
        match self.lookup(o, key)? {
            Ok(v) => {
                self.set(self.sp - 1, v);
                Ok(None)
            }
            Err(getter) => {
                self.sp -= 1;
                self.call_accessor(getter, o, None)
            }
        }
    }

    /// Calls a getter (`arg = None`, result pushed) or a setter (result dropped). `pc`
    /// must already be the instruction to resume.
    fn call_accessor(&mut self, f: Value, this: Value, arg: Option<Value>) -> Step {
        let base = self.sp;
        self.ensure(base + 3)?;
        self.push(f);
        self.push(this);
        let (argc, cont) = match arg {
            None => (0, Continuation::PushResult),
            Some(v) => {
                self.push(v);
                (1, Continuation::DiscardResult)
            }
        };
        let ret = self.pc;
        self.call(base, argc, cont, ret)
            .inspect_err(|_| self.sp = base)
    }

    /// [[Set]] with receiver `o` for `o v` (the top `n` operands are consumed).
    fn set_property(&mut self, o: Obj, key: Atom, v: Value, n: u32) -> Step {
        let found = self.heap.find_property(o, key);
        let readonly = |heap: &Heap| {
            Err(Fault::Error(
                ErrorKind::TypeError,
                format!("'{}' is read-only", heap.atom_name(key)),
            ))
        };
        match found {
            Some((_, Property::Accessor { setter, .. })) => {
                if setter.is_undefined() {
                    return type_error(&format!(
                        "no setter for property '{}'",
                        self.heap.atom_name(key)
                    ));
                }
                self.sp -= n;
                return self.call_accessor(setter, Value::object(o), Some(v));
            }
            Some((holder, Property::Data { flags, .. })) => {
                if !flags.writable() {
                    return readonly(self.heap);
                }
                if holder == o {
                    let d = PropertyDescriptor {
                        value: Some(v),
                        ..Default::default()
                    };
                    if !self.heap.define_own_property(o, key, &d)? {
                        return readonly(self.heap);
                    }
                } else if !self.heap.create_data_property(o, key, v)? {
                    return type_error("object is not extensible");
                }
            }
            None => {
                if !self.heap.create_data_property(o, key, v)? {
                    return type_error("object is not extensible");
                }
            }
        }
        self.sp -= n;
        Ok(None)
    }

    fn put_on_primitive(&mut self, o: Value, key: Atom) -> Step {
        if o.is_nullish() {
            return self.nullish_base(o, key, "set");
        }
        // A string's own `length` and indexes are read-only; anything else would be
        // created on a temporary wrapper object, which strict mode rejects.
        if let Some(s) = o.as_string()
            && (key == crate::heap::atoms::LENGTH
                || key.as_int().is_some_and(|i| i < self.heap.str_len(s)))
        {
            return type_error(&format!("'{}' is read-only", self.heap.atom_name(key)));
        }
        type_error("not an object")
    }

    /// The value of a global lexical binding (`None` if there is none); a ReferenceError
    /// while it is uninitialized (TDZ).
    fn global_lexical(&self, key: Atom) -> Result<Option<Value>, Fault> {
        let lex = realm::global_lex(self.heap);
        match self.heap.get_own_property(lex, key) {
            Some(Property::Data { value, .. }) if value.is_uninitialized() => Err(Fault::Error(
                ErrorKind::ReferenceError,
                format!("{} is not initialized", self.heap.atom_name(key)),
            )),
            Some(p) => Ok(p.value()),
            None => Ok(None),
        }
    }

    /// GlobalDeclarationInstantiation for the declaration list in constant `idx`, with
    /// QuickJS' checks (`JS_CheckDefineGlobalVar`): all checks first, then the bindings.
    fn global_decls(&mut self, idx: u32) -> Result<(), Fault> {
        let n = self.heap.value_array_len(self.consts);
        let consts: alloc::vec::Vec<Value> = (0..n)
            .map(|i| self.heap.value_array_get(self.consts, i))
            .collect();
        let decls = GlobalDecls::decode(&consts, idx as usize).ok_or_else(|| {
            Fault::Error(ErrorKind::InternalError, String::from("bad global_decls"))
        })?;
        let g = intrinsic_obj(self.heap, Intrinsic::Global);
        let lex = realm::global_lex(self.heap);
        let redeclaration = |heap: &Heap, k: Atom| {
            Err(Fault::Error(
                ErrorKind::SyntaxError,
                format!("redeclaration of '{}'", heap.atom_name(k)),
            ))
        };
        for &k in decls.lets.iter().chain(&decls.consts) {
            let own = self.heap.get_own_property(g, k);
            if own.is_some_and(|p| !p.flags().configurable()) || self.heap.has_own_property(lex, k)
            {
                return redeclaration(self.heap, k);
            }
        }
        for &k in decls.vars.iter().chain(&decls.funcs) {
            if !self.heap.has_own_property(g, k) && !self.heap.is_extensible(g) {
                return Err(Fault::Error(
                    ErrorKind::TypeError,
                    format!("cannot define variable '{}'", self.heap.atom_name(k)),
                ));
            }
            if self.heap.has_own_property(lex, k) {
                return redeclaration(self.heap, k);
            }
        }
        for &k in decls.vars.iter().chain(&decls.funcs) {
            if !self.heap.has_own_property(g, k) {
                let d = PropertyDescriptor::data(
                    Value::UNDEFINED,
                    PropFlags::WRITABLE | PropFlags::ENUMERABLE,
                );
                self.heap.define_own_property(g, k, &d)?;
            }
        }
        for (list, writable) in [(&decls.lets, true), (&decls.consts, false)] {
            for &k in list {
                realm::declare_global_lexical(self.heap, k, writable)?;
            }
        }
        Ok(())
    }

    /// `special_object kind`.
    fn special_object(&mut self, kind: u8) -> Result<Value, Fault> {
        Ok(match kind {
            special::THIS => self.get(self.bp + 1),
            special::NEW_TARGET => self.header(frame::NEW_TARGET),
            special::ACTIVE_FUNCTION => self.get(self.bp),
            special::ARGUMENTS => {
                // An unmapped (strict) arguments object.
                let argc = self.header_int(frame::ARGC);
                let proto = intrinsic_obj(self.heap, Intrinsic::ObjectProto);
                let a = self
                    .heap
                    .alloc_object(ObjectClass::Arguments, Some(proto), 0)?;
                for i in 0..argc {
                    let v = self.get(self.bp + 2 + i);
                    self.heap
                        .create_data_property(a, Atom::int(i).unwrap(), v)?;
                }
                realm::define_hidden(
                    self.heap,
                    a,
                    crate::heap::atoms::LENGTH,
                    Value::int(argc as i32),
                )?;
                Value::object(a)
            }
            _ => {
                return Err(Fault::Error(
                    ErrorKind::InternalError,
                    format!("not supported yet: special_object {kind}"),
                ));
            }
        })
    }

    /// `{...src}` (CopyDataProperties without excluded keys). Like QuickJS, primitive
    /// sources contribute nothing.
    fn copy_data_props(&mut self, o: Obj, src: Value) -> Result<(), Fault> {
        let Some(s) = src.as_object() else {
            return Ok(());
        };
        for k in self.heap.own_keys(s) {
            match self.heap.get_own_property(s, k) {
                Some(Property::Data { value, flags }) if flags.enumerable() => {
                    self.heap.create_data_property(o, k, value)?;
                }
                Some(Property::Accessor { flags, .. }) if flags.enumerable() => {
                    return type_error("not supported yet: getter in an object spread");
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The `delete` operator (strict: failure is a TypeError).
    fn delete(&mut self, o: Value, k: Value) -> Result<bool, Fault> {
        if o.is_nullish() {
            return type_error("cannot convert to object");
        }
        let key = self.property_key(o, k)?;
        let deleted = match (o.as_object(), o.as_string()) {
            (Some(obj), _) => self.heap.delete_own_property(obj, key),
            // A string's `length` and its index properties are not configurable.
            (None, Some(s)) => {
                !(key == crate::heap::atoms::LENGTH
                    || key.as_int().is_some_and(|i| i < self.heap.str_len(s)))
            }
            (None, None) => true,
        };
        if !deleted {
            return type_error("could not delete property");
        }
        Ok(true)
    }

    /// A method, getter or setter of an object literal: DefinePropertyOrThrow with
    /// enumerable and configurable set (methods are also writable). A getter or setter
    /// alone keeps the other half of an existing accessor.
    fn define_literal_member(&mut self, o: Obj, key: Atom, f: Value, op: Op) -> Result<(), Fault> {
        let d = match op {
            Op::DefineMethod | Op::DefineMethodElem => PropertyDescriptor::data(f, PropFlags::ALL),
            Op::DefineGetter | Op::DefineGetterElem => PropertyDescriptor {
                get: Some(f),
                enumerable: Some(true),
                configurable: Some(true),
                ..Default::default()
            },
            _ => PropertyDescriptor {
                set: Some(f),
                enumerable: Some(true),
                configurable: Some(true),
                ..Default::default()
            },
        };
        if !self.heap.define_own_property(o, key, &d)? {
            return type_error("could not define property");
        }
        Ok(())
    }

    /// `[...src]`: appends the elements of an iterable. Arrays (holes read as undefined)
    /// and strings (by code points) are done here; other iterables need the iterator
    /// protocol (0029's %GetIterator).
    fn array_spread(&mut self, arr: Obj, src: Value) -> Result<(), Fault> {
        let mut vals = alloc::vec::Vec::new();
        if let Some(s) = src.as_string() {
            let units = self.heap.str_to_utf16(s);
            let mut i = 0;
            while i < units.len() {
                let pair = (0xD800..0xDC00).contains(&units[i])
                    && units
                        .get(i + 1)
                        .is_some_and(|u| (0xDC00..0xE000).contains(u));
                let n = if pair { 2 } else { 1 };
                vals.push(Value::string(self.heap.alloc_utf16(&units[i..i + n])?));
                i += n;
            }
        } else if let Some(a) = src
            .as_object()
            .filter(|&a| self.heap.object_class(a) == ObjectClass::Array)
        {
            for i in 0..self.heap.array_length(a) {
                let v = match self.heap.get_element_fast(a, i) {
                    Some(v) => v,
                    None => {
                        let key = self.heap.atom_from_index(i)?;
                        match self.heap.find_property(a, key) {
                            None => Value::UNDEFINED,
                            Some((_, Property::Data { value, .. })) => value,
                            Some(_) => {
                                return type_error("not supported yet: getter in an array spread");
                            }
                        }
                    }
                };
                vals.push(v);
            }
        } else if src.as_object().is_some() {
            return type_error("not supported yet: spreading a non-array iterable");
        } else {
            return type_error("value is not iterable");
        }
        for v in vals {
            let len = self.heap.array_length(arr);
            if !self.heap.set_element_fast(arr, len, v)? {
                let k = self.heap.atom_from_index(len)?;
                self.heap.create_data_property(arr, k, v)?;
            }
        }
        Ok(())
    }

    /// Replaces the array at the top of the stack by its elements; returns their count.
    /// The compiler builds the array, so it is a plain array (holes read as undefined).
    fn spread_args(&mut self, args: Value) -> Result<u32, Fault> {
        let a = args.as_object().expect("spread arguments array");
        let n = self.heap.array_length(a);
        self.sp -= 1;
        self.ensure(self.sp + n)?;
        for i in 0..n {
            let v = match self.heap.get_element_fast(a, i) {
                Some(v) => v,
                None => {
                    let key = self.heap.atom_from_index(i)?;
                    self.heap
                        .find_property(a, key)
                        .and_then(|(_, p)| p.value())
                        .unwrap_or(Value::UNDEFINED)
                }
            };
            self.push(v);
        }
        Ok(n)
    }

    /// SetFunctionName(f, key, prefix): `name` is the key (`[description]` for symbols),
    /// after `get ` / `set ` for accessors.
    fn set_function_name(&mut self, f: Value, key: Atom, prefix: u8) -> Result<(), Fault> {
        let Some(fo) = f.as_object() else {
            return Ok(());
        };
        let base = if key.is_symbol() {
            let sym = self.heap.atom_symbol(key).unwrap();
            match self.heap.symbol_description(sym).as_string() {
                Some(d) => format!("[{}]", self.heap.str_to_string(d)),
                None => String::new(),
            }
        } else {
            self.heap.atom_name(key)
        };
        let name = match prefix {
            1 => format!("get {base}"),
            2 => format!("set {base}"),
            _ => base,
        };
        let s = self.heap.alloc_str(&name)?;
        let d = PropertyDescriptor::data(Value::string(s), PropFlags::CONFIGURABLE);
        self.heap
            .define_own_property(fo, crate::heap::atoms::NAME, &d)?;
        Ok(())
    }

    /// IsConstructor: ordinary (non-async, non-generator) functions and class
    /// constructors.
    fn is_constructor(&self, fo: Obj) -> bool {
        self.heap.object_class(fo) == ObjectClass::Function && {
            let t = self
                .heap
                .object_slot(fo, 0)
                .as_internal()
                .expect("template");
            let flags = self.heap.arena().get(t, FuncTemplate::FLAGS);
            flags & (func_flags::ASYNC | func_flags::GENERATOR) == 0
                && matches!(
                    func_flags::kind(flags),
                    func_flags::KIND_NORMAL | func_flags::KIND_CLASS_CTOR
                )
        }
    }

    /// QuickJS's `<name> is not a constructor` (just `not a constructor` without a name).
    fn not_a_constructor_message(&self, f: Value) -> String {
        let name = f
            .as_object()
            .and_then(|fo| self.heap.get_own_property(fo, crate::heap::atoms::NAME))
            .and_then(|p| p.value())
            .and_then(|v| v.as_string())
            .map(|s| self.heap.str_to_string(s))
            .filter(|s| !s.is_empty());
        match name {
            Some(n) => format!("{n} is not a constructor"),
            None => String::from("not a constructor"),
        }
    }

    /// `new f(args)`: `f a1 .. an` on the stack.
    fn construct(&mut self, argc: u32, next: u32) -> Step {
        let base = self.sp - 1 - argc;
        let f = self.get(base);
        let Some(fo) = f.as_object() else {
            return type_error("not a function");
        };
        if !self.is_constructor(fo) {
            if !self.heap.object_class(fo).is_callable() {
                return type_error("not a function");
            }
            return type_error(&self.not_a_constructor_message(f));
        }
        let proto = match self.heap.find_property(fo, crate::heap::atoms::PROTOTYPE) {
            Some((_, Property::Data { value, .. })) => value.as_object(),
            None => None,
            Some(_) => return type_error("not supported yet: getter for 'prototype'"),
        };
        let proto = proto.unwrap_or_else(|| intrinsic_obj(self.heap, Intrinsic::ObjectProto));
        let this = Value::object(self.heap.new_object(Some(proto))?);
        // Make room for `this` below the arguments.
        self.ensure(self.sp + 1)?;
        for i in (0..argc).rev() {
            let v = self.get(base + 1 + i);
            self.set(base + 2 + i, v);
        }
        self.set(base + 1, this);
        self.sp += 1;
        let r = self.call(base, argc, Continuation::ConstructResult, next)?;
        let flags = self.header_int(frame::FLAGS) as i32 | frame::FLAG_CONSTRUCT;
        self.set(self.hp + frame::FLAGS, Value::int(flags));
        self.set(self.hp + frame::NEW_TARGET, f);
        Ok(r)
    }

    /// OrdinaryHasInstance (no `Symbol.hasInstance` yet).
    fn instance_of(&mut self, v: Value, f: Value) -> Result<bool, Fault> {
        let Some(fo) = f
            .as_object()
            .filter(|&o| self.heap.object_class(o).is_callable())
        else {
            return type_error("invalid 'instanceof' right operand");
        };
        let Some(mut o) = v.as_object() else {
            return Ok(false);
        };
        let proto = match self.heap.find_property(fo, crate::heap::atoms::PROTOTYPE) {
            Some((_, Property::Data { value, .. })) => value,
            None => Value::UNDEFINED,
            Some(_) => return type_error("not supported yet: getter for 'prototype'"),
        };
        let Some(p) = proto.as_object() else {
            return type_error("operand 'prototype' property is not an object");
        };
        while let Some(q) = self.heap.get_prototype_of(o) {
            if q == p {
                return Ok(true);
            }
            o = q;
        }
        Ok(false)
    }
}

// ------------------------------------------------------------------ host calls

/// Adds a pending host call and returns its id.
fn add_host_call(heap: &mut Heap, fn_id: u32, args: u32, this: Value) -> Result<u32, HeapError> {
    let i = state::interp(heap);
    let id = heap.arena().get(i, Interp::NEXT_CALL_ID);
    let c = heap.alloc_cell(CellKind::HostCall, HostCall::LAYOUT.min_size)?;
    let table = state::host_calls(heap);
    let len = heap.value_array_len(table);
    let table2 = heap.value_array_reserve(table, len + 1)?;
    let a = heap.arena_mut();
    a.set(c, HostCall::ID, id);
    a.set(c, HostCall::FN, fn_id);
    a.set(c, HostCall::ARGS, Ref(args));
    a.set(c, HostCall::THIS, this);
    a.set(0, Roots::HOST_CALLS, Ref(table2));
    a.set(i, Interp::NEXT_CALL_ID, id.wrapping_add(1).max(1));
    heap.value_array_set(table2, len, Value::internal(c));
    heap.arena_mut().set(table2, ValueArray::LEN, len + 1);
    Ok(id)
}

/// The `HostCall` cell with `id`, if pending.
pub(crate) fn host_call_cell(heap: &Heap, id: u32) -> Option<u32> {
    let table = state::host_calls(heap);
    (0..heap.value_array_len(table))
        .map(|i| heap.value_array_get(table, i).as_internal().unwrap())
        .find(|&c| heap.arena().get(c, HostCall::ID) == id)
}

/// Removes a pending host call.
pub(crate) fn remove_host_call(heap: &mut Heap, id: u32) {
    let table = state::host_calls(heap);
    let len = heap.value_array_len(table);
    let Some(pos) = (0..len).find(|&i| {
        let c = heap.value_array_get(table, i).as_internal().unwrap();
        heap.arena().get(c, HostCall::ID) == id
    }) else {
        return;
    };
    for i in pos + 1..len {
        let v = heap.value_array_get(table, i);
        heap.value_array_set(table, i - 1, v);
    }
    heap.value_array_set(table, len - 1, Value::ZERO);
    heap.arena_mut().set(table, ValueArray::LEN, len - 1);
}

/// The parked call's continuation and whether it was the entry callee.
pub(crate) fn parked(heap: &Heap) -> (Continuation, bool) {
    let f = heap.arena().get(state::interp(heap), Interp::FLAGS);
    (
        Continuation::from_flags((f & !PARK_ENTRY) as i32),
        f & PARK_ENTRY != 0,
    )
}
