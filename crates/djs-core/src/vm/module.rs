//! ES modules for embedders (D3171): a VM loads a module graph through the host's
//! [`ModuleLoader`], synchronously and with no I/O of its own, links it, evaluates it in
//! a run like any call, and hands out its exports. The records live in the realm
//! (interp/module.rs), so an image of the VM carries the loaded, linked and evaluated
//! graph: a VM made from it reads the same exports, and their bindings stay live.
//!
//! ```ignore
//! let key = vm.load_module("main.js", &mut loader)?; // loads and links the graph
//! vm.evaluate_module(&key)?;                          // then run until Done
//! let on_request = vm.module_export(&key, "onRequest");
//! ```
//!
//! Without the `modules` feature every call fails: loading with a SyntaxError "modules
//! are not available in this build".

use alloc::string::String;
use alloc::vec::Vec;

use super::{Vm, VmError};
use crate::compiler::{CompileOptions, compile_module};
use crate::interp::conv::{Fault, FaultKind};
use crate::interp::module::{self, status};
use crate::interp::{ErrorKind, realm};
use crate::value::{Obj, Value};

/// A module's source as the host supplies it.
#[derive(Clone, Debug)]
pub struct ModuleSource {
    pub text: String,
    /// Compile it as TypeScript (`CompileOptions::typescript`).
    pub typescript: bool,
}

/// The host side of module loading. Specifiers and keys are opaque strings: the engine
/// gives them no URL or path meaning. A key names one module: the VM loads each key once
/// and keeps its record.
pub trait ModuleLoader {
    /// The key of the module that `specifier` names when module `referrer` (its key)
    /// imports it; `None` for the graph's entry point. An error becomes an `Error` with
    /// the message.
    fn resolve(&mut self, specifier: &str, referrer: Option<&str>) -> Result<String, String>;
    /// The source of module `key`. An error becomes an `Error` with the message.
    fn load(&mut self, key: &str) -> Result<ModuleSource, String>;
}

impl Vm {
    /// Loads the module `specifier` names and every module it imports, directly or not,
    /// through `loader`, compiles and links them: its key. A module already loaded (by
    /// key) is not loaded again. Fails with the JS error: a SyntaxError of a source, a
    /// link error (a missing or ambiguous import, as a SyntaxError), or the loader's
    /// message (an `Error`); the modules this call loaded are forgotten then, so a later
    /// call loads them again. Runs no JS: allowed whenever the VM is not running.
    pub fn load_module(
        &mut self,
        specifier: &str,
        loader: &mut dyn ModuleLoader,
    ) -> Result<String, Value> {
        if !crate::MODULES {
            return Err(self.error_value(
                ErrorKind::SyntaxError,
                "modules are not available in this build",
            ));
        }
        let mut created: Vec<String> = Vec::new();
        let r = self.load_graph(specifier, loader, &mut created);
        match r {
            Ok(key) => Ok(key),
            Err(f) => {
                for k in &created {
                    let _ = module::remove(&mut self.heap, k);
                }
                Err(self.fault_to_value(f))
            }
        }
    }

    fn load_graph(
        &mut self,
        specifier: &str,
        loader: &mut dyn ModuleLoader,
        created: &mut Vec<String>,
    ) -> Result<String, Fault> {
        let key = loader.resolve(specifier, None).map_err(host_error)?;
        let mut queue = Vec::new();
        let root = self.module_record(&key, loader, created, &mut queue)?;
        // The new records. requests (each key is loaded once).
        while let Some(m) = queue.pop() {
            let meta = module::meta(&self.heap, m);
            let referrer = module::key_of(&self.heap, m);
            let mut records = Vec::new();
            for &spec in &meta.requests {
                let s = self.heap.atom_name(spec);
                let k = loader.resolve(&s, Some(&referrer)).map_err(host_error)?;
                records.push(self.module_record(&k, loader, created, &mut queue)?);
            }
            module::set_requested(&mut self.heap, m, &records)?;
        }
        module::link(&mut self.heap, root)?;
        Ok(key)
    }

    /// The record of `key`: loaded and compiled now if new (and queued for its requests).
    fn module_record(
        &mut self,
        key: &str,
        loader: &mut dyn ModuleLoader,
        created: &mut Vec<String>,
        queue: &mut Vec<Obj>,
    ) -> Result<Obj, Fault> {
        if let Some(m) = module::find(&mut self.heap, key)? {
            return Ok(m);
        }
        let src = loader.load(key).map_err(host_error)?;
        let mut opts = CompileOptions::module(key);
        opts.typescript = src.typescript;
        let (t, meta) =
            compile_module(&mut self.heap, src.text.as_bytes(), &opts).map_err(|e| {
                if e.kind == crate::compiler::CompileErrorKind::Syntax {
                    Fault::error(
                        ErrorKind::SyntaxError,
                        crate::text::cat!(&e.message, " at ", key, ":", e.line, ":", e.col),
                    )
                } else {
                    Fault::heap(crate::heap::HeapError::OutOfMemory)
                }
            })?;
        let m = module::new_record(&mut self.heap, key, t, meta)?;
        created.push(String::from(key));
        queue.push(m);
        Ok(m)
    }

    /// Starts the evaluation of the loaded module `key` (Evaluate(), 16.2.1.5.3): `run`
    /// executes it like a [`Vm::call`], and it ends `Done` with a promise, fulfilled once
    /// the graph has run, or rejected with the error a module threw. A module evaluates
    /// once: a later call settles the same way without running anything (an error is
    /// kept). `Ok(false)` if no module `key` is loaded.
    pub fn evaluate_module(&mut self, key: &str) -> Result<bool, VmError> {
        let Some(m) = module::find(&mut self.heap, key)? else {
            return Ok(false);
        };
        if module::status_of(&self.heap, m) < status::LINKED {
            return Ok(false);
        }
        use crate::bytecode::intrinsics::intrinsic_id_of as id;
        // `%ModuleOp` with `this` 5: Evaluate (builtins/module.ts).
        let f = realm::installed_intrinsic(&self.heap, id!("ModuleOp"));
        self.call(f, Value::int(5), &[Value::object(m)])?;
        Ok(true)
    }

    /// Whether a module `key` is loaded (and linked).
    pub fn has_module(&mut self, key: &str) -> bool {
        matches!(module::find(&mut self.heap, key), Ok(Some(m)) if module::status_of(&self.heap, m) >= status::LINKED)
    }

    /// The namespace object of the loaded module `key` (made on first use), or `None`.
    pub fn module_namespace(&mut self, key: &str) -> Result<Option<Value>, VmError> {
        let Some(m) = self.linked_module(key)? else {
            return Ok(None);
        };
        match module::namespace(&mut self.heap, m) {
            Ok(ns) => Ok(Some(Value::object(ns))),
            Err(f) => Err(fault_vm_error(f)),
        }
    }

    /// The current value of export `name` of the loaded module `key`: `None` if there is
    /// no such module or export (or it is ambiguous), or the binding is not initialized
    /// yet (its module has not run that far).
    pub fn module_export(&mut self, key: &str, name: &str) -> Result<Option<Value>, VmError> {
        let Some(m) = self.linked_module(key)? else {
            return Ok(None);
        };
        module::export_value(&mut self.heap, m, name).map_err(fault_vm_error)
    }

    fn linked_module(&mut self, key: &str) -> Result<Option<Obj>, VmError> {
        let m = module::find(&mut self.heap, key)?;
        Ok(m.filter(|&m| module::status_of(&self.heap, m) >= status::LINKED))
    }

    fn error_value(&mut self, kind: ErrorKind, msg: &str) -> Value {
        realm::new_error(&mut self.heap, kind, msg).unwrap_or(Value::UNDEFINED)
    }

    fn fault_to_value(&mut self, f: Fault) -> Value {
        match *f.0 {
            FaultKind::Throw(v) => v,
            FaultKind::Error(kind, msg) => self.error_value(kind, &msg),
            FaultKind::Heap(e) => self.error_value(ErrorKind::InternalError, e.as_str()),
        }
    }
}

fn host_error(msg: String) -> Fault {
    Fault::error(ErrorKind::Error, msg)
}

fn fault_vm_error(f: Fault) -> VmError {
    match *f.0 {
        FaultKind::Heap(e) => VmError::Heap(e),
        _ => VmError::Heap(crate::heap::HeapError::OutOfMemory),
    }
}
