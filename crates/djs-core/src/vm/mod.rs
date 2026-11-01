//! The embedding API (ARCH "Execution API").
//!
//! ```text
//! Vm::new(opts)                      a fresh VM: heap, realm, interpreter state
//! vm.instantiate(template)           a closure for a compiled top-level template
//! vm.call(f, this, args)             pushes the entry frame; does not run it
//! vm.run(fuel) -> Status             Done | Threw | OutOfFuel | HostCall | Idle
//! vm.host_call(id)                   the fn id, `this` and arguments of a parked call
//! vm.resolve_host_call(id, result)   completes it; then `run` again
//! vm.bytes() / Vm::from_bytes(..)    the whole VM as a position-independent image
//! ```
//!
//! Values returned by the API (`Status::Done(v)`, `host_call` arguments) are raw heap
//! references: valid until the next `run` or collection. Keep them in a
//! [`Handle`](crate::heap::Handle) to hold them longer.

mod format;

use alloc::string::String;
use alloc::vec::Vec;

use crate::heap::cells::{HostCall, Interp, ValueArray};
use crate::heap::{
    Heap, HeapError, LoadError, ObjectClass, PropFlags, PropertyDescriptor, Ref, atoms,
};
use crate::interp::realm::{self, Intrinsic, define_hidden, intrinsic_obj};
use crate::interp::run::{self, Exec};
use crate::interp::{Continuation, state};
pub use crate::interp::{ErrorKind, ExecOptions, RunState, Status};
use crate::value::{Obj, Value};

/// VM configuration.
#[derive(Clone, Copy, Debug)]
pub struct VmOptions {
    /// The heap's memory cap in bytes.
    pub memory_limit: usize,
    pub exec: ExecOptions,
}

impl Default for VmOptions {
    fn default() -> Self {
        VmOptions {
            memory_limit: crate::heap::DEFAULT_LIMIT,
            exec: ExecOptions::default(),
        }
    }
}

/// Misuse of the API.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VmError {
    /// `call` while a call is in progress (running or parked).
    Busy,
    /// `resolve_host_call` with an id the VM is not parked on.
    UnknownHostCall,
    Heap(HeapError),
}

impl From<HeapError> for VmError {
    fn from(e: HeapError) -> Self {
        VmError::Heap(e)
    }
}

/// A parked host call, as the embedder sees it.
#[derive(Clone, Debug)]
pub struct HostCallInfo {
    pub fn_id: u32,
    pub this: Value,
    pub args: Vec<Value>,
}

/// A VM: a heap with a realm and interpreter state. Everything is in the heap; the
/// struct only holds options.
pub struct Vm {
    heap: Heap,
    opts: VmOptions,
}

impl Vm {
    pub fn new(opts: VmOptions) -> Result<Vm, HeapError> {
        let mut heap = Heap::with_limit(opts.memory_limit)?;
        state::init(&mut heap)?;
        realm::init(&mut heap)?;
        Ok(Vm { heap, opts })
    }

    /// Restores a VM from [`Vm::bytes`] of the same build, in any state (including
    /// suspended mid-call or parked on a host call).
    pub fn from_bytes(bytes: Vec<u8>, opts: VmOptions) -> Result<Vm, LoadError> {
        let heap = Heap::from_bytes(bytes, opts.memory_limit)?;
        if state::interp(&heap) == 0 || heap.realm().as_internal().is_none() {
            return Err(LoadError::BadHeader);
        }
        Ok(Vm { heap, opts })
    }

    /// The complete VM image.
    pub fn bytes(&self) -> &[u8] {
        self.heap.bytes()
    }

    pub fn heap(&self) -> &Heap {
        &self.heap
    }

    pub fn heap_mut(&mut self) -> &mut Heap {
        &mut self.heap
    }

    pub fn options(&self) -> VmOptions {
        self.opts
    }

    pub fn set_options(&mut self, opts: VmOptions) {
        self.opts = opts;
    }

    pub fn state(&self) -> RunState {
        state::state(&self.heap)
    }

    /// The global object.
    pub fn global(&self) -> Obj {
        intrinsic_obj(&self.heap, Intrinsic::Global)
    }

    /// A closure for a top-level `FuncTemplate` (one without captures).
    pub fn instantiate(&mut self, template: Ref) -> Result<Value, HeapError> {
        let proto = intrinsic_obj(&self.heap, Intrinsic::FunctionProto);
        let f = self
            .heap
            .alloc_object(ObjectClass::Function, Some(proto), 1)?;
        self.heap.set_object_slot(f, 0, Value::internal(template.0));
        Ok(Value::object(f))
    }

    /// A function implemented by the embedder: calling it parks the VM with
    /// `Status::HostCall { fn_id, .. }`.
    pub fn new_host_function(&mut self, fn_id: u32, name: &str) -> Result<Value, HeapError> {
        let proto = intrinsic_obj(&self.heap, Intrinsic::FunctionProto);
        let f = self
            .heap
            .alloc_object(ObjectClass::HostFunction, Some(proto), 1)?;
        self.heap.set_object_slot(f, 0, Value::int(fn_id as i32));
        let n = self.heap.alloc_str(name)?;
        let d = PropertyDescriptor::data(Value::string(n), PropFlags::CONFIGURABLE);
        self.heap.define_own_property(f, atoms::NAME, &d)?;
        Ok(Value::object(f))
    }

    /// A new ordinary object.
    pub fn new_object(&mut self) -> Result<Obj, HeapError> {
        let proto = intrinsic_obj(&self.heap, Intrinsic::ObjectProto);
        self.heap.new_object(Some(proto))
    }

    /// Defines a property of an object the way builtins are defined (writable,
    /// configurable, not enumerable).
    pub fn define(&mut self, o: Obj, name: &str, v: Value) -> Result<(), HeapError> {
        let key = self.heap.atom_str(name)?;
        define_hidden(&mut self.heap, o, key, v)
    }

    /// Defines a global binding (a property of the global object).
    pub fn define_global(&mut self, name: &str, v: Value) -> Result<(), HeapError> {
        let g = self.global();
        self.define(g, name, v)
    }

    /// A new Error object of `kind`.
    pub fn new_error(&mut self, kind: ErrorKind, message: &str) -> Result<Value, HeapError> {
        realm::new_error(&mut self.heap, kind, message)
    }

    /// Starts a call of `f`: pushes the entry frame (or parks on a host function). `run`
    /// executes it.
    pub fn call(&mut self, f: Value, this: Value, args: &[Value]) -> Result<(), VmError> {
        match self.state() {
            RunState::Running | RunState::HostCall | RunState::ThrowPending => {
                return Err(VmError::Busy);
            }
            RunState::Idle | RunState::Done | RunState::Threw => {}
        }
        let i = state::interp(&self.heap);
        for f in [Interp::BP, Interp::HP, Interp::SP, Interp::PC] {
            self.heap.arena_mut().set(i, f, 0);
        }
        let argc = u32::try_from(args.len()).map_err(|_| HeapError::OutOfMemory)?;
        state::set_state(&mut self.heap, RunState::Running);
        let mut ex = Exec::new(&mut self.heap, self.opts.exec);
        let r = ex.ensure(2 + argc).and_then(|()| {
            ex.push(f);
            ex.push(this);
            for &a in args {
                ex.push(a);
            }
            ex.call(0, argc, Continuation::Entry, 0)
        });
        match r {
            Ok(_) => ex.flush(),
            Err(fault) => {
                let exc = ex.fault_value(fault);
                ex.set_sp(0);
                ex.flush();
                state::set_result(&mut self.heap, exc);
                state::set_state(&mut self.heap, RunState::Threw);
            }
        }
        Ok(())
    }

    /// Runs with `fuel` (instruction cost units).
    pub fn run(&mut self, fuel: u64) -> Status {
        Exec::new(&mut self.heap, self.opts.exec).run(fuel)
    }

    /// The parked host call `call_id`.
    pub fn host_call(&self, call_id: u32) -> Option<HostCallInfo> {
        let c = run::host_call_cell(&self.heap, call_id)?;
        let a = self.heap.arena();
        let args: Ref = a.get(c, HostCall::ARGS);
        let n = a.get(args.0, ValueArray::LEN);
        Some(HostCallInfo {
            fn_id: a.get(c, HostCall::FN),
            this: a.get(c, HostCall::THIS),
            args: (0..n)
                .map(|i| self.heap.value_array_get(args.0, i))
                .collect(),
        })
    }

    /// Completes the parked host call `call_id` with a value (`Ok`) or an exception
    /// (`Err`, thrown at the call). `run` continues afterwards.
    pub fn resolve_host_call(
        &mut self,
        call_id: u32,
        result: Result<Value, Value>,
    ) -> Result<(), VmError> {
        let i = state::interp(&self.heap);
        if self.state() != RunState::HostCall
            || self.heap.arena().get(i, Interp::HOST_CALL) != call_id
        {
            return Err(VmError::UnknownHostCall);
        }
        let (cont, entry) = run::parked(&self.heap);
        run::remove_host_call(&mut self.heap, call_id);
        self.heap.arena_mut().set(i, Interp::HOST_CALL, 0);
        if entry {
            let (s, v) = match result {
                Ok(v) => (RunState::Done, v),
                Err(e) => (RunState::Threw, e),
            };
            state::set_result(&mut self.heap, v);
            state::set_state(&mut self.heap, s);
            return Ok(());
        }
        match result {
            Ok(v) => {
                if cont == Continuation::PushResult {
                    let mut ex = Exec::new(&mut self.heap, self.opts.exec);
                    ex.push(v);
                    ex.flush();
                }
                state::set_state(&mut self.heap, RunState::Running);
            }
            Err(e) => {
                state::set_result(&mut self.heap, e);
                state::set_state(&mut self.heap, RunState::ThrowPending);
            }
        }
        Ok(())
    }

    /// Formats a value the way `console.log` shows it in `djs run`: strings raw, Errors
    /// as `Name: message`, everything else as JSON with a `String()`-like fallback.
    pub fn display(&mut self, v: Value) -> String {
        format::display(&mut self.heap, v)
    }

    /// Collects garbage now. Raw values held outside the heap are invalid afterwards.
    pub fn collect(&mut self) -> Result<(), HeapError> {
        self.heap.collect().map(|_| ())
    }
}

#[cfg(test)]
mod tests;
