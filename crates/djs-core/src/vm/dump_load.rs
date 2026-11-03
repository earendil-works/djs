//! The dump loader (0170, docs/continuation-dumps.md, D2580/D2581): `Vm::load_dump`
//! runs a dump as a Script.
//!
//! - **Version 1** dumps (`// djs-dump 1`, D2353) are plain JavaScript: they make their own
//!   `$dump` and run as they are.
//! - **Format 2** dumps (`// djs-dump 2`) get `$dump` from the loader: a global property
//!   (not writable, not enumerable, configurable) holding an ordinary object with the
//!   loader's primitives, removed again when the load ends. The dump adds its helpers to
//!   it. The primitives are host functions with ids the loader reserves, served only by
//!   the loader's run loop: a dump run as an ordinary Script has no `$dump`, and a
//!   primitive that escaped the load parks the VM on a host call its embedder does not
//!   know.
//!
//! While a dump loads, classes skip their static initializers (`jump_if_loading`): the
//! dump restores static fields and private brands itself.

use alloc::string::String;
use alloc::vec::Vec;

use super::{Status, Vm};
use crate::compiler::CompileOptions;
use crate::heap::cells::{BoxCell, FuncTemplate};
use crate::heap::{Heap, ObjectClass, PropFlags, Property, PropertyDescriptor};
use crate::interp::ErrorKind;
use crate::value::{Obj, Sym, Value};

/// What `load_dump` needs besides the text.
#[derive(Clone, Copy, Debug)]
pub struct LoadOptions {
    /// The fuel the load may use (the Script and the jobs it queues).
    pub fuel: u64,
}

impl Default for LoadOptions {
    fn default() -> Self {
        LoadOptions { fuel: 10_000_000_000 }
    }
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
    /// The dump (or a job it queued) threw.
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
            LoadError::NotADump => "not a djs dump (no `// djs-dump N` header)".into(),
            LoadError::Version(v) => cat!("a djs dump of format ", v, ", which this build cannot load"),
            LoadError::Compile(m) => cat!("the dump does not compile: ", m),
            LoadError::Threw(m) => cat!("the dump threw: ", m),
            LoadError::HostCall(id) => cat!("the dump called host function ", *id, ", not a loader primitive"),
            LoadError::OutOfFuel => "the dump used up its fuel".into(),
            LoadError::Memory => "out of memory while loading the dump".into(),
        }
    }
}

/// The loader primitives' host function ids: the top of the id space, which embedders
/// are asked not to use.
const PRIVATE: u32 = 0xFFFF_FF00;
const PRIMITIVES: &[(&str, u32)] = &[("private", PRIVATE)];

/// The format of a dump: the number in its first line, `// djs-dump N`.
pub fn dump_format(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("// djs-dump ")?;
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest.get(..end).filter(|v| !v.is_empty())
}

impl Vm {
    /// Loads a dump into this VM: runs it as a Script (with the loader's `$dump` for
    /// format 2) and the jobs it queues, until nothing is left to run. Meant for a fresh
    /// VM: the dump's bindings become its globals.
    pub fn load_dump(&mut self, text: &str, opts: &LoadOptions) -> Result<(), LoadError> {
        let format = dump_format(text).ok_or(LoadError::NotADump)?;
        let privileged = match format {
            "1" => false,
            "2" => true,
            v => return Err(LoadError::Version(String::from(v))),
        };
        let f = self
            .compile(text.as_bytes(), &CompileOptions::script("dump.js"))
            .map_err(|e| LoadError::Compile(e.message))?;
        if privileged {
            self.install_loader().map_err(|_| LoadError::Memory)?;
        }
        self.heap.loading_dump = true;
        let r = self.drive_load(f, opts);
        self.heap.loading_dump = false;
        if privileged {
            let g = self.global();
            if let Ok(k) = self.heap.atom_str("$dump") {
                let _ = self.heap.delete_own_property(g, k);
            }
        }
        r
    }

    /// Defines the global `$dump` with the loader's primitives.
    fn install_loader(&mut self) -> Result<(), crate::heap::HeapError> {
        let o = self.new_object()?;
        for &(name, id) in PRIMITIVES {
            let f = self.new_host_function(id, name)?;
            self.define(o, name, f)?;
        }
        let g = self.global();
        let k = self.heap.atom_str("$dump")?;
        let d = PropertyDescriptor::data(Value::object(o), PropFlags::CONFIGURABLE);
        self.heap.define_own_property(g, k, &d)?;
        Ok(())
    }

    fn drive_load(&mut self, f: Value, opts: &LoadOptions) -> Result<(), LoadError> {
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
                    let r = match fn_id {
                        PRIVATE => self.primitive_private(call_id),
                        id => {
                            self.abort();
                            return Err(LoadError::HostCall(id));
                        }
                    };
                    let r = match r {
                        Ok(v) => Ok(v),
                        Err(m) => Err(self.new_error(ErrorKind::TypeError, &m).map_err(|_| LoadError::Memory)?),
                    };
                    self.resolve_host_call(call_id, r)
                        .map_err(|_| LoadError::Memory)?;
                }
            }
        }
    }

    /// `$dump.private(obj, C, "#x", value)`: the private field `#x` of the class `C` the
    /// dump recreated, on `obj`; without `value`, the private method or accessor `#x`
    /// (its brand).
    fn primitive_private(&mut self, call_id: u32) -> Result<Value, String> {
        let info = self.host_call(call_id).ok_or("no host call")?;
        let arg = |i: usize| info.args.get(i).copied().unwrap_or(Value::UNDEFINED);
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
        let prop = if info.args.len() > 3 {
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
            return Err(cat!("$dump.private: ", name, " is a field: it needs a value"));
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
