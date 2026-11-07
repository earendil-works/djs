//! [`Instance`]: a VM made from a snapshot, and the call protocol.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use djs_core::heap::Handle;
use djs_core::interp::{Intrinsic, intrinsic};
use djs_core::value::Value as Raw;
use djs_core::vm::{Status, Vm, VmError};

use crate::snapshot::vm_options;
use crate::value::to_raw;
use crate::{Data, Error, Exception, Snapshot, Throw, Value, ValueRef};

/// The budgets of an instance's calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The heap cap in `bytes`; reaching it ends the call with [`Error::OutOfMemory`],
    /// which scripts cannot catch
    pub memory: usize,
    /// Fuel per call (instruction cost units, about 4 ns each); [`Error::OutOfFuel`] beyond
    pub fuel: u64,
    /// Fuel per [`Instance::step`]: how often the host gets control back
    pub slice: u64,
    /// Wall-clock time per call, checked before each slice ([`Error::Deadline`]); `None`:
    /// no deadline
    pub deadline: Option<Duration>,
}

impl Default for Limits {
    /// 64 MiB, 50M fuel (about 0.2 s), slices of 200k fuel (about 1 ms), no deadline.
    fn default() -> Self {
        Limits {
            memory: 64 << 20,
            fuel: 50_000_000,
            slice: 200_000,
            deadline: None,
        }
    }
}

/// A pending host call's id: unique among the instance's open calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallId(u32);

impl CallId {
    pub fn get(self) -> u32 {
        self.0
    }
}

/// What [`Instance::step`] stopped at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// The call settled with a value: read it with [`Instance::result`]. (A throw or a
    /// rejection is [`Error::Exception`] instead.)
    Done,
    /// A sync host call: answer it ([`Instance::host_call`], [`HostCall::reply`] or
    /// [`Instance::resolve`]), then step again
    HostCall(CallId),
    /// Nothing can run until an async host call is answered. Lists the calls made since
    /// the last `Wait`: start them, answer at least one, then step again
    Wait(Vec<CallId>),
    /// The slice's fuel is used up: step again (after yielding the thread, if the host
    /// shares it)
    Yield,
}

/// The current call.
struct Call {
    /// When the call's deadline passes (read the clock only for calls that have one)
    deadline: Option<Instant>,
    fuel_used: u64,
    /// The entry's return value once it returned: the promise it settles with, or the
    /// result itself
    entry: Option<Handle>,
    /// The settled value
    result: Option<Handle>,
    /// Over (settled or failed): nothing more runs
    over: bool,
}

/// A VM in a snapshot's state: runs one call at a time. `Send` (move it to another thread
/// between calls or steps), not `Sync`.
pub struct Instance {
    pub(crate) vm: Vm,
    snapshot: Snapshot,
    limits: Limits,
    call: Option<Call>,
    poisoned: bool,
}

/// Runs engine code, turning a panic into [`Error::Panicked`] (the caller poisons what
/// the code ran on).
pub(crate) fn guard_vm<R>(f: impl FnOnce() -> R) -> Result<R, Error> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|p| {
        let msg = p
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "a panic without a message".to_owned());
        Error::Panicked(msg)
    })
}

/// A thrown value as an [`Error`]: the cap's out-of-memory error is [`Error::OutOfMemory`].
pub(crate) fn exception(vm: &mut Vm, e: Raw) -> Error {
    if e == intrinsic(vm.heap(), Intrinsic::OutOfMemoryError) {
        return Error::OutOfMemory;
    }
    Error::Exception(Exception {
        message: vm.display(e),
        stack: vm.error_stack(e),
    })
}

impl Instance {
    pub(crate) fn new(snapshot: Snapshot, limits: Limits) -> Result<Instance, Error> {
        let opts = vm_options(limits.memory, snapshot.frozen_builtins());
        let bytes = snapshot.image_buffer(limits.memory);
        let vm = guard_vm(|| Vm::from_bytes_trusted(bytes, opts))?.map_err(|e| match e {
            djs_core::heap::LoadError::TooLarge => Error::OutOfMemory,
            e => Error::Usage(format!("cannot load the snapshot: {e:?}")),
        })?;
        Ok(Instance {
            vm,
            snapshot,
            limits,
            call: None,
            poisoned: false,
        })
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Sets the budgets: fuel, slice and deadline from the next call on, the memory cap
    /// from the next [`Instance::reset`].
    pub fn set_limits(&mut self, limits: Limits) {
        self.limits = limits;
        let opts = vm_options(limits.memory, self.snapshot.frozen_builtins());
        self.vm.set_options(opts);
    }

    /// Whether a panic poisoned the instance: every use fails with [`Error::Poisoned`].
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Runs engine code on the VM; a panic poisons the instance.
    pub(crate) fn guard<R>(&mut self, f: impl FnOnce(&mut Vm) -> R) -> Result<R, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        let vm = &mut self.vm;
        guard_vm(|| f(vm)).inspect_err(|_| self.poisoned = true)
    }

    /// Back to the snapshot's state (globals, heap, clock), reusing the heap's allocation:
    /// what a fresh instance would be, without its allocation. Ends a call in progress.
    pub fn reset(&mut self) -> Result<(), Error> {
        self.call = None;
        let image = self.snapshot.clone();
        match self.guard(|vm| vm.reset_to(image.image()))? {
            Ok(()) => Ok(()),
            Err(e) => {
                // The VM has no heap left.
                self.poisoned = true;
                Err(Error::Usage(format!("cannot load the snapshot: {e:?}")))
            }
        }
    }

    /// Sets the clock `Date.now()` and `new Date()` read, in ms since the epoch. The VM
    /// never reads a system clock; a snapshot's clock is what setup left (0 unless set).
    pub fn set_now(&mut self, ms: f64) -> Result<(), Error> {
        self.guard(|vm| vm.set_now(ms))
    }

    /// Fuel the current (or last) call used so far.
    pub fn fuel_used(&self) -> u64 {
        self.call.as_ref().map_or(0, |c| c.fuel_used)
    }

    /// When the current call's deadline passes, if it has one: for a host that awaits an
    /// async host call's work, to give up in time.
    pub fn deadline(&self) -> Option<Instant> {
        let c = self.call.as_ref()?;
        c.deadline
    }

    /// Drops the handles of the previous call.
    fn clear_call(&mut self) -> Result<(), Error> {
        if let Some(c) = self.call.take() {
            self.guard(|vm| {
                for h in [c.entry, c.result].into_iter().flatten() {
                    vm.heap_mut().drop_handle(h);
                }
            })?;
        }
        Ok(())
    }

    /// Ends the call with `e`: the VM is aborted (idle, nothing left of the call).
    fn fail<T>(&mut self, e: Error) -> Result<T, Error> {
        if !self.poisoned {
            let _ = self.guard(|vm| vm.abort());
        }
        if let Some(c) = self.call.as_mut() {
            c.over = true;
        }
        Err(e)
    }

    /// Starts calling the function at the dotted global `path` (`"onRequest"`,
    /// `"hooks.onRequest"`; own data properties only, no getters run) with `this` the
    /// object it was found on. Then [`Instance::step`] until [`Step::Done`] or an error,
    /// or use [`Instance::call`].
    pub fn start(&mut self, path: &str, args: &[Value<'_>]) -> Result<(), Error> {
        if self.call.as_ref().is_some_and(|c| !c.over) {
            return Err(Error::Usage("a call is in progress".into()));
        }
        self.clear_call()?;
        let deadline = self.limits.deadline.map(|d| Instant::now() + d);
        let r = self.guard(|vm| -> Result<(), Error> {
            let (this, f) = lookup(vm, path)?;
            let f = match f.as_object() {
                Some(o) if vm.heap().is_callable(o) => f,
                _ => return Err(Error::Usage(format!("{path} is not a function"))),
            };
            let mut raw = Vec::with_capacity(args.len());
            for a in args {
                raw.push(to_raw(vm, a)?);
            }
            vm.call(f, this, &raw).map_err(|e| match e {
                VmError::Heap(_) => Error::OutOfMemory,
                e => Error::Usage(format!("{e:?}")),
            })
        })?;
        self.call = Some(Call {
            deadline,
            fuel_used: 0,
            entry: None,
            result: None,
            over: false,
        });
        match r {
            Ok(()) => Ok(()),
            Err(e) => self.fail(e),
        }
    }

    /// Runs one slice of the current call (at most [`Limits::slice`] fuel).
    pub fn step(&mut self) -> Result<Step, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        let Some(c) = self.call.as_ref() else {
            return Err(Error::Usage("no call in progress".into()));
        };
        if c.result.is_some() {
            return Ok(Step::Done);
        }
        if c.over {
            return Err(Error::Usage("the call is over".into()));
        }
        if let Some(d) = c.deadline
            && Instant::now() >= d
        {
            return self.fail(Error::Deadline);
        }
        let left = self.limits.fuel.saturating_sub(c.fuel_used);
        if left == 0 {
            return self.fail(Error::OutOfFuel {
                budget: self.limits.fuel,
            });
        }
        let slice = self.limits.slice.clamp(1, left);
        let (st, used) = match self.guard(|vm| vm.run_metered(slice)) {
            Ok(r) => r,
            Err(e) => return self.fail(e),
        };
        let c = self.call.as_mut().expect("a call");
        c.fuel_used += used;
        let spent = c.fuel_used >= self.limits.fuel;
        match st {
            Status::OutOfFuel if spent => self.fail(Error::OutOfFuel {
                budget: self.limits.fuel,
            }),
            Status::OutOfFuel => Ok(Step::Yield),
            Status::HostCall { call_id, .. } => Ok(Step::HostCall(CallId(call_id))),
            Status::HostOutOfMemory => self.fail(Error::OutOfMemory),
            Status::Threw(e) => {
                let e = self.guard(|vm| exception(vm, e))?;
                self.fail(e)
            }
            Status::Done(_) | Status::Idle => {
                let mut c = self.call.take().expect("a call");
                let r = self.guard(|vm| settle(vm, &mut c, st == Status::Idle));
                self.call = Some(c);
                match r {
                    Ok(Ok(step)) => Ok(step),
                    Ok(Err(e)) | Err(e) => self.fail(e),
                }
            }
        }
    }

    /// A pending host call (sync or async) of the current call.
    pub fn host_call(&mut self, id: CallId) -> Result<HostCall<'_>, Error> {
        let info = self.guard(|vm| vm.host_call(id.0))?;
        let Some(info) = info else {
            return Err(Error::Usage(format!("no pending host call {}", id.0)));
        };
        Ok(HostCall {
            fn_id: info.fn_id,
            is_async: info.is_async,
            args: info.args,
            id,
            instance: self,
        })
    }

    /// Answers a pending host call: a sync call returns (or throws) at the call when the
    /// VM continues, an async one settles its promise.
    pub fn resolve(&mut self, id: CallId, reply: Result<Value<'_>, Throw>) -> Result<(), Error> {
        if self.call.as_ref().is_none_or(|c| c.over) {
            return Err(Error::Usage("no call in progress".into()));
        }
        let r = self.guard(|vm| -> Result<(), Error> {
            let result = match reply {
                Ok(v) => Ok(to_raw(vm, &v)?),
                Err(t) => Err(vm
                    .new_error(t.kind.core(), &t.message)
                    .map_err(|_| Error::OutOfMemory)?),
            };
            vm.resolve_host_call(id.0, result).map_err(|e| match e {
                VmError::Heap(_) => Error::OutOfMemory,
                _ => Error::Usage(format!("no pending host call {}", id.0)),
            })
        })?;
        match r {
            Ok(()) => Ok(()),
            // A stale id leaves the call as it is; anything else ends it.
            Err(e @ Error::Usage(_)) => Err(e),
            Err(e) => self.fail(e),
        }
    }

    /// The settled value of the last call, after [`Step::Done`].
    pub fn result(&self) -> Result<ValueRef<'_>, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        let h = self
            .call
            .as_ref()
            .and_then(|c| c.result)
            .ok_or_else(|| Error::Usage("no settled call".into()))?;
        Ok(ValueRef {
            heap: self.vm.heap(),
            raw: self.vm.heap().get_handle(h),
            max_bytes: self.limits.memory,
        })
    }

    /// Abandons the current call: the VM is idle, its pending host calls are dropped.
    pub fn abort(&mut self) {
        let _ = self.fail::<()>(Error::Usage("aborted".into()));
    }

    /// Calls the function at `path` to the end, answering every host call (sync or async)
    /// with `host` at once, and returns the settled value as plain data: for hosts whose
    /// host functions finish synchronously, and for tests. `host` must answer each call
    /// with [`HostCall::reply`].
    pub fn call<F>(&mut self, path: &str, args: &[Value<'_>], mut host: F) -> Result<Data, Error>
    where
        F: FnMut(HostCall<'_>) -> Result<(), Error>,
    {
        self.start(path, args)?;
        loop {
            match self.step()? {
                Step::Done => return self.result()?.to_data(),
                Step::Yield => {}
                Step::HostCall(id) => {
                    if let Err(e) = host(self.host_call(id)?) {
                        return self.fail(e);
                    }
                    if self.guard(|vm| vm.host_call(id.0).is_some())? {
                        return self
                            .fail(Error::Usage(format!("host call {} was not answered", id.0)));
                    }
                }
                Step::Wait(ids) => {
                    if ids.is_empty() {
                        return self
                            .fail(Error::Usage("async host calls were not answered".into()));
                    }
                    for id in ids {
                        if let Err(e) = host(self.host_call(id)?) {
                            return self.fail(e);
                        }
                    }
                }
            }
        }
    }
}

/// The function at `path` and the object it is a property of.
fn lookup(vm: &mut Vm, path: &str) -> Result<(Raw, Raw), Error> {
    let missing = || Error::Usage(format!("no global {path}"));
    match path.rsplit_once('.') {
        None => Ok((Raw::UNDEFINED, vm.global_path(path).ok_or_else(missing)?)),
        Some((base, _)) => {
            let this = vm.global_path(base).ok_or_else(missing)?;
            Ok((this, vm.global_path(path).ok_or_else(missing)?))
        }
    }
}

/// After the VM ran out of work (`idle`: async host calls are pending): the entry
/// returned; settles the call if its value or promise is settled.
fn settle(vm: &mut Vm, c: &mut Call, idle: bool) -> Result<Step, Error> {
    let heap_err = |_| Error::OutOfMemory;
    if c.entry.is_none() {
        let v = match vm.outcome() {
            Some(Ok(v)) => v,
            Some(Err(e)) => return Err(exception(vm, e)),
            None => return Err(Error::NeverSettles),
        };
        if vm.promise_state(v).is_some() {
            // Its rejection is the call's error, not an unhandled one.
            vm.mark_promise_handled(v);
        }
        c.entry = Some(vm.heap_mut().new_handle(v).map_err(heap_err)?);
    }
    let v = vm.heap().get_handle(c.entry.expect("the entry"));
    let r = match vm.promise_state(v) {
        None => v,
        Some(Some(Ok(r))) => r,
        Some(Some(Err(e))) => return Err(exception(vm, e)),
        Some(None) if idle => {
            return Ok(Step::Wait(
                vm.new_host_calls().into_iter().map(CallId).collect(),
            ));
        }
        Some(None) => return Err(Error::NeverSettles),
    };
    c.result = Some(vm.heap_mut().new_handle(r).map_err(heap_err)?);
    c.over = true;
    // Whatever the call left (async host calls nobody waits for, queued jobs, tracked
    // rejections) is dropped.
    vm.abort();
    Ok(Step::Done)
}

/// A pending host call: which function, its arguments, and the way to answer it.
pub struct HostCall<'a> {
    instance: &'a mut Instance,
    id: CallId,
    fn_id: u32,
    is_async: bool,
    args: Vec<Raw>,
}

impl HostCall<'_> {
    pub fn id(&self) -> CallId {
        self.id
    }

    /// The [`crate::HostFn`] id.
    pub fn fn_id(&self) -> u32 {
        self.fn_id
    }

    pub fn is_async(&self) -> bool {
        self.is_async
    }

    /// The number of arguments.
    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    /// Argument `i` (`undefined` past the end).
    pub fn arg(&self, i: usize) -> ValueRef<'_> {
        ValueRef {
            heap: self.instance.vm.heap(),
            raw: self.args.get(i).copied().unwrap_or(Raw::UNDEFINED),
            max_bytes: self.instance.limits.memory,
        }
    }

    /// Answers the call ([`Instance::resolve`]).
    pub fn reply(self, reply: Result<Value<'_>, Throw>) -> Result<(), Error> {
        self.instance.resolve(self.id, reply)
    }
}
