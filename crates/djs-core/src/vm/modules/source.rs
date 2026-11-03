//! Host loading and resumable evaluation of acyclic source-module graphs.
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::{CELL, EXPORTS, LOCALS, ModuleError, ModuleGraph, get, put, record};
use crate::compiler::CompileError;
use crate::heap::HeapError;
use crate::value::{Obj, Value};
use crate::vm::{RunState, Status, Vm, VmError};

/// Host policy for resolving canonical module names and loading UTF-8 JavaScript.
///
/// Implementations perform any path, package or URL normalization. The engine caches
/// by the exact canonical identity returned from `resolve`. Synthetic modules already
/// registered in the graph are used without calling `load`.
pub trait ModuleLoader {
    /// Resolves a static specifier relative to its canonical referrer.
    /// # Errors
    /// Return a loader error when the specifier cannot be resolved.
    fn resolve(&mut self, referrer: &str, specifier: &str) -> Result<String, SourceError>;
    /// Loads source text for an already canonical identity.
    /// # Errors
    /// Return a loader error when the module cannot be read.
    fn load(&mut self, canonical: &str) -> Result<String, SourceError>;
}

/// A source-module loading, linking or evaluation API error.
#[derive(Debug)]
pub enum SourceError {
    /// A host resolution or loading failure.
    Loader(String),
    /// A source syntax error or compiler allocation failure.
    Compile(CompileError),
    /// A missing/duplicate binding or an allocation failure in the binding graph.
    Binding(ModuleError),
    /// A dependency cycle. Cyclic declaration instantiation is not implemented yet.
    Cycle(String),
    /// A previous load/link failure for this canonical identity.
    Cached(String),
    /// A module or evaluation depth limit was reached.
    DepthLimit,
    /// Another evaluation or VM call is still active.
    Busy,
    /// Polling without an evaluation started by `begin_evaluation`.
    NoEvaluation,
    /// A prior host allocator failure abandoned a partially executed module.
    Aborted,
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Loader(s) | Self::Cached(s) => f.write_str(s),
            Self::Compile(e) => e.fmt(f),
            Self::Binding(e) => e.fmt(f),
            Self::Cycle(id) => write!(f, "cyclic source modules are not supported yet: {id}"),
            Self::DepthLimit => f.write_str("module dependency depth limit exceeded"),
            Self::Busy => f.write_str("module evaluation or VM call is active"),
            Self::NoEvaluation => f.write_str("no module evaluation started"),
            Self::Aborted => {
                f.write_str("module evaluation was abandoned after host allocation failure")
            }
        }
    }
}
impl core::error::Error for SourceError {}
impl From<ModuleError> for SourceError {
    fn from(e: ModuleError) -> Self {
        Self::Binding(e)
    }
}
impl From<HeapError> for SourceError {
    fn from(e: HeapError) -> Self {
        Self::Binding(ModuleError::Heap(e))
    }
}
impl From<CompileError> for SourceError {
    fn from(e: CompileError) -> Self {
        Self::Compile(e)
    }
}

// These record keys and state codes are part of the image dependency descriptor.
const STATE: &str = "state";
const FUNCTION: &str = "function";
const DEPS: &str = "dependencies";
const ERROR: &str = "error";
const QUEUE: &str = "queue";
const CURSOR: &str = "cursor";
const ENTRY: &str = "entry";
const ACTIVE: &str = "active";
const LOADING: i32 = -1;
const LOAD_FAILED: i32 = -2;
const LINKED: i32 = 0;
const EVALUATING: i32 = 1;
const EVALUATED: i32 = 2;
const FAILED: i32 = 3;
const ABORTED: i32 = 4;
const MAX_DEPTH: usize = 256;

fn state(vm: &mut Vm, module: Obj) -> Result<Option<i32>, HeapError> {
    Ok(get(&mut vm.heap, module, STATE)?.and_then(Value::as_int))
}

fn string(vm: &mut Vm, value: Value) -> String {
    vm.heap
        .str_to_string(value.as_string().expect("module identity string"))
}

impl ModuleGraph {
    /// Loads, compiles and links an acyclic source graph without executing any body.
    ///
    /// `id` is already canonical. Every dependency is resolved by the host. Previously
    /// loaded modules are reused. Load/link failures are cached and never execute code.
    /// Native modules registered with `declare` can satisfy static imports.
    /// # Errors
    /// Returns loader, syntax, missing-export, unsupported-cycle or allocation errors.
    pub fn load_source(
        &self,
        vm: &mut Vm,
        id: &str,
        loader: &mut impl ModuleLoader,
    ) -> Result<(), SourceError> {
        let root = self.graph_root(vm);
        if get(&mut vm.heap, root, ACTIVE)? == Some(Value::TRUE) {
            return Err(SourceError::Busy);
        }
        self.load_at(vm, id, loader, 0)
    }

    fn load_at(
        &self,
        vm: &mut Vm,
        id: &str,
        loader: &mut impl ModuleLoader,
        depth: usize,
    ) -> Result<(), SourceError> {
        if depth >= MAX_DEPTH {
            return Err(SourceError::DepthLimit);
        }
        if self.contains(vm, id)? {
            let module = self.module(vm, id)?;
            return match state(vm, module)? {
                Some(LOADING) => Err(SourceError::Cycle(String::from(id))),
                Some(LOAD_FAILED) => {
                    let message = get(&mut vm.heap, module, ERROR)?.and_then(Value::as_string);
                    Err(SourceError::Cached(message.map_or_else(
                        || String::from("previous module load failed"),
                        |s| vm.heap.str_to_string(s),
                    )))
                }
                _ => Ok(()),
            };
        }
        // Publish only after the loading marker and error slot exist. A failed
        // allocation must never leave a source record looking like a native module.
        let root = self.root(vm)?;
        let module = vm.heap.new_object(None)?;
        let locals = vm.heap.new_object(None)?;
        let exports = vm.heap.new_object(None)?;
        put(&mut vm.heap, module, LOCALS, Value::object(locals))?;
        put(&mut vm.heap, module, EXPORTS, Value::object(exports))?;
        put(&mut vm.heap, module, STATE, Value::int(LOADING))?;
        put(&mut vm.heap, module, ERROR, Value::UNDEFINED)?;
        put(&mut vm.heap, root, id, Value::object(module))?;
        let result = self.load_body(vm, id, loader, depth);
        if let Err(error) = &result {
            let module = self.module(vm, id)?;
            put(&mut vm.heap, module, STATE, Value::int(LOAD_FAILED))?;
            let text = alloc::format!("{error}");
            let value = Value::string(vm.heap.alloc_str(&text)?);
            put(&mut vm.heap, module, ERROR, value)?;
        }
        result
    }

    fn load_body(
        &self,
        vm: &mut Vm,
        id: &str,
        loader: &mut impl ModuleLoader,
        depth: usize,
    ) -> Result<(), SourceError> {
        let source = loader.load(id)?;
        let locals = self.table(vm, id, LOCALS)?;
        let (template, info) =
            crate::compiler::source_module::compile(&mut vm.heap, source.as_bytes(), id, locals)?;
        let function = vm.instantiate(template)?;
        let module = self.module(vm, id)?;
        put(&mut vm.heap, module, FUNCTION, function)?;
        let mut requests = Vec::new();
        for request in &info.requests {
            let canonical = loader.resolve(id, request)?;
            self.load_at(vm, &canonical, loader, depth + 1)?;
            requests.push((request, canonical));
        }
        for import in &info.imports {
            let dependency = &requests
                .iter()
                .find(|(request, _)| **request == import.request)
                .expect("resolved import request")
                .1;
            let target = self.export_cell(vm, dependency, &import.imported)?;
            let local_name = vm.heap.atom_name(import.local);
            let binding = self.binding(vm, id, &local_name)?;
            put(&mut vm.heap, binding, CELL, Value::object(target))?;
        }
        for (exported, local) in &info.exports {
            let local = vm.heap.atom_name(*local);
            self.export(vm, id, exported, &local)?;
        }
        let mut dependencies = Vec::new();
        for (_, dependency) in requests {
            dependencies.push(Value::string(vm.heap.alloc_str(&dependency)?));
        }
        let deps = vm.heap.new_array_from_values(None, &dependencies)?;
        let module = self.module(vm, id)?;
        put(&mut vm.heap, module, DEPS, Value::object(deps))?;
        put(&mut vm.heap, module, STATE, Value::int(LINKED))?;
        Ok(())
    }

    /// Starts dependency-order evaluation, reusing successfully evaluated modules.
    ///
    /// Drive `poll_evaluation` and the usual host-call APIs until Done or Threw.
    /// Module evaluation state, the queue and the current VM frame survive snapshots.
    /// The host must not start unrelated VM calls while this evaluation is active.
    /// # Errors
    /// Returns missing/load-failed module, busy-VM, depth-limit or allocation errors.
    pub fn begin_evaluation(&self, vm: &mut Vm, id: &str) -> Result<(), SourceError> {
        let root = self.graph_root(vm);
        if get(&mut vm.heap, root, ACTIVE)? == Some(Value::TRUE)
            || matches!(
                vm.state(),
                RunState::Running | RunState::HostCall | RunState::ThrowPending
            )
            || !vm.pending_host_calls().is_empty()
        {
            return Err(SourceError::Busy);
        }
        let mut names = Vec::new();
        self.evaluation_order(vm, id, &mut names, 0)?;
        let mut values = Vec::new();
        for name in names {
            values.push(Value::string(vm.heap.alloc_str(&name)?));
        }
        let queue = vm.heap.new_array_from_values(None, &values)?;
        let entry = Value::string(vm.heap.alloc_str(id)?);
        put(&mut vm.heap, root, QUEUE, Value::object(queue))?;
        put(&mut vm.heap, root, CURSOR, Value::int(0))?;
        put(&mut vm.heap, root, ENTRY, entry)?;
        put(&mut vm.heap, root, ACTIVE, Value::TRUE)?;
        Ok(())
    }

    fn evaluation_order(
        &self,
        vm: &mut Vm,
        id: &str,
        names: &mut Vec<String>,
        depth: usize,
    ) -> Result<(), SourceError> {
        if depth >= MAX_DEPTH {
            return Err(SourceError::DepthLimit);
        }
        if names.iter().any(|name| name == id) {
            return Ok(());
        }
        let module = self.module(vm, id)?;
        match state(vm, module)? {
            None | Some(EVALUATED) => return Ok(()),
            Some(LOADING | LOAD_FAILED) => {
                return Err(SourceError::Cached(String::from(
                    "module has not linked successfully",
                )));
            }
            Some(ABORTED) => return Err(SourceError::Aborted),
            Some(FAILED) => {
                names.push(String::from(id));
                return Ok(());
            }
            _ => {}
        }
        let deps = record(&mut vm.heap, module, DEPS)?;
        for index in 0..vm.heap.array_length(deps) {
            let dependency = vm
                .heap
                .get_element_fast(deps, index)
                .expect("module dependency identity");
            let dependency = string(vm, dependency);
            self.evaluation_order(vm, &dependency, names, depth + 1)?;
        }
        names.push(String::from(id));
        Ok(())
    }

    /// Runs at most one module body with a fuel budget, retaining all continuation state.
    ///
    /// `OutOfFuel` also yields between completed dependency bodies. `Done(undefined)`
    /// means the graph completed; `Threw` preserves and caches the thrown value. Handle
    /// `HostCall`/`Idle` with the existing VM host-call API, then poll again. No source
    /// module is evaluated twice, including after GC or snapshot restoration.
    /// # Errors
    /// Returns no-evaluation, VM-busy or allocation errors. Source exceptions use Status.
    pub fn poll_evaluation(&self, vm: &mut Vm, fuel: u64) -> Result<Status, SourceError> {
        let root = self.graph_root(vm);
        if get(&mut vm.heap, root, ACTIVE)? != Some(Value::TRUE) {
            return Err(SourceError::NoEvaluation);
        }
        let queue = record(&mut vm.heap, root, QUEUE)?;
        let cursor = get(&mut vm.heap, root, CURSOR)?
            .and_then(Value::as_int)
            .expect("module evaluation cursor") as u32;
        if cursor >= vm.heap.array_length(queue) {
            put(&mut vm.heap, root, ACTIVE, Value::FALSE)?;
            return Ok(Status::Done(Value::UNDEFINED));
        }
        let id_value = vm
            .heap
            .get_element_fast(queue, cursor)
            .expect("evaluation identity");
        let id = string(vm, id_value);
        let module = self.module(vm, &id)?;
        if state(vm, module)? == Some(FAILED) {
            let error = get(&mut vm.heap, module, ERROR)?.expect("cached module exception");
            return self.fail_evaluation(vm, &id, error);
        }
        if state(vm, module)? == Some(LINKED) {
            let function = get(&mut vm.heap, module, FUNCTION)?.expect("linked module body");
            put(&mut vm.heap, module, STATE, Value::int(EVALUATING))?;
            if let Err(error) = vm.call(function, Value::UNDEFINED, &[]) {
                put(&mut vm.heap, module, STATE, Value::int(LINKED))?;
                return Err(match error {
                    VmError::Heap(error) => {
                        vm.abort();
                        SourceError::from(error)
                    }
                    _ => SourceError::Busy,
                });
            }
        }
        let status = vm.run(fuel);
        match status {
            Status::Done(_) => {
                // run may compact the arena: reacquire every record after it.
                let module = self.module(vm, &id)?;
                put(&mut vm.heap, module, STATE, Value::int(EVALUATED))?;
                let root = self.graph_root(vm);
                put(&mut vm.heap, root, CURSOR, Value::int(cursor as i32 + 1))?;
                Ok(Status::OutOfFuel)
            }
            Status::Threw(value) => self.fail_evaluation(vm, &id, value),
            Status::HostOutOfMemory => {
                // Vm::run abandoned its frames. Never restart a partially executed
                // body, and do not leave an active scheduler waiting on those frames.
                let module = self.module(vm, &id)?;
                put(&mut vm.heap, module, STATE, Value::int(ABORTED))?;
                let root = self.graph_root(vm);
                let entry = get(&mut vm.heap, root, ENTRY)?.expect("evaluation entry identity");
                let entry = string(vm, entry);
                let module = self.module(vm, &entry)?;
                put(&mut vm.heap, module, STATE, Value::int(ABORTED))?;
                put(&mut vm.heap, root, ACTIVE, Value::FALSE)?;
                Ok(status)
            }
            _ => Ok(status),
        }
    }

    fn fail_evaluation(&self, vm: &mut Vm, id: &str, error: Value) -> Result<Status, SourceError> {
        let module = self.module(vm, id)?;
        put(&mut vm.heap, module, ERROR, error)?;
        put(&mut vm.heap, module, STATE, Value::int(FAILED))?;
        let root = self.graph_root(vm);
        let entry = get(&mut vm.heap, root, ENTRY)?.expect("evaluation entry identity");
        let entry = string(vm, entry);
        let module = self.module(vm, &entry)?;
        put(&mut vm.heap, module, ERROR, error)?;
        put(&mut vm.heap, module, STATE, Value::int(FAILED))?;
        put(&mut vm.heap, root, ACTIVE, Value::FALSE)?;
        Ok(Status::Threw(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{ExecOptions, VmOptions};

    struct Loader;
    impl ModuleLoader for Loader {
        fn resolve(&mut self, _: &str, specifier: &str) -> Result<String, SourceError> {
            Ok(String::from(specifier))
        }
        fn load(&mut self, _: &str) -> Result<String, SourceError> {
            Ok(String::from(
                "export let x = 1; for (let i = 0; i < 100; i++) x++;",
            ))
        }
    }

    #[test]
    fn host_allocator_failure_terminates_the_scheduler_without_reexecuting() {
        let mut vm = Vm::new(VmOptions {
            prelude: false,
            exec: ExecOptions {
                gc_stress: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
        let graph = ModuleGraph::new(&mut vm).unwrap();
        graph.load_source(&mut vm, "main.js", &mut Loader).unwrap();
        graph.begin_evaluation(&mut vm, "main.js").unwrap();
        vm.heap_mut().fail_next_gc_reserve = true;
        assert_eq!(
            graph.poll_evaluation(&mut vm, 10000).unwrap(),
            Status::HostOutOfMemory
        );
        assert!(matches!(
            graph.poll_evaluation(&mut vm, 10000),
            Err(SourceError::NoEvaluation)
        ));
        assert!(matches!(
            graph.begin_evaluation(&mut vm, "main.js"),
            Err(SourceError::Aborted)
        ));
        vm.heap().verify().unwrap();
    }
}
