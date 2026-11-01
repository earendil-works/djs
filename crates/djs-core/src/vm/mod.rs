//! The embedding API (ARCH "Execution API").
//!
//! ```text
//! Vm::new(opts)                      a fresh VM: heap, realm, interpreter state
//! vm.instantiate(template)           a closure for a compiled top-level template
//! vm.call(f, this, args)             pushes the entry frame; does not run it
//! vm.run(fuel) -> Status             Done | Threw | OutOfFuel | HostCall | Idle
//!                                    (the job queue runs after the entry call, D1370)
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

/// `HostCall::FLAGS`: returned by `Vm::new_host_calls` already.
const HOST_CALL_REPORTED: u32 = 1;

/// The generated prelude (`builtins/gen/prelude.js`, D1172).
const PRELUDE: &str = include_str!("../../../../builtins/gen/prelude.js");

// `SOURCE_HASH`: the library's non-test sources and the prelude (build.rs, D1330).
include!(concat!(env!("OUT_DIR"), "/source_hash.rs"));

/// The build hash of VM images (D1320, D1330): the heap layout (with the well-known
/// atoms), the instruction table, the intrinsic ids and the source hash. Images hold
/// bytecode and ids of all kinds (realm slots, object classes, flags), so they load only
/// into the build that made them. Stored in `Roots::BUILD`.
pub const BUILD_HASH: u32 = {
    use crate::bytecode::intrinsics::INTRINSICS_HASH;
    use crate::bytecode::{OPS_HASH, build_hash};
    build_hash(
        build_hash(
            build_hash(crate::heap::cells::LAYOUT_HASH, OPS_HASH),
            INTRINSICS_HASH,
        ),
        SOURCE_HASH,
    )
};

/// VM configuration.
#[derive(Clone, Copy, Debug)]
pub struct VmOptions {
    /// The heap's memory cap in bytes.
    pub memory_limit: usize,
    pub exec: ExecOptions,
    /// Install the builtins of `builtins/gen/prelude.js` at creation (default). Off gives
    /// a bare realm (interpreter tests, measuring the prelude's cost).
    pub prelude: bool,
    /// The seed of `Math.random` (ARCH "Nondeterminism": the host decides; 0 means the
    /// fixed default). Applied by `Vm::new` and after copying a base (`from_base`); a
    /// restored snapshot (`from_bytes`) continues its own sequence.
    pub random_seed: u64,
}

impl Default for VmOptions {
    fn default() -> Self {
        VmOptions {
            memory_limit: crate::heap::DEFAULT_LIMIT,
            exec: ExecOptions::default(),
            prelude: true,
            random_seed: crate::builtins::number::DEFAULT_RANDOM_SEED,
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

/// A pending host call, as the embedder sees it: the call the VM is parked on, or an
/// async call (D1374).
#[derive(Clone, Debug)]
pub struct HostCallInfo {
    pub fn_id: u32,
    pub this: Value,
    pub args: Vec<Value>,
    /// An async host call: the script got a promise and went on.
    pub is_async: bool,
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
        heap.arena_mut()
            .set(0, crate::heap::cells::Roots::BUILD, BUILD_HASH);
        state::init(&mut heap)?;
        realm::init(&mut heap)?;
        let mut vm = Vm { heap, opts };
        crate::builtins::number::set_seed(&mut vm.heap, opts.random_seed);
        if opts.prelude {
            vm.install_prelude()?;
        }
        Ok(vm)
    }

    /// Compiles the generated prelude in privileged mode and runs it (D1172, D1281): it
    /// installs the builtin methods and registers the `@intrinsic` builtins. Compiled at
    /// every VM creation until there is a base snapshot (ARCH G2).
    fn install_prelude(&mut self) -> Result<(), HeapError> {
        use crate::compiler::{CompileErrorKind, CompileOptions};
        let opts = CompileOptions::privileged_script("<prelude>");
        let template = match crate::compiler::compile(&mut self.heap, PRELUDE.as_bytes(), &opts) {
            Ok(t) => t,
            Err(e) if e.kind == CompileErrorKind::OutOfMemory => {
                return Err(HeapError::OutOfMemory);
            }
            Err(e) => panic!("the prelude does not compile: {e}"),
        };
        let script: Ref = self
            .heap
            .arena()
            .get(template.0, crate::heap::cells::FuncTemplate::SCRIPT);
        let r = realm::realm(&self.heap);
        self.heap.arena_mut().write(
            crate::heap::cells::Realm::INTRINSICS.at(r, Intrinsic::PreludeScript as u32),
            Value::internal(script.0),
        );
        let f = self.instantiate(template)?;
        let this = Value::object(self.global());
        self.call(f, this, &[])
            .map_err(|_| HeapError::OutOfMemory)?;
        match self.run(u64::MAX) {
            Status::Done(_) => {}
            Status::Threw(e)
                if e == crate::interp::intrinsic(&self.heap, Intrinsic::OutOfMemoryError) =>
            {
                return Err(HeapError::OutOfMemory);
            }
            s => {
                let shown = match s {
                    Status::Threw(e) => self.display(e),
                    _ => alloc::format!("{s:?}"),
                };
                panic!("the prelude failed: {shown}");
            }
        }
        state::set_state(&mut self.heap, RunState::Idle);
        Ok(())
    }

    /// Restores a VM from [`Vm::bytes`] of the same build, in any state (including
    /// suspended mid-call or parked on a host call). The whole image is verified.
    pub fn from_bytes(bytes: Vec<u8>, opts: VmOptions) -> Result<Vm, LoadError> {
        Vm::check_image(&bytes)?;
        Vm::over(Heap::from_bytes(bytes, opts.memory_limit)?, opts)
    }

    /// A new VM from a base snapshot: the image of an idle VM with its prelude installed,
    /// made at build time (`djs-prelude`, D1321). Creating a VM is then a copy, with no
    /// compile. The image is trusted (its cells are not verified: it was verified when it
    /// was built), but its header and build hash are checked.
    ///
    /// What the base holds: the realm (global object, intrinsics, builtins), the atoms
    /// and the compiled prelude. Nothing per VM: no handles, no host functions, no
    /// pending calls. Per-VM state is set up after the copy (see `after_copy`).
    pub fn from_base(base: &[u8], opts: VmOptions) -> Result<Vm, LoadError> {
        Vm::check_image(base)?;
        let heap = Heap::from_bytes_trusted(base.to_vec(), opts.memory_limit)?;
        let mut vm = Vm::over(heap, opts)?;
        if vm.state() != RunState::Idle || vm.heap.handle_count() != 0 {
            return Err(LoadError::BadHeader);
        }
        vm.after_copy();
        Ok(vm)
    }

    /// Per-VM initialization after copying a base: state that must not be shared between
    /// VMs or baked into the base: the `Math.random` seed.
    fn after_copy(&mut self) {
        let seed = self.opts.random_seed;
        crate::builtins::number::set_seed(&mut self.heap, seed);
    }

    /// The build hash, before anything else: an image from another build must be
    /// rejected with a clear error, not fail verification somewhere in the middle.
    fn check_image(bytes: &[u8]) -> Result<(), LoadError> {
        use crate::heap::cells::{self, Roots};
        let u32_at = |at: u32| {
            let at = at as usize;
            bytes
                .get(at..at + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        if u32_at(Roots::MAGIC.offset) != Some(cells::MAGIC) {
            return Err(LoadError::BadHeader);
        }
        let found = u32_at(Roots::BUILD.offset).ok_or(LoadError::BadHeader)?;
        if found != BUILD_HASH {
            return Err(LoadError::BuildMismatch {
                found,
                expected: BUILD_HASH,
            });
        }
        Ok(())
    }

    fn over(heap: Heap, opts: VmOptions) -> Result<Vm, LoadError> {
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

    /// Compiles `src` (UTF-8) and returns the top-level function, ready for
    /// [`Vm::call`]. Call it with `this` = the global object; in
    /// [`Goal::AsyncBody`](crate::compiler::Goal) mode, pass the parameters as arguments.
    /// Until async functions exist, an async body runs as a plain function body.
    pub fn compile(
        &mut self,
        src: &[u8],
        opts: &crate::compiler::CompileOptions<'_>,
    ) -> Result<Value, crate::compiler::CompileError> {
        let template = crate::compiler::compile(&mut self.heap, src, opts)?;
        self.instantiate(template)
            .map_err(|_| crate::compiler::CompileError {
                kind: crate::compiler::CompileErrorKind::OutOfMemory,
                message: String::from("out of memory"),
                pos: 0,
                line: 0,
                col: 0,
            })
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

    /// An async host function (D1374): calling it returns a promise at once and adds a
    /// pending call (see [`Vm::new_host_calls`]); the VM goes on running. Resolving the
    /// call with [`Vm::resolve_host_call`] settles the promise.
    pub fn new_async_host_function(&mut self, fn_id: u32, name: &str) -> Result<Value, HeapError> {
        let f = self.new_host_function(fn_id, name)?;
        let o = f.as_object().expect("a function object");
        let flags = self.heap.cell_flags(o.offset());
        self.heap.set_cell_flags(
            o.offset(),
            flags | crate::heap::cells::flags::OBJ_HOST_ASYNC,
        );
        Ok(f)
    }

    /// The async host calls made since the last `new_host_calls`, oldest first. Read
    /// them whenever `run` returns, with [`Vm::host_call`], and start them.
    pub fn new_host_calls(&mut self) -> Vec<u32> {
        let calls = self.async_calls();
        let mut out = Vec::new();
        for c in calls {
            let flags = self.heap.arena().get(c, HostCall::FLAGS);
            if flags & HOST_CALL_REPORTED == 0 {
                self.heap
                    .arena_mut()
                    .set(c, HostCall::FLAGS, flags | HOST_CALL_REPORTED);
                out.push(self.heap.arena().get(c, HostCall::ID));
            }
        }
        out
    }

    /// Every async host call that is not resolved yet, oldest first (after a restore,
    /// the embedder re-issues or replays these).
    pub fn pending_host_calls(&self) -> Vec<u32> {
        self.async_calls()
            .into_iter()
            .map(|c| self.heap.arena().get(c, HostCall::ID))
            .collect()
    }

    /// The `HostCall` cells of the pending async calls.
    fn async_calls(&self) -> Vec<u32> {
        let table = state::host_calls(&self.heap);
        (0..self.heap.value_array_len(table))
            .map(|i| self.heap.value_array_get(table, i).as_internal().unwrap())
            .filter(|&c| {
                self.heap
                    .arena()
                    .get::<Value>(c, HostCall::PROMISE)
                    .as_object()
                    .is_some()
            })
            .collect()
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
    /// Sets the clock that `Date.now()` and `new Date()` read, in ms since the epoch
    /// (ARCH "Nondeterminism", D1383). The VM never reads a system clock: the host sets
    /// it, typically before each run slice. It is heap data, so restores and replays see
    /// the value the host set. Until then it is 0.
    pub fn set_now(&mut self, ms: f64) {
        let v = Value::number(ms);
        crate::interp::realm::set_intrinsic(&mut self.heap, crate::interp::Intrinsic::Clock, v);
    }

    /// Sets the local time zone as a fixed offset in minutes east of UTC (D1383): local
    /// Date methods and toString use it. Default 0 (UTC). There is no daylight saving
    /// rule; a host that cares updates it.
    pub fn set_time_zone_offset(&mut self, minutes: i32) {
        let v = Value::int(minutes);
        crate::interp::realm::set_intrinsic(
            &mut self.heap,
            crate::interp::Intrinsic::TimeZoneOffset,
            v,
        );
    }

    /// Reseeds `Math.random` (see `VmOptions::random_seed`). The state lives in the
    /// heap, so a snapshot continues the same sequence.
    pub fn set_random_seed(&mut self, seed: u64) {
        crate::builtins::number::set_seed(&mut self.heap, seed);
    }

    pub fn new_error(&mut self, kind: ErrorKind, message: &str) -> Result<Value, HeapError> {
        realm::new_error(&mut self.heap, kind, message)
    }

    /// Starts a call of `f`: pushes the entry frame (or parks on a host function). `run`
    /// executes it. Never collects, so `f`, `this` and the arguments the embedder holds
    /// stay valid until the next `run`.
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
                let exc = ex.fault_value(fault, None);
                ex.set_sp(0);
                ex.flush();
                state::set_outcome(&mut self.heap, RunState::Threw, exc);
            }
        }
        Ok(())
    }

    /// Abandons the current call, if any (running, out of fuel, or parked on a host
    /// call): its frames, pending host calls, queued jobs and tracked rejections are
    /// dropped and the VM is `Idle` again,
    /// with its realm and globals intact. For hosts that stop a run that used up its fuel
    /// budget.
    pub fn abort(&mut self) {
        let i = state::interp(&self.heap);
        let stack = state::stack(&self.heap);
        let sp = self.heap.arena().get(i, Interp::SP);
        // Popped frames must not keep garbage alive.
        for k in 0..sp.min(self.heap.value_array_capacity(stack)) {
            self.heap.value_array_set(stack, k, Value::UNDEFINED);
        }
        let a = self.heap.arena_mut();
        for f in [
            Interp::BP,
            Interp::HP,
            Interp::SP,
            Interp::PC,
            Interp::HOST_CALL,
            Interp::FLAGS,
        ] {
            a.set(i, f, 0);
        }
        let calls = state::host_calls(&self.heap);
        let n = self.heap.value_array_len(calls);
        for k in 0..n {
            self.heap.value_array_set(calls, k, Value::ZERO);
        }
        self.heap.arena_mut().set(calls, ValueArray::LEN, 0);
        run::promise::clear_jobs(&mut self.heap);
        run::promise::clear_rejections(&mut self.heap);
        state::set_pending_throw(&mut self.heap, Value::UNDEFINED);
        state::set_outcome(&mut self.heap, RunState::Idle, Value::UNDEFINED);
    }

    /// Runs with `fuel` (instruction cost units).
    pub fn run(&mut self, fuel: u64) -> Status {
        Exec::new(&mut self.heap, self.opts.exec).run(fuel)
    }

    /// The pending host call `call_id` (parked or async).
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
            is_async: a.get::<Value>(c, HostCall::PROMISE).as_object().is_some(),
        })
    }

    /// Completes the host call `call_id` with a value (`Ok`) or an exception (`Err`).
    /// The call the VM is parked on returns or throws at the call; `run` continues
    /// afterwards. An async call (D1374) settles its promise in a job (in any state:
    /// the job runs at the next `run`); a value is adopted like a resolve function's
    /// argument (thenables too), an exception rejects.
    pub fn resolve_host_call(
        &mut self,
        call_id: u32,
        result: Result<Value, Value>,
    ) -> Result<(), VmError> {
        if let Some(c) = run::host_call_cell(&self.heap, call_id) {
            let p: Value = self.heap.arena().get(c, HostCall::PROMISE);
            if let Some(p) = p.as_object() {
                run::promise::enqueue_host_settle(&mut self.heap, p, result)?;
                run::remove_host_call(&mut self.heap, call_id);
                return Ok(());
            }
        }
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
            state::set_outcome(&mut self.heap, s, v);
            return Ok(());
        }
        match result {
            Ok(v) => {
                let mut ex = Exec::new(&mut self.heap, self.opts.exec);
                match cont {
                    Continuation::PushResult => ex.push(v),
                    Continuation::Replace(slot) => ex.set_slot(slot, v),
                    _ => {}
                }
                ex.flush();
                state::set_state(&mut self.heap, RunState::Running);
            }
            Err(e) => {
                state::set_pending_throw(&mut self.heap, e);
                state::set_state(&mut self.heap, RunState::ThrowPending);
            }
        }
        Ok(())
    }

    /// The state of a Promise: `None` if `v` is not a promise, `Some(None)` while it is
    /// pending, else its result (`Ok` fulfilled, `Err` rejected). For hosts whose entry
    /// function returns a promise (an async body).
    #[allow(clippy::option_option)]
    pub fn promise_state(&self, v: Value) -> Option<Option<Result<Value, Value>>> {
        use run::promise;
        let p = promise::as_promise(&self.heap, v)?;
        let r = promise::result_of(&self.heap, p);
        Some(match promise::state_of(&self.heap, p) {
            promise::FULFILLED => Some(Ok(r)),
            promise::REJECTED => Some(Err(r)),
            _ => None,
        })
    }

    /// Marks a promise handled (as a `then` would, D1423): its rejection is not reported
    /// by `take_unhandled_rejections`. For an embedder that consumes the outcome itself,
    /// such as an async body's result promise. No-op for other values.
    pub fn mark_promise_handled(&mut self, v: Value) {
        if let Some(p) = run::promise::as_promise(&self.heap, v) {
            run::promise::mark_handled(&mut self.heap, p);
        }
    }

    /// The reasons of the promises that were rejected without a handler and still have
    /// none (HostPromiseRejectionTracker, D1373), oldest first; they are forgotten. Check
    /// after `run` returns `Done`, `Threw` or `Idle` (the job queue is empty then), the way
    /// qjs reports `Possibly unhandled promise rejection`.
    pub fn take_unhandled_rejections(&mut self) -> Vec<Value> {
        run::promise::take_rejections(&mut self.heap)
            .into_iter()
            .map(|p| run::promise::result_of(&self.heap, p))
            .collect()
    }

    /// Formats a value the way `console.log` shows it in `djs run`: strings raw, Errors
    /// as `Name: message`, everything else as JSON with a `String()`-like fallback.
    pub fn display(&mut self, v: Value) -> String {
        format::display(&mut self.heap, v)
    }

    /// The `stack` of an Error object (own data property), if it has one.
    pub fn error_stack(&self, v: Value) -> Option<String> {
        let o = v.as_object()?;
        let s = self
            .heap
            .get_own_property(o, atoms::STACK)?
            .value()?
            .as_string()?;
        Some(self.heap.str_to_string(s))
    }

    /// Collects garbage now. Raw values held outside the heap are invalid afterwards.
    pub fn collect(&mut self) -> Result<(), HeapError> {
        self.heap.collect().map(|_| ())
    }
}

#[cfg(test)]
mod tests;
