//! [`Snapshot`]: a VM image after the host's setup, compiled once and shared.

use std::sync::Arc;

use djs_core::compiler::CompileOptions;
use djs_core::heap::Heap;
use djs_core::value::Value as Raw;
use djs_core::vm::{ExecOptions, Status, Vm, VmOptions};

use crate::instance::{exception, guard_vm};
use crate::value::wire_error;
use crate::{Data, Error, Instance, Limits};

/// JavaScript source with its name (for stack traces and syntax errors).
#[derive(Clone, Debug)]
pub struct Source {
    name: String,
    text: String,
    kind: SourceKind,
}

/// How a [`Source`] is compiled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceKind {
    /// A classic script: its top-level declarations become globals
    Script,
}

impl Source {
    /// A classic script.
    pub fn script(name: impl Into<String>, text: impl Into<String>) -> Source {
        Source {
            name: name.into(),
            text: text.into(),
            kind: SourceKind::Script,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn kind(&self) -> SourceKind {
        self.kind
    }
}

/// A host function: a global the scripts call, which the host answers by `id`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFn {
    id: u32,
    name: String,
    is_async: bool,
}

impl HostFn {
    /// A synchronous host function: a call stops the VM (`Step::HostCall`) until the host
    /// answers; the script gets the value or the exception at the call. For bounded work
    /// (parsing, logging).
    pub fn sync(id: u32, name: impl Into<String>) -> HostFn {
        HostFn {
            id,
            name: name.into(),
            is_async: false,
        }
    }

    /// An async host function: a call returns a promise at once and the script runs on;
    /// the host starts the work when the VM waits (`Step::Wait`) and its answer settles
    /// the promise. For I/O.
    pub fn asynchronous(id: u32, name: impl Into<String>) -> HostFn {
        HostFn {
            id,
            name: name.into(),
            is_async: true,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_async(&self) -> bool {
        self.is_async
    }
}

/// Builds a [`Snapshot`]: host functions and data globals are defined first, then the
/// sources run in order, each to completion.
#[derive(Clone, Debug)]
pub struct Builder {
    host_fns: Vec<HostFn>,
    globals: Vec<(String, Data)>,
    sources: Vec<Source>,
    frozen_builtins: bool,
    memory: usize,
    fuel: u64,
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            host_fns: Vec::new(),
            globals: Vec::new(),
            sources: Vec::new(),
            frozen_builtins: true,
            memory: 64 << 20,
            fuel: 1_000_000_000,
        }
    }
}

impl Builder {
    /// Defines a host function as a global (`name`, not enumerable).
    pub fn host_fn(mut self, f: HostFn) -> Builder {
        self.host_fns.push(f);
        self
    }

    /// Defines a global holding plain data (configuration for the scripts).
    pub fn global(mut self, name: impl Into<String>, value: Data) -> Builder {
        self.globals.push((name.into(), value));
        self
    }

    /// Runs `source` at setup, after the sources added before it.
    pub fn source(mut self, source: Source) -> Builder {
        self.sources.push(source);
        self
    }

    /// Frozen builtins (default on): the builtin objects behave as frozen, so scripts
    /// cannot patch `Array.prototype` and the like; assigning an inherited builtin
    /// property defines an own property instead (as with Node's
    /// `--frozen-intrinsics`). The global object stays writable.
    pub fn frozen_builtins(mut self, on: bool) -> Builder {
        self.frozen_builtins = on;
        self
    }

    /// The heap cap during setup in `bytes` (default 64 MiB).
    pub fn memory(mut self, bytes: usize) -> Builder {
        self.memory = bytes;
        self
    }

    /// The fuel budget of all sources together (default 10^9, about 4 s).
    pub fn fuel(mut self, fuel: u64) -> Builder {
        self.fuel = fuel;
        self
    }

    /// Runs the setup and takes the snapshot. Setup sources must not call host functions:
    /// the snapshot has no host to answer them ([`Error::Usage`]).
    pub fn build(self) -> Result<Snapshot, Error> {
        let opts = vm_options(self.memory, self.frozen_builtins);
        let mut vm = guard_vm(|| djs_prelude::new_vm(opts))?.map_err(|_| Error::OutOfMemory)?;
        let image = guard_vm(|| self.setup(&mut vm))??;
        Ok(Snapshot(Arc::new(Inner {
            image,
            frozen_builtins: self.frozen_builtins,
            host_fns: self.host_fns,
        })))
    }

    fn setup(&self, vm: &mut Vm) -> Result<Vec<u8>, Error> {
        let oom = |_| Error::OutOfMemory;
        for f in &self.host_fns {
            let v = if f.is_async {
                vm.new_async_host_function(f.id, &f.name)
            } else {
                vm.new_host_function(f.id, &f.name)
            }
            .map_err(oom)?;
            vm.define_global(&f.name, v).map_err(oom)?;
        }
        for (name, data) in &self.globals {
            let v = vm.decode_wire(&data.to_wire()).map_err(wire_error)?;
            vm.define_global(name, v).map_err(oom)?;
        }
        let mut fuel = self.fuel;
        for s in &self.sources {
            let f = vm
                .compile(s.text.as_bytes(), &CompileOptions::script(&s.name))
                .map_err(|e| Error::Syntax {
                    source: s.name.clone(),
                    message: e.to_string(),
                })?;
            let this = Raw::object(vm.global());
            vm.call(f, this, &[])
                .map_err(|e| Error::Usage(format!("{e:?}")))?;
            loop {
                let (st, used) = vm.run_metered(fuel);
                fuel = fuel.saturating_sub(used);
                match st {
                    Status::Done(_) => break,
                    Status::Threw(e) => return Err(exception(vm, e)),
                    Status::OutOfFuel if fuel == 0 => {
                        vm.abort();
                        return Err(Error::OutOfFuel { budget: self.fuel });
                    }
                    Status::OutOfFuel => {}
                    Status::HostOutOfMemory => return Err(Error::OutOfMemory),
                    Status::HostCall { fn_id, .. } => {
                        vm.abort();
                        return Err(Error::Usage(format!(
                            "{} called host function {fn_id} during setup",
                            s.name
                        )));
                    }
                    Status::Idle => {
                        vm.abort();
                        return Err(Error::Usage(format!(
                            "{} left async host calls pending during setup",
                            s.name
                        )));
                    }
                }
            }
        }
        // Everything the setup left is old from now on: collections during calls leave it
        // alone.
        vm.seal().map_err(oom)?;
        Ok(vm.image().to_vec())
    }
}

/// The options of every VM of a snapshot: reaching the heap cap is never catchable.
pub(crate) fn vm_options(memory: usize, frozen_builtins: bool) -> VmOptions {
    VmOptions {
        memory_limit: memory,
        frozen_builtins,
        exec: ExecOptions {
            fatal_out_of_memory: true,
            ..ExecOptions::default()
        },
        ..VmOptions::default()
    }
}

/// A compiled setup: the image of a VM after the host's host functions, globals and setup
/// sources, from which [`Instance`]s start. Cheap to clone (shared), `Send` and `Sync`.
/// Images live in this process only: there is no way to save one, and none is needed,
/// since building one from the sources is the same thing.
#[derive(Clone)]
pub struct Snapshot(Arc<Inner>);

struct Inner {
    image: Vec<u8>,
    frozen_builtins: bool,
    host_fns: Vec<HostFn>,
}

impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Snapshot")
            .field("image_len", &self.0.image.len())
            .field("host_fns", &self.0.host_fns)
            .finish()
    }
}

const _: () = {
    const fn shareable<T: Send + Sync>() {}
    shareable::<Snapshot>();
};

impl Snapshot {
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// A new VM in the snapshot's state. Copies the image (no compile, no setup run).
    pub fn instantiate(&self, limits: Limits) -> Result<Instance, Error> {
        Instance::new(self.clone(), limits)
    }

    /// The image's size in `bytes`: what each instance copies.
    pub fn image_len(&self) -> usize {
        self.0.image.len()
    }

    /// The host functions, as declared.
    pub fn host_fns(&self) -> &[HostFn] {
        &self.0.host_fns
    }

    pub(crate) fn image(&self) -> &[u8] {
        &self.0.image
    }

    pub(crate) fn frozen_builtins(&self) -> bool {
        self.0.frozen_builtins
    }

    /// A buffer of the image with room for the heap to start growing in.
    pub(crate) fn image_buffer(&self, memory: usize) -> Vec<u8> {
        let image = self.image();
        let mut v = Vec::with_capacity(Heap::image_capacity(image.len(), memory));
        v.extend_from_slice(image);
        v
    }
}
