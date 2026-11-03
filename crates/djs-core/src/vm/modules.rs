//! Arena-backed source modules and native host bindings.
//!
//! [`ModuleGraph::load_source`] parses, compiles and links acyclic source graphs
//! through a [`ModuleLoader`]. Drive execution with `begin_evaluation` and
//! `poll_evaluation`. Named/default imports, explicit exports/reexports and nested
//! closures share live binding cells. Source cycles, namespace objects, dynamic
//! loading and top-level await are not implemented yet. Hosts can also declare and
//! initialize synthetic modules through the binding API.
//!
//! The graph owns one host handle. Call [`ModuleGraph::release`] when finished. Keep
//! its handle index alongside a snapshot and use [`ModuleGraph::from_handle_index`]
//! after restoring that snapshot. Like all host handles, it belongs to one VM lineage.

mod source;
pub use source::{ModuleLoader, SourceError};

use core::fmt;

use super::Vm;
use crate::heap::{Handle, Heap, HeapError};
use crate::value::{Obj, Value};

/// Whether the defining module can assign a binding after initialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    /// `let`, `var`, function declarations and mutable synthetic exports.
    Mutable,
    /// `const` and immutable synthetic exports.
    Constant,
}

/// A linking or binding access failure. These are host errors, not JS exceptions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleError {
    /// The canonical identity has already been registered.
    DuplicateModule,
    /// No module has been registered under this canonical identity.
    MissingModule,
    /// A local or export name has already been declared in this module.
    DuplicateBinding,
    /// A requested local or exported name does not exist.
    MissingBinding,
    /// The original binding has not been initialized yet.
    Uninitialized,
    /// Initialization was attempted twice.
    AlreadyInitialized,
    /// Assignment or initialization through an import, or assignment to a constant.
    ReadOnly,
    /// Internal TDZ markers cannot be supplied as binding values.
    InvalidValue,
    /// Allocation failed; no successful graph mutation has been committed.
    Heap(HeapError),
}

impl From<HeapError> for ModuleError {
    fn from(value: HeapError) -> Self {
        Self::Heap(value)
    }
}

impl fmt::Display for ModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DuplicateModule => "module already registered",
            Self::MissingModule => "module not registered",
            Self::DuplicateBinding => "binding already declared",
            Self::MissingBinding => "binding not declared",
            Self::Uninitialized => "binding not initialized",
            Self::AlreadyInitialized => "binding already initialized",
            Self::ReadOnly => "binding is read-only",
            Self::InvalidValue => "invalid binding value",
            Self::Heap(e) => e.as_str(),
        })
    }
}

impl core::error::Error for ModuleError {}

/// A host-owned module graph whose binding state lives entirely in the arena.
///
/// Internal objects are never exposed as JavaScript values. Returned export values
/// are raw values: root them in a handle before running or collecting the VM.
#[derive(Debug)]
pub struct ModuleGraph {
    root: Handle,
}

// All records are ordinary null-prototype objects with ordinary data properties.
// This deliberately uses the existing image format and GC tracing paths.
const MODULES: &str = "modules";
const LOCALS: &str = "locals";
const EXPORTS: &str = "exports";
const CELL: &str = "cell";
const KIND: &str = "kind";
const VALUE: &str = "value";
const MUTABLE: i32 = 0;
const CONSTANT: i32 = 1;
const IMPORT: i32 = 2;

fn get(heap: &mut Heap, object: Obj, name: &str) -> Result<Option<Value>, HeapError> {
    let key = heap.atom_str(name)?;
    Ok(heap.get_own_property(object, key).and_then(|p| p.value()))
}

fn put(heap: &mut Heap, object: Obj, name: &str, value: Value) -> Result<(), HeapError> {
    let key = heap.atom_str(name)?;
    // Every internal property remains writable/configurable; no JS receives the records.
    heap.create_data_property(object, key, value)?;
    Ok(())
}

fn record(heap: &mut Heap, object: Obj, name: &str) -> Result<Obj, ModuleError> {
    Ok(get(heap, object, name)?
        .and_then(Value::as_object)
        .expect("module graph record must contain an object"))
}

impl ModuleGraph {
    /// Creates an empty graph and roots it in `vm`.
    ///
    /// # Errors
    /// Returns an allocation error if the root object or handle cannot be allocated.
    pub fn new(vm: &mut Vm) -> Result<Self, HeapError> {
        let object = vm.heap.new_object(None)?;
        let modules = vm.heap.new_object(None)?;
        put(&mut vm.heap, object, MODULES, Value::object(modules))?;
        Ok(Self {
            root: vm.heap.new_handle(Value::object(object))?,
        })
    }

    /// The live host handle index to store alongside a snapshot of this VM.
    pub fn handle_index(&self) -> u32 {
        self.root.index()
    }

    /// Reattaches to this graph's live handle in a restored VM.
    ///
    /// The caller must supply the index from this graph and the corresponding image.
    /// Do not release two wrappers for the same handle, or reuse a released index.
    /// A wrong index has the same panic behavior as [`Heap::get_handle`].
    pub fn from_handle_index(index: u32) -> Self {
        Self {
            root: Handle::from_index(index),
        }
    }

    /// Releases the root; exported values held by other handles remain alive.
    pub fn release(self, vm: &mut Vm) {
        vm.heap.drop_handle(self.root);
    }

    fn graph_root(&self, vm: &Vm) -> Obj {
        vm.heap
            .get_handle(self.root)
            .as_object()
            .expect("module graph handle must contain an object")
    }

    fn root(&self, vm: &mut Vm) -> Result<Obj, HeapError> {
        let graph = self.graph_root(vm);
        Ok(get(&mut vm.heap, graph, MODULES)?
            .and_then(Value::as_object)
            .expect("module registry table"))
    }

    fn module(&self, vm: &mut Vm, id: &str) -> Result<Obj, ModuleError> {
        let root = self.root(vm)?;
        get(&mut vm.heap, root, id)?
            .and_then(Value::as_object)
            .ok_or(ModuleError::MissingModule)
    }

    /// Checks whether the host's canonical identity has already been registered.
    ///
    /// # Errors
    /// Returns an allocation error if interning the identity fails.
    pub fn contains(&self, vm: &mut Vm, id: &str) -> Result<bool, HeapError> {
        let root = self.root(vm)?;
        Ok(get(&mut vm.heap, root, id)?.is_some())
    }

    /// Registers an empty module under a canonical, host-resolved identity.
    ///
    /// No filesystem or URL normalization occurs. Registration does not execute code.
    /// # Errors
    /// Returns [`ModuleError::DuplicateModule`] on an existing identity, or allocation errors.
    pub fn declare(&self, vm: &mut Vm, id: &str) -> Result<(), ModuleError> {
        let root = self.root(vm)?;
        if get(&mut vm.heap, root, id)?.is_some() {
            return Err(ModuleError::DuplicateModule);
        }
        let module = vm.heap.new_object(None)?;
        let locals = vm.heap.new_object(None)?;
        let exports = vm.heap.new_object(None)?;
        put(&mut vm.heap, module, LOCALS, Value::object(locals))?;
        put(&mut vm.heap, module, EXPORTS, Value::object(exports))?;
        put(&mut vm.heap, root, id, Value::object(module))?;
        Ok(())
    }

    fn table(&self, vm: &mut Vm, id: &str, name: &str) -> Result<Obj, ModuleError> {
        let module = self.module(vm, id)?;
        record(&mut vm.heap, module, name)
    }

    fn binding(&self, vm: &mut Vm, id: &str, name: &str) -> Result<Obj, ModuleError> {
        let table = self.table(vm, id, LOCALS)?;
        get(&mut vm.heap, table, name)?
            .and_then(Value::as_object)
            .ok_or(ModuleError::MissingBinding)
    }

    fn export_cell(&self, vm: &mut Vm, id: &str, name: &str) -> Result<Obj, ModuleError> {
        let table = self.table(vm, id, EXPORTS)?;
        get(&mut vm.heap, table, name)?
            .and_then(Value::as_object)
            .ok_or(ModuleError::MissingBinding)
    }

    fn add_binding(
        &self,
        vm: &mut Vm,
        id: &str,
        name: &str,
        cell: Obj,
        kind: i32,
    ) -> Result<(), ModuleError> {
        let table = self.table(vm, id, LOCALS)?;
        if get(&mut vm.heap, table, name)?.is_some() {
            return Err(ModuleError::DuplicateBinding);
        }
        let binding = vm.heap.new_object(None)?;
        put(&mut vm.heap, binding, CELL, Value::object(cell))?;
        put(&mut vm.heap, binding, KIND, Value::int(kind))?;
        put(&mut vm.heap, table, name, Value::object(binding))?;
        Ok(())
    }

    /// Declares an uninitialized local binding. Export it separately with `export`.
    ///
    /// Declare locals and exports before linking imports in a cyclic graph. For `var`
    /// or function hoisting the host must explicitly initialize before evaluating bodies.
    /// # Errors
    /// Returns missing-module, duplicate-binding or allocation errors.
    pub fn declare_binding(
        &self,
        vm: &mut Vm,
        id: &str,
        name: &str,
        kind: BindingKind,
    ) -> Result<(), ModuleError> {
        let cell = vm.heap.new_object(None)?;
        put(&mut vm.heap, cell, VALUE, Value::UNINITIALIZED)?;
        self.add_binding(
            vm,
            id,
            name,
            cell,
            match kind {
                BindingKind::Mutable => MUTABLE,
                BindingKind::Constant => CONSTANT,
            },
        )
    }

    /// Exports a local or imported binding under a name, including `default`.
    ///
    /// Exporting an imported binding is a live reexport of the original cell.
    /// # Errors
    /// Returns missing-module, missing-binding, duplicate-export or allocation errors.
    pub fn export(
        &self,
        vm: &mut Vm,
        id: &str,
        exported: &str,
        local: &str,
    ) -> Result<(), ModuleError> {
        let binding = self.binding(vm, id, local)?;
        let cell = record(&mut vm.heap, binding, CELL)?;
        let exports = self.table(vm, id, EXPORTS)?;
        if get(&mut vm.heap, exports, exported)?.is_some() {
            return Err(ModuleError::DuplicateBinding);
        }
        put(&mut vm.heap, exports, exported, Value::object(cell))?;
        Ok(())
    }

    /// Links a read-only local import to an already declared export cell.
    ///
    /// The exporting module need not have initialized or evaluated its bindings.
    /// # Errors
    /// Returns missing-module, missing-export, duplicate-local or allocation errors.
    pub fn import(
        &self,
        vm: &mut Vm,
        id: &str,
        local: &str,
        dependency: &str,
        exported: &str,
    ) -> Result<(), ModuleError> {
        let cell = self.export_cell(vm, dependency, exported)?;
        self.add_binding(vm, id, local, cell, IMPORT)
    }

    /// Reads a local binding, following import aliases and enforcing TDZ.
    ///
    /// # Errors
    /// Returns missing-module, missing-binding, uninitialized or allocation errors.
    pub fn read(&self, vm: &mut Vm, id: &str, local: &str) -> Result<Value, ModuleError> {
        let binding = self.binding(vm, id, local)?;
        let cell = record(&mut vm.heap, binding, CELL)?;
        read_cell(&mut vm.heap, cell)
    }

    /// Reads an exported binding's current value, enforcing TDZ.
    ///
    /// # Errors
    /// Returns missing-module, missing-export, uninitialized or allocation errors.
    pub fn read_export(&self, vm: &mut Vm, id: &str, exported: &str) -> Result<Value, ModuleError> {
        let cell = self.export_cell(vm, id, exported)?;
        read_cell(&mut vm.heap, cell)
    }

    /// Initializes a defining local once. Imports cannot initialize their target.
    ///
    /// # Errors
    /// Returns missing-binding/module, read-only, already-initialized, invalid-value
    /// or allocation errors.
    pub fn initialize(
        &self,
        vm: &mut Vm,
        id: &str,
        local: &str,
        value: Value,
    ) -> Result<(), ModuleError> {
        self.write(vm, id, local, value, Write::Initialize)
    }

    /// Assigns an initialized mutable defining local, updating all aliases immediately.
    ///
    /// # Errors
    /// Returns missing-binding/module, read-only, uninitialized, invalid-value or
    /// allocation errors.
    pub fn set(&self, vm: &mut Vm, id: &str, local: &str, value: Value) -> Result<(), ModuleError> {
        self.write(vm, id, local, value, Write::Assign)
    }

    fn write(
        &self,
        vm: &mut Vm,
        id: &str,
        local: &str,
        value: Value,
        operation: Write,
    ) -> Result<(), ModuleError> {
        if value.is_uninitialized() {
            return Err(ModuleError::InvalidValue);
        }
        let binding = self.binding(vm, id, local)?;
        let kind = get(&mut vm.heap, binding, KIND)?;
        if kind == Some(Value::int(IMPORT)) {
            return Err(ModuleError::ReadOnly);
        }
        let cell = record(&mut vm.heap, binding, CELL)?;
        let old = get(&mut vm.heap, cell, VALUE)?.expect("binding cell must contain a value");
        match operation {
            Write::Initialize if !old.is_uninitialized() => {
                return Err(ModuleError::AlreadyInitialized);
            }
            Write::Assign if old.is_uninitialized() => return Err(ModuleError::Uninitialized),
            Write::Assign if kind == Some(Value::int(CONSTANT)) => {
                return Err(ModuleError::ReadOnly);
            }
            _ => {}
        }
        put(&mut vm.heap, cell, VALUE, value)?;
        Ok(())
    }
}

enum Write {
    Initialize,
    Assign,
}

fn read_cell(heap: &mut Heap, cell: Obj) -> Result<Value, ModuleError> {
    let value = get(heap, cell, VALUE)?.expect("binding cell must contain a value");
    if value.is_uninitialized() {
        Err(ModuleError::Uninitialized)
    } else {
        Ok(value)
    }
}

/// Allocates a compiler binding record. Allocation never collects, so the compiler
/// can keep raw binding references until it roots the completed module template.
pub(crate) fn source_binding(
    heap: &mut Heap,
    locals: Obj,
    name: crate::heap::Atom,
    constant: bool,
    imported: bool,
    initial: Value,
) -> Result<Obj, HeapError> {
    let cell = heap.new_object(None)?;
    put(heap, cell, VALUE, initial)?;
    let binding = heap.new_object(None)?;
    put(heap, binding, CELL, Value::object(cell))?;
    put(
        heap,
        binding,
        KIND,
        Value::int(if imported {
            IMPORT
        } else if constant {
            CONSTANT
        } else {
            MUTABLE
        }),
    )?;
    heap.create_data_property(locals, name, Value::object(binding))?;
    Ok(binding)
}

pub(crate) fn compiled_read(heap: &mut Heap, binding: Obj) -> Result<Value, ModuleError> {
    let cell = record(heap, binding, CELL)?;
    read_cell(heap, cell)
}

pub(crate) fn compiled_write(
    heap: &mut Heap,
    binding: Obj,
    value: Value,
    initialize: bool,
) -> Result<(), ModuleError> {
    let kind = get(heap, binding, KIND)?;
    if kind == Some(Value::int(IMPORT)) {
        return Err(ModuleError::ReadOnly);
    }
    let cell = record(heap, binding, CELL)?;
    let previous = get(heap, cell, VALUE)?.expect("module binding value");
    if !initialize {
        if previous.is_uninitialized() {
            return Err(ModuleError::Uninitialized);
        }
        if kind == Some(Value::int(CONSTANT)) {
            return Err(ModuleError::ReadOnly);
        }
    }
    put(heap, cell, VALUE, value)?;
    Ok(())
}
