//! The dump loader (0170, docs/continuation-dumps.md, D2580/D2581): `Vm::load_dump`
//! runs a dump as a Script.
//!
//! - **Version 1** dumps (`// djs-dump 1`, D2353) are plain JavaScript: they make their own
//!   `$dump` and run as they are.
//! - **Format 2** dumps (first statement the directive `"djs-dump 2";`, D2640) get `$dump` from the loader: a global property
//!   (not writable, not enumerable, configurable) holding an ordinary object with the
//!   loader's primitives (read-only, D2622; the helpers in dump_prims.rs), removed again
//!   when the load ends. The dump adds its locations (`$dump.p1`) to it. The primitives are host functions with ids the loader reserves, served only by
//!   the loader's run loop: a dump run as an ordinary Script has no `$dump`, and a
//!   primitive that escaped the load parks the VM on a host call its embedder does not
//!   know.
//!
//! While a dump loads, classes skip their static initializers (`jump_if_loading`): the
//! dump restores static fields and private brands itself.

use alloc::string::String;
use alloc::vec::Vec;

use super::{Status, Vm};
use crate::compiler::{CompileOptions, FunctionMap};
use crate::heap::cells::{BoxCell, FuncTemplate, HostCall, Interp};
use crate::heap::{Handle, Heap, ObjectClass, PropFlags, Property, PropertyDescriptor};
use crate::interp::{ErrorKind, state};
use crate::value::{Obj, Sym, Value};

/// What `load_dump` needs besides the text.
#[derive(Clone, Copy)]
pub struct LoadOptions<'a> {
    /// The fuel the load may use (the Script and the jobs it queues).
    pub fuel: u64,
    /// The host function id for the name of an open call (`$dump.pending(id, name,
    /// args)`, e.g. `tools.read`); `None` refuses the load.
    pub host_fn: &'a dyn Fn(&str) -> Option<u32>,
}

fn no_host_fn(_: &str) -> Option<u32> {
    None
}

impl Default for LoadOptions<'_> {
    fn default() -> Self {
        LoadOptions {
            fuel: 10_000_000_000,
            host_fn: &no_host_fn,
        }
    }
}

/// A loaded dump: the VM is idle (no jobs, no frames) and waits on `calls`.
#[derive(Debug, Default)]
pub struct Loaded {
    /// The open host calls the dump recreated, in its order.
    pub calls: Vec<OpenCall>,
    /// The values the dump named for the host (`$dump.handle(name, v)`): heap handles,
    /// which the host drops when done.
    pub handles: Vec<(String, Handle)>,
}

/// An open host call of a loaded dump: its id (the host's from before the dump), the
/// function the host named for it, and the name the dump gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCall {
    pub id: u32,
    pub fn_id: u32,
    pub name: String,
}

/// Why a dump did not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The text does not start with a dump header.
    NotADump,
    /// A format this build does not know.
    Version(String),
    /// The Script does not compile.
    Compile(String),
    /// The dump (or a job it queued) threw; a loader primitive that refuses what it was
    /// given throws a TypeError saying why.
    Threw(String),
    /// It called a host function that is not a loader primitive.
    HostCall(u32),
    /// It used up `LoadOptions::fuel`.
    OutOfFuel,
    /// The heap or the host ran out of memory.
    Memory,
}

impl LoadError {
    pub fn message(&self) -> String {
        match self {
            LoadError::NotADump => {
                "not a djs dump (it does not start with `\"djs-dump 2\";` or `// djs-dump 1`)"
                    .into()
            }
            LoadError::Version(v) => {
                cat!("a djs dump of format ", v, ", which this build cannot load")
            }
            LoadError::Compile(m) => cat!("the dump does not compile: ", m),
            LoadError::Threw(m) => cat!("the dump threw: ", m),
            LoadError::HostCall(id) => cat!(
                "the dump called host function ",
                *id,
                ", not a loader primitive"
            ),
            LoadError::OutOfFuel => "the dump used up its fuel".into(),
            LoadError::Memory => "out of memory while loading the dump".into(),
        }
    }
}

/// The loader primitives' host function ids: the top of the id space, which embedders
/// are asked not to use.
const PRIVATE: u32 = 0xFFFF_FF00;
const RESUME: u32 = PRIVATE + 1;
const CELL: u32 = PRIVATE + 2;
const PENDING: u32 = PRIVATE + 3;
const HANDLE: u32 = PRIVATE + 4;
const ISSUED: u32 = PRIVATE + 5;
const HOST: u32 = PRIVATE + 6;
const BUILTIN: u32 = PRIVATE + 7;
const ADVANCE: u32 = PRIVATE + 8;
const FROM_SYNC: u32 = PRIVATE + 9;
const CLOSE_THROW: u32 = PRIVATE + 10;
const SPENT: u32 = PRIVATE + 11;
const REJECTED: u32 = PRIVATE + 12;
const BYTES: u32 = PRIVATE + 13;
const FN: u32 = PRIVATE + 14;
const KEEP: u32 = PRIVATE + 15;
const RANDOM: u32 = PRIVATE + 16;
const USING: u32 = PRIVATE + 17;
const FOR_IN: u32 = PRIVATE + 18;
const REGEXP_ITERATOR: u32 = PRIVATE + 19;
const ITERATOR_HELPER: u32 = PRIVATE + 20;
/// The ids of the primitives' own calls while a dump loads: above every id a dump's open
/// call may have, so the two never meet.
const LOADER_CALL_IDS: u32 = 0x8000_0000;
const PRIMITIVES: &[(&str, u32)] = &[
    ("private", PRIVATE),
    ("resume", RESUME),
    ("cell", CELL),
    ("pending", PENDING),
    ("handle", HANDLE),
    ("issued", ISSUED),
    ("host", HOST),
    ("builtin", BUILTIN),
    ("advance", ADVANCE),
    ("fromSync", FROM_SYNC),
    ("closeThrow", CLOSE_THROW),
    ("spent", SPENT),
    ("rejected", REJECTED),
    ("bytes", BYTES),
    ("fn", FN),
    ("keep", KEEP),
    ("random", RANDOM),
    ("using", USING),
    ("forIn", FOR_IN),
    ("regexpIterator", REGEXP_ITERATOR),
    ("iteratorHelper", ITERATOR_HELPER),
];

/// How a dump is compiled: a Script, TypeScript when its header says so
/// (`"djs-dump 2 typescript";`: it holds functions of TypeScript scripts, D2615).
pub(super) fn dump_options(typescript: bool) -> CompileOptions<'static> {
    CompileOptions {
        typescript,
        ..CompileOptions::script("dump.js")
    }
}

/// Whether a dump's header says it is TypeScript (`"djs-dump 2 typescript";`).
fn is_typescript(text: &str) -> bool {
    header(text).is_some_and(|(_, rest)| rest == " typescript")
}

/// The format of a dump: `N` of its first statement, the directive `"djs-dump N";`
/// (D2640), or of version 1's first line, the comment `// djs-dump 1` (D2353). The
/// comment form of any other version is not a dump: format 2 had it once, before
/// anything was promised across builds (D024).
pub fn dump_format(text: &str) -> Option<&str> {
    if let Some(rest) = text.strip_prefix("// djs-dump 1") {
        return (!rest.starts_with(|c: char| c.is_ascii_digit())).then_some("1");
    }
    header(text).map(|(v, _)| v)
}

/// The directive `"djs-dump N<rest>";` a dump starts with: `N` and `<rest>`.
fn header(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("\"djs-dump ")?;
    let line = rest.get(..rest.find('\n').unwrap_or(rest.len()))?;
    let line = line.trim_end().strip_suffix("\";")?;
    let end = line
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(line.len());
    let v = line.get(..end).filter(|v| !v.is_empty())?;
    Some((v, line.get(end..).unwrap_or_default()))
}

/// A load in progress.
pub(super) struct Load<'t> {
    pub text: &'t str,
    /// Compiled as TypeScript (`dump_options`).
    pub typescript: bool,
    /// The dump's Script function: functions its text made share its script.
    pub script: Handle,
    /// The suspension maps of the dump's functions, made on the first `resume`.
    pub maps: Option<Vec<FunctionMap>>,
    pub out: Loaded,
    /// `$dump.issued(n)`: the id the host's next call gets.
    issued: Option<u32>,
}

/// A value the loader gave out that stands for a slot's content: `$dump.TDZ` (a
/// binding before its declaration) and `$dump.cell(...)` (a captured variable's Box).
/// Both are internal Records with one slot, never handed to user code otherwise.
pub(super) enum Given {
    Value(Value),
    Tdz,
    Cell(u32),
}

pub(super) fn given(heap: &Heap, v: Value) -> Given {
    match v.as_object() {
        Some(o)
            if heap.object_class(o) == ObjectClass::Record && heap.object_slot_count(o) == 1 =>
        {
            let s = heap.object_slot(o, 0);
            match s.as_internal() {
                Some(b) if heap.cell_kind(b) == crate::heap::CellKind::BoxCell => Given::Cell(b),
                None if s == Value::UNINITIALIZED => Given::Tdz,
                _ => Given::Value(v),
            }
        }
        _ => Given::Value(v),
    }
}

/// `v` as a string, or why not.
pub(super) fn string_arg(heap: &Heap, v: Value, what: &str) -> Result<String, String> {
    v.as_string()
        .map(|s| heap.str_to_string(s))
        .ok_or_else(|| cat!(what, ": not a string"))
}

/// The elements of a dense Array (`undefined`: none).
pub(super) fn array_arg(heap: &Heap, v: Value, what: &str) -> Result<Vec<Value>, String> {
    if v.is_undefined() {
        return Ok(Vec::new());
    }
    let a = v
        .as_object()
        .filter(|&o| heap.object_class(o) == ObjectClass::Array)
        .ok_or_else(|| cat!(what, ": not an array"))?;
    (0..heap.array_length(a))
        .map(|i| {
            heap.get_element_fast(a, i)
                .ok_or_else(|| cat!(what, ": an array with holes"))
        })
        .collect()
}

/// The own data properties of an ordinary object by name (`undefined`: none).
pub(super) fn entries_arg(
    heap: &Heap,
    v: Value,
    what: &str,
) -> Result<Vec<(String, Value)>, String> {
    if v.is_undefined() {
        return Ok(Vec::new());
    }
    let o = v
        .as_object()
        .filter(|&o| heap.object_class(o) == ObjectClass::Ordinary)
        .ok_or_else(|| cat!(what, ": not a plain object"))?;
    let mut out = Vec::new();
    for k in heap.own_keys(o) {
        let name = heap.atom_name(k);
        match heap.get_own_property(o, k) {
            Some(Property::Data { value, .. }) => out.push((name, value)),
            _ => return Err(cat!(what, ": ", name, " is not a data property")),
        }
    }
    Ok(out)
}

impl Vm {
    /// Loads a dump into this VM: runs it as a Script (with the loader's `$dump` for
    /// format 2) and the jobs it queues, until nothing is left to run. Meant for a fresh
    /// VM: the dump's bindings become its globals. The VM is then idle, waiting on the
    /// open calls the dump recreated.
    pub fn load_dump(&mut self, text: &str, opts: &LoadOptions) -> Result<Loaded, LoadError> {
        let format = dump_format(text).ok_or(LoadError::NotADump)?;
        let privileged = match format {
            "1" => false,
            "2" => true,
            v => return Err(LoadError::Version(String::from(v))),
        };
        let f = self
            .compile(text.as_bytes(), &dump_options(is_typescript(text)))
            .map_err(|e| LoadError::Compile(e.message))?;
        if privileged {
            self.install_loader().map_err(|_| LoadError::Memory)?;
        }
        let script = self.heap.new_handle(f).map_err(|_| LoadError::Memory)?;
        let i = state::interp(&self.heap);
        let next_id = self.heap.arena().get(i, Interp::NEXT_CALL_ID);
        self.heap
            .arena_mut()
            .set(i, Interp::NEXT_CALL_ID, LOADER_CALL_IDS);
        let mut load = Load {
            text,
            typescript: is_typescript(text),
            script,
            maps: None,
            out: Loaded::default(),
            issued: None,
        };
        self.heap.loading_dump = true;
        let r = self.drive_load(f, opts, &mut load);
        self.heap.loading_dump = false;
        self.heap.drop_handle(script);
        if privileged {
            let g = self.global();
            if let Ok(k) = self.heap.atom_str("$dump") {
                let _ = self.heap.delete_own_property(g, k);
            }
        }
        // The host's next call gets the id the dump says, else the one it would have
        // got, and never one of an open call.
        let open = load.out.calls.iter().map(|c| c.id + 1).max().unwrap_or(0);
        let next = load.issued.unwrap_or(next_id).max(open).max(1);
        let i = state::interp(&self.heap);
        self.heap.arena_mut().set(i, Interp::NEXT_CALL_ID, next);
        if let Err(e) = r {
            for (_, h) in load.out.handles {
                self.heap.drop_handle(h);
            }
            return Err(e);
        }
        Ok(load.out)
    }

    /// Defines the global `$dump` with the loader's primitives.
    fn install_loader(&mut self) -> Result<(), crate::heap::HeapError> {
        let o = self.new_object()?;
        // Read-only: a dump calls the primitives and defines nothing of its own on
        // `$dump` but the locations it fills (D2622).
        for &(name, id) in PRIMITIVES {
            let f = self.new_host_function(id, name)?;
            let k = self.heap.atom_str(name)?;
            let d = PropertyDescriptor::data(f, PropFlags::NONE);
            self.heap.define_own_property(o, k, &d)?;
        }
        let tdz = self.heap.alloc_object(ObjectClass::Record, None, 1)?;
        self.heap.set_object_slot(tdz, 0, Value::UNINITIALIZED);
        let k = self.heap.atom_str("TDZ")?;
        let d = PropertyDescriptor::data(Value::object(tdz), PropFlags::NONE);
        self.heap.define_own_property(o, k, &d)?;
        let g = self.global();
        let k = self.heap.atom_str("$dump")?;
        let d = PropertyDescriptor::data(Value::object(o), PropFlags::CONFIGURABLE);
        self.heap.define_own_property(g, k, &d)?;
        Ok(())
    }

    fn drive_load(
        &mut self,
        f: Value,
        opts: &LoadOptions,
        load: &mut Load,
    ) -> Result<(), LoadError> {
        const SLICE: u64 = 10_000_000;
        let this = Value::object(self.global());
        self.call(f, this, &[]).map_err(|_| LoadError::Memory)?;
        let mut used = 0u64;
        loop {
            match self.run(SLICE.min(opts.fuel.saturating_sub(used).max(1))) {
                Status::Done(_) | Status::Idle => return Ok(()),
                Status::Threw(e) => return Err(LoadError::Threw(self.display(e))),
                Status::OutOfFuel => {
                    used = used.saturating_add(SLICE);
                    if used >= opts.fuel {
                        self.abort();
                        return Err(LoadError::OutOfFuel);
                    }
                }
                Status::HostOutOfMemory => return Err(LoadError::Memory),
                Status::HostCall { call_id, fn_id } => {
                    let args = self.host_call(call_id).map(|i| i.args).unwrap_or_default();
                    let r = match fn_id {
                        PRIVATE => self.primitive_private(&args),
                        RESUME => self.primitive_resume(&args, load),
                        CELL => self.primitive_cell(&args),
                        PENDING => self.primitive_pending(&args, opts, load),
                        HANDLE => self.primitive_handle(&args, load),
                        HOST => self.primitive_host(&args, opts),
                        BUILTIN => self.primitive_builtin(&args),
                        ADVANCE => self.primitive_advance(&args),
                        FROM_SYNC => self.primitive_from_sync(&args),
                        CLOSE_THROW => self.primitive_close_throw(&args),
                        SPENT => self.primitive_spent(&args),
                        REJECTED => self.primitive_rejected(&args),
                        #[cfg(feature = "full")]
                        BYTES => self.primitive_bytes(&args),
                        FN => self.primitive_fn(&args),
                        KEEP => self.primitive_keep(&args),
                        RANDOM => self.primitive_random(&args),
                        USING => self.primitive_using(&args),
                        FOR_IN => self.primitive_for_in(&args),
                        REGEXP_ITERATOR => self.primitive_regexp_iterator(&args),
                        ITERATOR_HELPER => self.primitive_iterator_helper(&args),
                        // (A positive int32: below LOADER_CALL_IDS.)
                        ISSUED => match args.first().and_then(|v| v.as_int()) {
                            Some(n) if n > 0 => {
                                load.issued = Some(n as u32);
                                Ok(Value::UNDEFINED)
                            }
                            _ => Err(String::from("$dump.issued: not a call id")),
                        },
                        id => {
                            self.abort();
                            return Err(LoadError::HostCall(id));
                        }
                    };
                    let r = match r {
                        Ok(v) => Ok(v),
                        Err(m) => Err(self
                            .new_error(ErrorKind::TypeError, &m)
                            .map_err(|_| LoadError::Memory)?),
                    };
                    self.resolve_host_call(call_id, r)
                        .map_err(|_| LoadError::Memory)?;
                }
            }
        }
    }

    /// `$dump.cell(() => x)`: the Box of the one variable the function captures, as a
    /// value `resume` takes for a binding, so that the frame and the closures share it.
    fn primitive_cell(&mut self, args: &[Value]) -> Result<Value, String> {
        let f = args
            .first()
            .and_then(|v| v.as_object())
            .filter(|&f| self.heap.object_class(f) == ObjectClass::Function)
            .ok_or("$dump.cell: not a function")?;
        let t = self.heap.closure_template(f);
        if self.heap.arena().get::<u16>(t, FuncTemplate::CAPTURES) != 1 {
            return Err("$dump.cell: the function must capture exactly one variable".into());
        }
        let b = self
            .heap
            .object_slot(f, 0)
            .as_internal()
            .ok_or("$dump.cell: no captured variable")?;
        let r = self
            .heap
            .alloc_object(ObjectClass::Record, None, 1)
            .map_err(|e| String::from(e.as_str()))?;
        self.heap.set_object_slot(r, 0, Value::internal(b));
        Ok(Value::object(r))
    }

    /// `$dump.pending(id, name, args)`: the open async host call `id` of the function the
    /// host knows as `name`; its promise.
    fn primitive_pending(
        &mut self,
        args: &[Value],
        opts: &LoadOptions,
        load: &mut Load,
    ) -> Result<Value, String> {
        let arg = |i: usize| args.get(i).copied().unwrap_or(Value::UNDEFINED);
        let id = arg(0)
            .as_int()
            .filter(|&n| n > 0)
            .ok_or("$dump.pending: not a call id")? as u32;
        // (Below LOADER_CALL_IDS, as every positive int32 is.)
        let name = string_arg(&self.heap, arg(1), "$dump.pending")?;
        let values = array_arg(&self.heap, arg(2), "$dump.pending")?;
        if crate::interp::run::host_call_cell(&self.heap, id).is_some() {
            return Err(cat!("$dump.pending: call ", id, " is open already"));
        }
        let fn_id = (opts.host_fn)(&name)
            .ok_or_else(|| cat!("$dump.pending: unknown host function ", name))?;
        let oom = |e: crate::heap::HeapError| String::from(e.as_str());
        let arr = self
            .heap
            .alloc_value_array(values.len() as u32)
            .map_err(oom)?;
        for (i, &v) in values.iter().enumerate() {
            self.heap.value_array_set(arr, i as u32, v);
        }
        self.heap.arena_mut().set(
            arr,
            crate::heap::cells::ValueArray::LEN,
            values.len() as u32,
        );
        let proto =
            crate::interp::realm::intrinsic_obj(&self.heap, crate::interp::Intrinsic::PromiseProto);
        let p = crate::interp::run::promise::alloc(&mut self.heap, Some(proto)).map_err(oom)?;
        crate::interp::run::add_loaded_host_call(&mut self.heap, id, fn_id, arr, Value::object(p))
            .map_err(oom)?;
        // The host has it already: `new_host_calls` does not report it again.
        if let Some(c) = crate::interp::run::host_call_cell(&self.heap, id) {
            let flags = self.heap.arena().get(c, HostCall::FLAGS);
            self.heap
                .arena_mut()
                .set(c, HostCall::FLAGS, flags | super::HOST_CALL_REPORTED);
        }
        load.out.calls.push(OpenCall { id, fn_id, name });
        Ok(Value::object(p))
    }

    /// `$dump.host(name, isAsync)`: a host function the host knows as `name`
    /// (`LoadOptions::host_fn`), held as a value by the dumped session.
    fn primitive_host(&mut self, args: &[Value], opts: &LoadOptions) -> Result<Value, String> {
        let name = string_arg(
            &self.heap,
            args.first().copied().unwrap_or(Value::UNDEFINED),
            "$dump.host",
        )?;
        let id = (opts.host_fn)(&name)
            .ok_or_else(|| cat!("$dump.host: unknown host function ", name))?;
        let r = if args.get(1).copied() == Some(Value::TRUE) {
            self.new_async_host_function(id, &name)
        } else {
            self.new_host_function(id, &name)
        };
        r.map_err(|e| String::from(e.as_str()))
    }

    /// `$dump.handle(name, v)`: `v` for the host, under `name`.
    fn primitive_handle(&mut self, args: &[Value], load: &mut Load) -> Result<Value, String> {
        let name = string_arg(
            &self.heap,
            args.first().copied().unwrap_or(Value::UNDEFINED),
            "$dump.handle",
        )?;
        if load.out.handles.iter().any(|(n, _)| *n == name) {
            return Err(cat!("$dump.handle: two handles named ", name));
        }
        let v = args.get(1).copied().unwrap_or(Value::UNDEFINED);
        let h = self
            .heap
            .new_handle(v)
            .map_err(|e| String::from(e.as_str()))?;
        load.out.handles.push((name, h));
        Ok(Value::UNDEFINED)
    }

    /// `$dump.private(obj, C, "#x", value)`: the private field `#x` of the class `C` the
    /// dump recreated, on `obj`; without `value`, the private method or accessor `#x`
    /// (its brand).
    fn primitive_private(&mut self, args: &[Value]) -> Result<Value, String> {
        let arg = |i: usize| args.get(i).copied().unwrap_or(Value::UNDEFINED);
        if args.len() == 2 {
            // `$dump.private(C, "#x")`: the private name itself, a value a frame holds
            // (a binding, the key operand of `this.#x += await v`).
            let class = arg(0)
                .as_object()
                .filter(|c| is_class(&self.heap, *c))
                .ok_or("$dump.private: not a class")?;
            let name = arg(1)
                .as_string()
                .map(|s| self.heap.str_to_string(s))
                .ok_or("$dump.private: not a private name")?;
            let privs = class_privates(&self.heap, class);
            let p = privs
                .iter()
                .find(|p| p.name == name && p.sym.0 != 0)
                .ok_or_else(|| cat!("$dump.private: the class has no private name ", name))?;
            if p.ambiguous {
                return Err(cat!("$dump.private: two private names ", name));
            }
            return Ok(Value::symbol(p.sym));
        }
        let obj = arg(0).as_object().ok_or("$dump.private: not an object")?;
        let class = arg(1)
            .as_object()
            .filter(|c| is_class(&self.heap, *c))
            .ok_or("$dump.private: not a class")?;
        let name = arg(2)
            .as_string()
            .map(|s| self.heap.str_to_string(s))
            .ok_or("$dump.private: not a private name")?;
        let privs = class_privates(&self.heap, class);
        let p = privs
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| cat!("$dump.private: the class has no private name ", name))?;
        if p.ambiguous {
            return Err(cat!("$dump.private: two private names ", name));
        }
        let key = self.heap.symbol_atom(p.sym);
        if self.heap.has_own_property(obj, key) {
            return Err(cat!("$dump.private: ", name, " is already present"));
        }
        let prop = if args.len() > 3 {
            Property::data(arg(3), PropFlags::WRITABLE)
        } else if !p.method.is_undefined() {
            Property::data(p.method, PropFlags::NONE)
        } else if !p.getter.is_undefined() || !p.setter.is_undefined() {
            Property::Accessor {
                getter: p.getter,
                setter: p.setter,
                flags: PropFlags::NONE,
            }
        } else {
            return Err(cat!(
                "$dump.private: ",
                name,
                " is a field: it needs a value"
            ));
        };
        self.heap
            .add_private_property(obj, key, &prop)
            .map_err(|e| String::from(e.as_str()))?;
        Ok(Value::UNDEFINED)
    }
}

/// Whether `f` is a class constructor.
pub(super) fn is_class(h: &Heap, f: Obj) -> bool {
    h.object_class(f) == ObjectClass::Function && {
        let t = h.closure_template(f);
        let kind = crate::bytecode::template::func_flags::kind(u32::from(
            h.arena().get::<u16>(t, FuncTemplate::FLAGS),
        ));
        matches!(
            kind,
            crate::bytecode::template::func_flags::KIND_CLASS_CTOR
                | crate::bytecode::template::func_flags::KIND_DERIVED_CTOR
        )
    }
}

/// A private name of a class: its symbol, and for a private method or accessor the
/// closures the class installs on its instances.
pub(super) struct ClassPrivate {
    pub name: String,
    pub sym: Sym,
    pub method: Value,
    pub getter: Value,
    pub setter: Value,
    /// Two different symbols of this name were found (a nested class's).
    pub ambiguous: bool,
}

/// The private names of class `c` that its code can use: the variables named `#x` that
/// its constructor, its methods and accessors, and the hidden closures they hold (field
/// initializers, private methods) capture. A private name no closure of the class
/// captures can no longer be read by anyone.
pub(super) fn class_privates(h: &Heap, c: Obj) -> Vec<ClassPrivate> {
    let mut out: Vec<ClassPrivate> = Vec::new();
    let mut queue: Vec<Obj> = alloc::vec![c];
    let mut seen: Vec<u32> = alloc::vec![c.offset()];
    let push = |queue: &mut Vec<Obj>, seen: &mut Vec<u32>, v: Value| {
        if let Some(f) = v.as_object()
            && h.object_class(f) == ObjectClass::Function
            && (f == c || !is_class(h, f))
            && !seen.contains(&f.offset())
        {
            seen.push(f.offset());
            queue.push(f);
        }
    };
    // Members: the class's own and its prototype's methods and accessors.
    let mut holders = alloc::vec![c];
    if let Some(Property::Data { value, .. }) = h.get_own_property(c, crate::heap::atoms::PROTOTYPE)
        && let Some(p) = value.as_object()
    {
        holders.push(p);
    }
    for o in holders {
        for k in h.own_keys(o) {
            match h.get_own_property(o, k) {
                Some(Property::Data { value, .. }) => push(&mut queue, &mut seen, value),
                Some(Property::Accessor { getter, setter, .. }) => {
                    push(&mut queue, &mut seen, getter);
                    push(&mut queue, &mut seen, setter);
                }
                None => {}
            }
        }
    }
    let mut i = 0;
    while i < queue.len() {
        let f = queue[i];
        i += 1;
        let t = h.closure_template(f);
        let a = h.arena();
        let locals = usize::from(a.get::<u16>(t, FuncTemplate::LOCALS));
        let captures = u32::from(a.get::<u16>(t, FuncTemplate::CAPTURES));
        let names = super::dump::template_names(h, t);
        for j in 0..captures {
            let Some(name) = names.get(locals + j as usize) else {
                continue;
            };
            let Some(b) = h.object_slot(f, j).as_internal() else {
                continue;
            };
            let v: Value = a.get(b, BoxCell::VALUE);
            if name.starts_with('#') {
                let Some(sym) = v.as_symbol() else { continue };
                match out.iter_mut().find(|p| p.name == *name) {
                    Some(p) if p.sym.0 == 0 => p.sym = sym,
                    Some(p) => p.ambiguous |= p.sym != sym,
                    None => out.push(ClassPrivate {
                        name: name.clone(),
                        sym,
                        method: Value::UNDEFINED,
                        getter: Value::UNDEFINED,
                        setter: Value::UNDEFINED,
                        ambiguous: false,
                    }),
                }
            } else if let Some(inner) = name.strip_prefix('<').and_then(|n| n.strip_suffix('>')) {
                // `<#m>`, `<get #x>`, `<set #x>`: a private method's closures; `<fields
                // N>` and others: hidden closures of the class.
                let (slot, pname) = match inner.split_once(' ') {
                    Some(("get", n)) if n.starts_with('#') => (1, n),
                    Some(("set", n)) if n.starts_with('#') => (2, n),
                    _ if inner.starts_with('#') => (0, inner),
                    _ => (3, ""),
                };
                if slot < 3 {
                    let p = match out.iter_mut().position(|p| p.name == pname) {
                        Some(p) => p,
                        None => {
                            // The method's name, before its symbol is seen.
                            out.push(ClassPrivate {
                                name: String::from(pname),
                                sym: Sym(0),
                                method: Value::UNDEFINED,
                                getter: Value::UNDEFINED,
                                setter: Value::UNDEFINED,
                                ambiguous: false,
                            });
                            out.len() - 1
                        }
                    };
                    match slot {
                        0 => out[p].method = v,
                        1 => out[p].getter = v,
                        _ => out[p].setter = v,
                    }
                }
                push(&mut queue, &mut seen, v);
            }
        }
    }
    out.retain(|p| p.sym.0 != 0);
    out
}
