//! Import: a logical snapshot into a fresh VM (see the module docs of `snapshot`).

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::export::Slots;
use super::export::*;
use super::schema::{self, SlotTy};
use super::wire::{Reader, Short};
use super::*;
use crate::bytecode::template;
use crate::heap::cells::{Interp, Object, Roots, ValueArray};
use crate::heap::layout::{CellLayout, FieldInfo, FieldTy};
use crate::heap::{Atom, CellKind, Heap, ObjectClass, PropFlags, Property, PropertyDescriptor};
use crate::interp::realm::{self, Intrinsic, intrinsic};
use crate::interp::state;
use crate::value::{Obj, Str, Sym, Value, tag};

/// Counts of an import, for measurements.
#[derive(Default, Debug, Clone)]
pub struct SnapshotStats {
    pub records: usize,
    pub strings: usize,
    pub scripts: usize,
    pub functions: usize,
    pub bases: usize,
    /// Records by type name.
    pub types: BTreeMap<String, usize>,
}

/// A value as the snapshot has it.
#[derive(Clone, Copy, Debug)]
enum V {
    Imm(Value),
    Str(u32),
    Rec(u32, bool),
    Base(u32, bool),
    Fn(u32),
    Script(u32),
    /// A host value or function (D3720): its host entry.
    Host(u32),
}

/// A host entry as the snapshot has it (export's `HostEntry`, string ids).
#[derive(Clone, Copy, Debug)]
enum HostEntry {
    Root(u32),
    Member(u32, u32),
    Function(u32),
}

#[derive(Clone, Debug)]
struct P {
    key: V,
    bits: u64,
    a: V,
    b: V,
}

#[derive(Clone, Debug)]
enum F {
    V(V),
    U(u64),
    I(i64),
    F64(f64),
    Vs(Vec<V>),
    Props(Vec<P>),
    Tail(Vec<Vec<F>>),
    Fn(u32),
    Intr(u32),
    /// A host call's function, by its host entry (D3720).
    HostFn(u32),
    Buckets(u64),
    Flags(u64),
    /// A named constant field's member index (an enum) or members' bits (D3710).
    Const(u64),
    None,
}

#[derive(Clone, Debug, PartialEq)]
enum Ty {
    Value,
    Ref,
    Uint,
    Int,
    F64,
    Values,
    Props,
    Function,
    Intrinsic,
    HostFn,
    Buckets,
    Program,
    Flags(Vec<String>),
    /// A field of named constants (D3710): the set and its member names by index.
    Enum(String, Vec<String>),
    Bits(String, Vec<String>),
    Tail(Vec<(String, Ty)>),
}

struct TypeInfo {
    name: String,
    fields: Vec<(String, Ty)>,
}

fn parse_ty(s: &str, sets: &BTreeMap<String, Vec<String>>) -> Option<Ty> {
    Some(match s {
        "value" => Ty::Value,
        "ref" => Ty::Ref,
        "uint" => Ty::Uint,
        "int" => Ty::Int,
        "f64" => Ty::F64,
        "values" => Ty::Values,
        "props" => Ty::Props,
        "function" => Ty::Function,
        "intrinsic" => Ty::Intrinsic,
        "hostfn" => Ty::HostFn,
        "buckets" => Ty::Buckets,
        "program" => Ty::Program,
        _ => {
            if let Some(f) = s.strip_prefix("flags:") {
                return Some(Ty::Flags(sets.get(f)?.clone()));
            }
            if let Some(f) = s.strip_prefix("enum:") {
                return Some(Ty::Enum(f.to_string(), sets.get(f)?.clone()));
            }
            if let Some(f) = s.strip_prefix("bits:") {
                return Some(Ty::Bits(f.to_string(), sets.get(f)?.clone()));
            }
            let inner = s.strip_prefix("tail(")?.strip_suffix(')')?;
            if let Some(r) = inner
                .strip_prefix("record(")
                .and_then(|r| r.strip_suffix(')'))
            {
                let mut fs = Vec::new();
                for part in r.split(',') {
                    let (n, t) = part.split_once(':')?;
                    fs.push((n.to_string(), parse_ty(t, sets)?));
                }
                return Some(Ty::Tail(fs));
            }
            Ty::Tail(alloc::vec![(String::new(), parse_ty(inner, sets)?)])
        }
    })
}

/// A script: name and parameters (string ids), options, the source or its hash.
type ScriptEntry<'a> = (u32, u32, u32, Result<&'a [u8], u64>);
/// A function: script, span, pinned, hash, template objects by constant index.
type FunctionEntry = (u32, u32, u32, bool, u64, Vec<(u32, V)>);

struct Snap<'a> {
    types: Vec<TypeInfo>,
    strings: Vec<(bool, &'a [u8])>,
    bases: Vec<u32>,
    intrinsics: Vec<u32>,
    host: Vec<HostEntry>,
    /// Name, parameters, options, and the source or its hash.
    scripts: Vec<ScriptEntry<'a>>,
    /// Script, span, whether the bytecode is pinned, the hash, the template objects made
    /// at its tagged templates (by constant index).
    functions: Vec<FunctionEntry>,
    records: Vec<(u32, Vec<F>)>,
    root: (u32, Vec<F>),
    /// Where each section starts, for `describe`.
    marks: Vec<(&'static str, usize)>,
}

fn short(_: Short) -> Refusal {
    Refusal(alloc::vec![String::from(
        "the snapshot is truncated or malformed"
    )])
}

fn refusal(s: String) -> Refusal {
    Refusal(alloc::vec![s])
}

impl<'a> Snap<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Snap<'a>, Refusal> {
        let mut r = Reader::new(bytes);
        if r.raw(6).map_err(short)? != MAGIC {
            return Err(refusal(String::from("not a djs logical snapshot")));
        }
        let v = r.byte().map_err(short)?;
        if v != FORMAT {
            return Err(refusal(format!(
                "format version {v}, this build reads {FORMAT}"
            )));
        }
        let mut s = Snap {
            types: Vec::new(),
            strings: Vec::new(),
            bases: Vec::new(),
            intrinsics: Vec::new(),
            host: Vec::new(),
            scripts: Vec::new(),
            functions: Vec::new(),
            records: Vec::new(),
            root: (0, Vec::new()),
            marks: Vec::new(),
        };
        s.marks.push(("types", r.at));
        let mut sets = BTreeMap::new();
        let n = r.u().map_err(short)?;
        for _ in 0..n {
            let name = r.str().map_err(short)?.to_string();
            let k = r.u().map_err(short)?;
            let mut names = Vec::new();
            for _ in 0..k {
                names.push(r.str().map_err(short)?.to_string());
            }
            sets.insert(name, names);
        }
        let n = r.u().map_err(short)?;
        for _ in 0..n {
            let name = r.str().map_err(short)?.to_string();
            let nf = r.u().map_err(short)?;
            let mut fields = Vec::new();
            for _ in 0..nf {
                let f = r.str().map_err(short)?.to_string();
                let t = r.str().map_err(short)?;
                let ty = parse_ty(t, &sets)
                    .ok_or_else(|| refusal(format!("{name}.{f}: unknown field type {t}")))?;
                fields.push((f, ty));
            }
            let _digest = r.u64_raw().map_err(short)?;
            s.types.push(TypeInfo { name, fields });
        }
        s.marks.push(("strings", r.at));
        let n = r.u().map_err(short)?;
        for _ in 0..n {
            let l = r.u().map_err(short)?;
            let data = r.raw((l >> 1) as usize).map_err(short)?;
            s.strings.push((l & 1 != 0, data));
        }
        s.marks.push(("builtins", r.at));
        let n = r.u().map_err(short)?;
        for _ in 0..n {
            s.bases.push(r.u32().map_err(short)?);
        }
        s.marks.push(("intrinsics", r.at));
        let n = r.u().map_err(short)?;
        for _ in 0..n {
            s.intrinsics.push(r.u32().map_err(short)?);
        }
        s.marks.push(("host", r.at));
        let n = r.u().map_err(short)?;
        for i in 0..n {
            s.host.push(match r.byte().map_err(short)? {
                0 => HostEntry::Root(r.u32().map_err(short)?),
                // A member's parent comes before it.
                1 => match r.u32().map_err(short)? {
                    p if u64::from(p) < i => HostEntry::Member(p, r.u32().map_err(short)?),
                    _ => return Err(short(Short)),
                },
                2 => HostEntry::Function(r.u32().map_err(short)?),
                _ => return Err(short(Short)),
            });
        }
        s.marks.push(("scripts", r.at));
        let n = r.u().map_err(short)?;
        for _ in 0..n {
            let name = r.u32().map_err(short)?;
            let params = r.u32().map_err(short)?;
            let bits = r.u32().map_err(short)?;
            let src = match r.byte().map_err(short)? {
                0 => Ok(r.bytes().map_err(short)?),
                _ => Err(r.u64_raw().map_err(short)?),
            };
            s.scripts.push((name, params, bits, src));
        }
        s.marks.push(("functions", r.at));
        r.reset_refs();
        let n = r.u().map_err(short)?;
        for _ in 0..n {
            let script = r.u32().map_err(short)?;
            let start = r.u32().map_err(short)?;
            let end = r.u32().map_err(short)?;
            let pinned = r.byte().map_err(short)? != 0;
            let hash = r.u64_raw().map_err(short)?;
            let nc = r.u().map_err(short)?;
            let mut sites = Vec::new();
            for _ in 0..nc {
                let i = r.u32().map_err(short)?;
                sites.push((i, value(&mut r).map_err(short)?));
            }
            s.functions.push((script, start, end, pinned, hash, sites));
        }
        s.marks.push(("records", r.at));
        r.reset_refs();
        let n = r.u().map_err(short)?;
        s.records.reserve(n as usize);
        for _ in 0..n {
            let t = r.u32().map_err(short)?;
            let ty = s.types.get(t as usize).ok_or_else(|| short(Short))?;
            let fields = fields(&mut r, &ty.fields).map_err(short)?;
            s.records.push((t, fields));
        }
        s.marks.push(("vm", r.at));
        r.reset_refs();
        let t = r.u32().map_err(short)?;
        let ty = s.types.get(t as usize).ok_or_else(|| short(Short))?;
        if ty.name != "Vm" {
            return Err(short(Short));
        }
        s.root = (t, fields(&mut r, &ty.fields).map_err(short)?);
        s.marks.push(("end", r.at));
        Ok(s)
    }
}

fn value(r: &mut Reader<'_>) -> Result<V, Short> {
    Ok(match r.byte()? {
        V_UNDEF => V::Imm(Value::UNDEFINED),
        V_NULL => V::Imm(Value::NULL),
        V_FALSE => V::Imm(Value::FALSE),
        V_TRUE => V::Imm(Value::TRUE),
        V_INT => V::Imm(Value::int(i32::try_from(r.i()?).map_err(|_| Short)?)),
        // A number only: tagged bits would make a reference out of nothing.
        V_F64 => V::Imm(
            Value::from_bits(r.u64_raw()?)
                .filter(|v| v.tag().is_none())
                .ok_or(Short)?,
        ),
        V_HOLE => V::Imm(Value::HOLE),
        V_UNINIT => V::Imm(Value::UNINITIALIZED),
        V_STR => V::Str(r.str_ref()?),
        V_REC => V::Rec(r.rec_ref()?, false),
        V_REC_INTERNAL => V::Rec(r.rec_ref()?, true),
        V_BASE => V::Base(r.u32()?, false),
        V_BASE_INTERNAL => V::Base(r.u32()?, true),
        V_FN => V::Fn(r.u32()?),
        V_SCRIPT => V::Script(r.u32()?),
        V_HOST => V::Host(r.u32()?),
        _ => return Err(Short),
    })
}

fn values(r: &mut Reader<'_>) -> Result<Vec<V>, Short> {
    let n = r.u()? as usize;
    let mut out = Vec::with_capacity(n.min(r.buf.len()));
    for _ in 0..n {
        out.push(value(r)?);
    }
    Ok(out)
}

fn field(r: &mut Reader<'_>, t: &Ty) -> Result<F, Short> {
    Ok(match t {
        Ty::Value | Ty::Ref => F::V(value(r)?),
        Ty::Uint => F::U(r.u()?),
        Ty::Int => F::I(r.i()?),
        Ty::F64 => F::F64(r.f64()?),
        Ty::Values => F::Vs(values(r)?),
        Ty::Props => {
            let n = r.u()? as usize;
            let mut out = Vec::with_capacity(n.min(r.buf.len()));
            for _ in 0..n {
                let key = value(r)?;
                let bits = r.u()?;
                let a = value(r)?;
                let b = if bits & PROP_ACCESSOR != 0 {
                    value(r)?
                } else {
                    V::Imm(Value::UNDEFINED)
                };
                out.push(P { key, bits, a, b });
            }
            F::Props(out)
        }
        Ty::Function => match value(r)? {
            V::Fn(i) => F::Fn(i),
            _ => return Err(Short),
        },
        Ty::Intrinsic => F::Intr(r.u32()?),
        Ty::HostFn => F::HostFn(r.u32()?),
        Ty::Buckets => F::Buckets(r.u()?),
        Ty::Program => F::None,
        Ty::Flags(_) => F::Flags(r.u()?),
        Ty::Enum(..) | Ty::Bits(..) => match r.u()? {
            0 => F::V(value(r)?),
            k => F::Const(k - 1),
        },
        Ty::Tail(elem) => {
            let n = r.u()? as usize;
            let mut out = Vec::with_capacity(n.min(r.buf.len()));
            for _ in 0..n {
                out.push(fields(r, elem)?);
            }
            F::Tail(out)
        }
    })
}

fn fields(r: &mut Reader<'_>, tys: &[(String, Ty)]) -> Result<Vec<F>, Short> {
    tys.iter().map(|(_, t)| field(r, t)).collect()
}

/// Whether a field holds the zero state of a fresh cell.
fn is_zero(f: &F) -> bool {
    match f {
        F::U(0) | F::I(0) | F::Flags(0) | F::None => true,
        F::F64(x) => x.to_bits() == 0,
        F::V(V::Imm(v)) => v.to_bits() == 0,
        F::Vs(v) => v.is_empty(),
        F::Tail(v) => v.is_empty(),
        _ => false,
    }
}

/// What a record became.
#[derive(Clone, Copy)]
enum Kind {
    Object(Slots),
    Generic(&'static schema::KindSpec),
    Patch,
}

struct Loader<'a> {
    vm: &'a mut Vm,
    snap: &'a Snap<'a>,
    base: &'a BaseInfo,
    /// A host's VM (D3720): its host values; `None` for a fresh VM.
    host: Option<&'a HostValues<'a>>,
    /// Per host entry: its value, and for a host function its id.
    host_vals: Vec<(Value, u32)>,
    loaded: Loaded,
    cells: Vec<u32>,
    kinds: Vec<Kind>,
    strs: Vec<u32>,
    bases: Vec<u32>,
    intrinsics: Vec<u16>,
    scripts: Vec<u32>,
    functions: Vec<u32>,
}

fn oom<E: core::fmt::Debug>(e: E) -> Refusal {
    refusal(format!("out of memory: {e:?}"))
}

pub(super) fn load(
    base: &BaseInfo,
    bytes: &[u8],
    sources: &[&[u8]],
    stats: Option<&mut SnapshotStats>,
) -> Result<(Vm, ()), Refusal> {
    let snap = Snap::parse(bytes)?;
    let image = base.image.as_deref().ok_or_else(|| {
        refusal(String::from(
            "no base image to make a fresh VM from (a host's base info)",
        ))
    })?;
    let mut vm = Vm::from_base(image, base.opts).map_err(|e| refusal(format!("{e:?}")))?;
    run(&mut vm, &snap, base, sources, None, stats)?;
    Ok((vm, ()))
}

/// [`Vm::load_snapshot`]: a host's run into the host's VM `vm` (D3720).
pub(super) fn load_into(
    vm: &mut Vm,
    base: &BaseInfo,
    bytes: &[u8],
    sources: &[&[u8]],
    host: &HostValues<'_>,
) -> Result<Loaded, Refusal> {
    let snap = Snap::parse(bytes)?;
    // The host's VM as it was made: nothing runs, waits or is queued.
    export::refuse_unless_idle(vm.heap())
        .map_err(|e| refusal(format!("the VM to load into is {e}")))?;
    if vm.heap().value_array_len(state::host_calls(vm.heap())) != 0 {
        return Err(refusal(String::from(
            "the VM to load into has open host calls: load into a fresh one",
        )));
    }
    run(vm, &snap, base, sources, Some(host), None)
}

fn run(
    vm: &mut Vm,
    snap: &Snap<'_>,
    base: &BaseInfo,
    sources: &[&[u8]],
    host: Option<&HostValues<'_>>,
    stats: Option<&mut SnapshotStats>,
) -> Result<Loaded, Refusal> {
    let mut l = Loader {
        vm,
        snap,
        base,
        host,
        host_vals: Vec::new(),
        loaded: Loaded::default(),
        cells: Vec::new(),
        kinds: Vec::new(),
        strs: alloc::vec![0; snap.strings.len()],
        bases: Vec::new(),
        intrinsics: Vec::new(),
        scripts: Vec::new(),
        functions: Vec::new(),
    };
    l.code(sources)?;
    l.names()?;
    l.allocate()?;
    for i in 0..snap.records.len() {
        l.fill(i).map_err(|e| {
            let name = &snap.types[snap.records[i].0 as usize].name;
            refusal(format!("record {i} ({name}): {}", e.0.join("; ")))
        })?;
    }
    // Before the rebuilds, which rely on the slots (D3702).
    for i in 0..snap.records.len() {
        if let Kind::Object(_) = l.kinds[i] {
            super::check::object(l.vm.heap(), crate::value::Obj(l.cells[i])).map_err(|e| {
                let name = &snap.types[snap.records[i].0 as usize].name;
                refusal(format!("record {i} ({name}): {e}"))
            })?;
        }
    }
    l.rebuild()?;
    l.root()?;
    l.vm.heap()
        .verify()
        .map_err(|e| refusal(format!("the imported heap does not verify: {e:?}")))?;
    if let Some(st) = stats {
        st.records = snap.records.len();
        st.strings = snap.strings.len();
        st.scripts = snap.scripts.len();
        st.functions = snap.functions.len();
        st.bases = snap.bases.len();
        for (t, _) in &snap.records {
            *st.types
                .entry(snap.types[*t as usize].name.clone())
                .or_default() += 1;
        }
    }
    l.loaded.calls = l.vm.pending_host_calls();
    Ok(l.loaded)
}

/// What loading `bytes` needs from the host ([`Needs`]).
pub fn needs(bytes: &[u8]) -> Result<Needs, Refusal> {
    let s = Snap::parse(bytes)?;
    let text = |i: u32| -> Result<String, Refusal> {
        let (two, d) = *s.strings.get(i as usize).ok_or_else(|| short(Short))?;
        Ok(string_of(two, d))
    };
    let mut out = Needs::default();
    for &(name, _, _, src) in &s.scripts {
        if src.is_err() {
            out.sources.push(text(name)?);
        }
    }
    for e in &s.host {
        match *e {
            HostEntry::Root(n) | HostEntry::Function(n) => out.host.push(text(n)?),
            HostEntry::Member(..) => {}
        }
    }
    Ok(out)
}

fn string_of(two: bool, d: &[u8]) -> String {
    if two {
        let units: Vec<u16> = d
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        d.iter().map(|&b| b as char).collect()
    }
}

impl Loader<'_> {
    fn text(&self, i: u32) -> Result<String, Refusal> {
        let (two, d) = *self
            .snap
            .strings
            .get(i as usize)
            .ok_or_else(|| short(Short))?;
        Ok(string_of(two, d))
    }

    /// Compiles the scripts again and finds the functions, checking their bytecode.
    fn code(&mut self, sources: &[&[u8]]) -> Result<(), Refusal> {
        let snap = self.snap;
        let mut spans: Vec<BTreeMap<(u32, u32), Vec<u32>>> = Vec::new();
        for (i, &(name, params, bits, src)) in snap.scripts.iter().enumerate() {
            let name = self.text(name)?;
            let src: &[u8] = match src {
                Ok(s) => s,
                Err(hash) => sources
                    .iter()
                    .copied()
                    .find(|s| super::source_hash(s) == hash)
                    .ok_or_else(|| {
                        refusal(format!(
                            "script {i} ({name}): the host has no source of its hash"
                        ))
                    })?,
            };
            let params = self.text(params)?;
            let ps: Vec<&str> = if params.is_empty() {
                Vec::new()
            } else {
                params.split(',').collect()
            };
            let opts = crate::compiler::CompileOptions::from_bits(&name, &ps, bits);
            // Eagerly, also in a VM that compiles lazily (D2990): a function the exporter's
            // lazy parent made at its first call is a template of the script only then.
            let root = crate::compiler::compile(self.vm.heap_mut(), src, &opts).map_err(|e| {
                refusal(format!(
                    "script {i} ({name}) does not compile: {}",
                    e.message
                ))
            })?;
            let h = self.vm.heap();
            let script = template::debug_info(h, root.0).script.get().unwrap_or(0);
            self.scripts.push(script);
            let mut map: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
            let mut stack = alloc::vec![root.0];
            while let Some(t) = stack.pop() {
                let d = template::debug_info(h, t);
                map.entry((d.src_start, d.src_end)).or_default().push(t);
                for c in 0..template::child_count(h, t) {
                    if let Some(c) = template::child(h, t, c).and_then(|r| r.get()) {
                        stack.push(c);
                    }
                }
            }
            spans.push(map);
        }
        for &(script, start, end, pinned, hash, _) in &snap.functions {
            let ts = spans
                .get(script as usize)
                .and_then(|m| m.get(&(start, end)))
                .ok_or_else(|| {
                    refusal(format!(
                        "no function at {start}..{end} of script {script} in this build"
                    ))
                })?;
            let h = self.vm.heap();
            // Two templates of one span (a class's field initializers): the hash decides.
            let of = |t: u32| {
                if pinned {
                    super::hash::template_hash(h, t)
                } else {
                    super::hash::interface_hash(h, t)
                }
            };
            match ts.iter().copied().find(|&t| of(t) == hash) {
                Some(t) => self.functions.push(t),
                None if pinned => {
                    return Err(refusal(format!(
                        "the bytecode of the function at {start}..{end} of script {script}, which has a stored frame, differs in this build (hash {hash:016x}, this build {:016x})",
                        of(ts[0])
                    )));
                }
                None => {
                    return Err(refusal(format!(
                        "the closures of the function at {start}..{end} of script {script} capture other bindings in this build"
                    )));
                }
            }
        }
        Ok(())
    }

    fn names(&mut self) -> Result<(), Refusal> {
        for &p in &self.snap.bases {
            let path = self.text(p)?;
            let c = self
                .base
                .cells
                .get(&path)
                .copied()
                .ok_or_else(|| refusal(format!("no builtin {path} in this build")))?;
            self.bases.push(c);
        }
        for &n in &self.snap.intrinsics {
            let name = self.text(n)?;
            let id = crate::bytecode::intrinsics::names()
                .position(|x| x == name)
                .ok_or_else(|| refusal(format!("no intrinsic {name} in this build")))?;
            self.intrinsics.push(id as u16);
        }
        self.host_values()
    }

    /// The host entries as the host's values (D3720).
    fn host_values(&mut self) -> Result<(), Refusal> {
        let snap = self.snap;
        if snap.host.is_empty() {
            return Ok(());
        }
        let Some(host) = self.host else {
            return Err(refusal(String::from(
                "a snapshot of a host's run: load it into the host's VM",
            )));
        };
        for k in 0..snap.host.len() {
            let v = match snap.host[k] {
                HostEntry::Root(n) => {
                    let name = self.text(n)?;
                    let v = host
                        .values
                        .iter()
                        .find(|(m, _)| *m == name)
                        .map(|(_, v)| *v)
                        .or_else(|| {
                            host.functions
                                .iter()
                                .find(|(m, ..)| *m == name)
                                .map(|(.., v)| *v)
                        })
                        .ok_or_else(|| refusal(format!("this host has no host value {name}")))?;
                    (v, 0)
                }
                HostEntry::Member(p, key) => {
                    let key = self.text(key)?;
                    let parent = self.host_vals[p as usize].0;
                    let h = self.vm.heap_mut();
                    let k = h.atom_str(&key).map_err(oom)?;
                    let v = parent
                        .as_object()
                        .and_then(|o| match h.get_own_property(o, k) {
                            Some(Property::Data { value, .. }) => value.as_object(),
                            _ => None,
                        })
                        .ok_or_else(|| {
                            refusal(format!(
                                "the host value {} has no object member {key} in this host",
                                self.host_path(p)
                            ))
                        })?;
                    (Value::object(v), 0)
                }
                HostEntry::Function(n) => {
                    let name = self.text(n)?;
                    let (_, id, v) = host
                        .functions
                        .iter()
                        .find(|(m, ..)| *m == name)
                        .ok_or_else(|| refusal(format!("this host has no host function {name}")))?;
                    (*v, *id)
                }
            };
            self.host_vals.push(v);
        }
        Ok(())
    }

    /// Host entry `k` as text, for refusals: `ALL_TOOLS.0.schema`.
    fn host_path(&self, k: u32) -> String {
        match self.snap.host.get(k as usize) {
            Some(HostEntry::Root(n) | HostEntry::Function(n)) => self.text(*n).unwrap_or_default(),
            Some(HostEntry::Member(p, key)) => {
                format!(
                    "{}.{}",
                    self.host_path(*p),
                    self.text(*key).unwrap_or_default()
                )
            }
            None => String::new(),
        }
    }

    fn allocate(&mut self) -> Result<(), Refusal> {
        let snap = self.snap;
        self.cells.reserve(snap.records.len());
        for (i, (t, fs)) in snap.records.iter().enumerate() {
            let ty = &snap.types[*t as usize];
            let (cell, kind) = if let Some(c) = ty.name.strip_prefix("Object.") {
                let (c, record) = match c.split_once('.') {
                    Some((c, r)) => (c, Some(r)),
                    None => (c, None),
                };
                let class = schema::class_by_name(c)
                    .ok_or_else(|| refusal(format!("no object class {c} in this build")))?;
                // A record's slots are its kind's schema's (D3710).
                let spec = match (record, schema::is_kinded(class.class)) {
                    (None, false) => Slots::class(class),
                    (Some(r), true) => {
                        Slots::record(schema::record_by_name(class.class, r).ok_or_else(|| {
                            refusal(format!("no {c} record kind {r} in this build"))
                        })?)
                    }
                    _ => return Err(refusal(format!("{}: not a record type", ty.name))),
                };
                let mut slots = spec.slots.len() as u32;
                let mut template = 0;
                for ((n, _), f) in ty.fields.iter().zip(fs) {
                    if Some(n.as_str()) == spec.rest
                        && let F::Vs(v) = f
                    {
                        slots += v.len() as u32;
                    }
                    if n == "template"
                        && let F::Fn(i) = f
                    {
                        template = *self
                            .functions
                            .get(*i as usize)
                            .ok_or_else(|| short(Short))?;
                    }
                }
                // Slots the snapshot's class has and this build's lacks: refused below
                // when the fields are matched.
                let h = self.vm.heap_mut();
                let o = if spec.class == ObjectClass::Function {
                    h.alloc_closure(None, template, slots).map_err(oom)?
                } else if let Some(r) = spec.record {
                    let kind = schema::record_by_name(spec.class, r).map_or(0, |r| r.kind);
                    let rest = slots - spec.slots.len() as u32;
                    h.alloc_record(kind, None, rest).map_err(oom)?
                } else {
                    h.alloc_object(spec.class, None, slots).map_err(oom)?
                };
                (o.offset(), Kind::Object(spec))
            } else if let Some(p) = ty.name.strip_prefix("Patch.") {
                let h = self.vm.heap();
                let (o, which) = match p {
                    "Global" => (intrinsic(h, Intrinsic::Global).as_object(), 0),
                    "GlobalLex" => (Some(realm::global_lex(h)), 1),
                    "SymbolRegistry" => (intrinsic(h, Intrinsic::SymbolRegistry).as_object(), 2),
                    _ => return Err(refusal(format!("unknown patch {p}"))),
                };
                (o.map_or(0, |o| o.offset()), {
                    let _ = which;
                    Kind::Patch
                })
            } else {
                let layout = crate::heap::cells::LAYOUTS
                    .iter()
                    .find(|l| l.name == ty.name)
                    .ok_or_else(|| {
                        refusal(format!(
                            "record {i}: no cell kind {} in this build",
                            ty.name
                        ))
                    })?;
                let kind = CellKind::from_u8(layout.kind).unwrap();
                let spec = schema::kind_spec(kind).ok_or_else(|| {
                    refusal(format!("record {i}: {} is not a generic kind", ty.name))
                })?;
                let n = ty
                    .fields
                    .iter()
                    .zip(fs)
                    .find_map(|((_, t), f)| match (t, f) {
                        (Ty::Tail(_), F::Tail(v)) => Some(v.len() as u32),
                        _ => None,
                    })
                    .unwrap_or(0);
                let size = match &layout.tail {
                    Some(t) => crate::heap::align8(t.offset + n * t.elem.size).max(layout.min_size),
                    None => layout.min_size,
                };
                let c = self.vm.heap_mut().alloc_cell(kind, size).map_err(oom)?;
                if kind == CellKind::Symbol {
                    // Its atom now: properties keyed by it may be filled before it.
                    self.vm
                        .heap_mut()
                        .register_symbol_atom(Sym(c))
                        .map_err(oom)?;
                }
                (c, Kind::Generic(spec))
            };
            self.cells.push(cell);
            self.kinds.push(kind);
        }
        Ok(())
    }

    fn string(&mut self, i: u32) -> Result<Str, Refusal> {
        let s = *self.strs.get(i as usize).ok_or_else(|| short(Short))?;
        if s != 0 {
            return Ok(Str(s));
        }
        let (two, d) = self.snap.strings[i as usize];
        let h = self.vm.heap_mut();
        let s = if two {
            let units: Vec<u16> = d
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            h.alloc_utf16(&units).map_err(oom)?
        } else {
            h.alloc_latin1(d).map_err(oom)?
        };
        self.strs[i as usize] = s.offset();
        Ok(s)
    }

    fn val(&mut self, v: V) -> Result<Value, Refusal> {
        let cell_value = |h: &Heap, off: u32, internal: bool| {
            if internal {
                return Value::internal(off);
            }
            match natural_tag(h.cell_kind(off)) {
                tag::OBJECT => Value::object(Obj(off)),
                tag::SYMBOL => Value::symbol(Sym(off)),
                tag::BIGINT => Value::bigint(off),
                tag::STRING => Value::string(Str(off)),
                _ => Value::internal(off),
            }
        };
        Ok(match v {
            V::Imm(v) => v,
            V::Str(i) => Value::string(self.string(i)?),
            V::Rec(i, internal) => {
                let off = *self.cells.get(i as usize).ok_or_else(|| short(Short))?;
                if off == 0 {
                    return Err(refusal(format!("record {i} has no cell in this VM")));
                }
                cell_value(self.vm.heap(), off, internal)
            }
            V::Base(i, internal) => {
                let off = *self.bases.get(i as usize).ok_or_else(|| short(Short))?;
                cell_value(self.vm.heap(), off, internal)
            }
            V::Fn(i) => {
                Value::internal(*self.functions.get(i as usize).ok_or_else(|| short(Short))?)
            }
            V::Script(i) => {
                Value::internal(*self.scripts.get(i as usize).ok_or_else(|| short(Short))?)
            }
            V::Host(i) => {
                self.host_vals
                    .get(i as usize)
                    .ok_or_else(|| short(Short))?
                    .0
            }
        })
    }

    fn key(&mut self, v: V) -> Result<Atom, Refusal> {
        let k = self.val(v)?;
        let h = self.vm.heap_mut();
        if let Some(s) = k.as_string() {
            return h.intern(s).map_err(oom);
        }
        if let Some(s) = k.as_symbol() {
            return Ok(h.symbol_atom(s));
        }
        Err(refusal(String::from(
            "a property key that is neither a string nor a symbol",
        )))
    }

    fn prop(&mut self, p: &P) -> Result<(Atom, Property), Refusal> {
        let key = self.key(p.key)?;
        let flags = PropFlags::from_bits(p.bits as u8);
        let a = self.val(p.a)?;
        Ok((
            key,
            if p.bits & PROP_ACCESSOR != 0 {
                let b = self.val(p.b)?;
                Property::Accessor {
                    getter: a,
                    setter: b,
                    flags,
                }
            } else {
                Property::Data { value: a, flags }
            },
        ))
    }

    fn flags_of(&self, names: &[String], bits: u64, table: &[(&str, u16)]) -> Result<u16, Refusal> {
        let mut out = 0;
        for (i, n) in names.iter().enumerate() {
            if bits & (1 << i) == 0 {
                continue;
            }
            let b = table
                .iter()
                .find(|(t, _)| t == n)
                .ok_or_else(|| refusal(format!("no flag {n} in this build")))?;
            out |= b.1;
        }
        Ok(out)
    }

    fn fill(&mut self, i: usize) -> Result<(), Refusal> {
        let snap = self.snap;
        let (t, fs) = &snap.records[i];
        let ty = &snap.types[*t as usize];
        let off = self.cells[i];
        match self.kinds[i] {
            Kind::Object(spec) => self.fill_object(off, spec, ty, fs),
            Kind::Patch => self.fill_patch(off, ty, fs),
            Kind::Generic(spec) => self.fill_generic(off, spec, ty, fs),
        }
    }

    fn set_elements(&mut self, o: Obj, vs: &[V]) -> Result<(), Refusal> {
        #[cfg(test)]
        if BROKEN.with(|b| b.get()) {
            // The negative control (tests.rs): an importer that loses elements.
            return Ok(());
        }
        if vs.is_empty() {
            return Ok(());
        }
        let mut vals = Vec::with_capacity(vs.len());
        for v in vs {
            vals.push(self.val(*v)?);
        }
        let h = self.vm.heap_mut();
        let arr = h.alloc_value_array(vals.len() as u32).map_err(oom)?;
        for (k, v) in vals.into_iter().enumerate() {
            h.value_array_set(arr, k as u32, v);
        }
        h.arena_mut().set(arr, ValueArray::LEN, vs.len() as u32);
        h.arena_mut().set(o.offset(), Object::ELEMENTS, Ref(arr));
        Ok(())
    }

    fn set_flags(&mut self, o: Obj, names: &[String], bits: u64) -> Result<(), Refusal> {
        let want = self.flags_of(names, bits, schema::OBJECT_FLAGS)?;
        let h = self.vm.heap_mut();
        for (_, b) in schema::OBJECT_FLAGS {
            h.snap_set_obj_flag(o, *b, want & b != 0);
        }
        Ok(())
    }

    fn fill_object(
        &mut self,
        off: u32,
        spec: Slots,
        ty: &TypeInfo,
        fs: &[F],
    ) -> Result<(), Refusal> {
        let o = Obj(off);
        let mut slot_vals: Vec<(u32, Value)> = Vec::new();
        // A record has every field of its schema (D3710): a field the snapshot lacks
        // refuses it.
        let mut seen = 0u64;
        let mut header = None;
        let mut buckets = None;
        for ((name, t), f) in ty.fields.iter().zip(fs) {
            match (name.as_str(), f) {
                ("proto", F::V(v)) => {
                    let p = self.val(*v)?;
                    let r = p.as_object().map_or(0, |p| p.offset());
                    self.vm
                        .heap_mut()
                        .arena_mut()
                        .set(off, Object::PROTO, Ref(r));
                }
                ("header", F::Flags(bits)) => {
                    // Set after the properties: a lazy `name` is read from the template
                    // when a key is looked up, and the checks come later.
                    let Ty::Flags(names) = t else { unreachable!() };
                    header = Some((names, *bits));
                }
                ("props", F::Props(ps)) => {
                    for p in ps {
                        let (k, p) = self.prop(p)?;
                        let h = self.vm.heap_mut();
                        if h.get_own_property(o, k).is_some() {
                            return Err(refusal(format!("{}: a key twice", ty.name)));
                        }
                        h.snap_prop_insert(o, k, &p).map_err(oom)?;
                    }
                }
                ("elements", F::Vs(vs)) => self.set_elements(o, vs)?,
                ("template", F::Fn(_)) => {}
                (n, f) => {
                    let fixed = spec.slots.iter().position(|(s, _)| *s == n);
                    if let Some(k) = fixed {
                        seen |= 1 << k.min(63);
                        constants_agree(&ty.name, n, t, spec.slots[k].1)?;
                    }
                    match (fixed, f) {
                        (Some(k), F::Const(i)) => {
                            let v = constant(&ty.name, n, t, spec.slots[k].1, *i)?;
                            slot_vals.push((k as u32, Value::int(v)));
                        }
                        (Some(k), F::V(v)) => {
                            let v = self.val(*v)?;
                            if v.as_int().is_some()
                                && matches!(spec.slots[k].1, SlotTy::Enum(_) | SlotTy::Bits(_))
                            {
                                return Err(refusal(format!(
                                    "{}.{n}: a number that is no named constant",
                                    ty.name
                                )));
                            }
                            slot_vals.push((k as u32, v));
                        }
                        (Some(k), F::Intr(i)) => {
                            let id = *self
                                .intrinsics
                                .get(*i as usize)
                                .ok_or_else(|| short(Short))?;
                            slot_vals.push((k as u32, Value::int(i32::from(id))));
                        }
                        (Some(k), F::Buckets(b)) => buckets = Some((k as u32, *b as u32)),
                        (Some(_), F::None) => {}
                        (None, F::Vs(vs)) if Some(n) == spec.rest => {
                            let base = spec.slots.len() as u32;
                            for (j, v) in vs.iter().enumerate() {
                                let v = self.val(*v)?;
                                slot_vals.push((base + j as u32, v));
                            }
                        }
                        _ => {
                            return Err(refusal(format!(
                                "{}: no slot {n} in this build's {}",
                                ty.name,
                                spec.what()
                            )));
                        }
                    }
                }
            }
        }
        if spec.record.is_some()
            && let Some((n, _)) = spec
                .slots
                .iter()
                .enumerate()
                .find(|(k, _)| seen & (1 << (*k).min(63)) == 0)
                .map(|(_, f)| f)
        {
            return Err(refusal(format!(
                "{}: the snapshot has no field {n} of this build's {}",
                ty.name,
                spec.what()
            )));
        }
        if let Some((names, bits)) = header {
            self.set_flags(o, names, bits)?;
        }
        let h = self.vm.heap_mut();
        for (k, v) in slot_vals {
            h.set_object_slot(o, k, v);
        }
        if let Some((k, b)) = buckets {
            // A power of two with room for every entry (the rehash probes for a free
            // bucket): the snapshot's count, if it is one.
            let used = match h.object_slot(o, 0).as_internal() {
                Some(e)
                    if h.cell_kind(e) == CellKind::ValueArray
                        || h.cell_kind(e) == CellKind::EphemeronArray =>
                {
                    h.value_array_len(e)
                }
                _ => 0,
            };
            let need = (2 * (used / 2 + 1)).next_power_of_two().max(8);
            let b = if b.is_power_of_two() && b >= need {
                b
            } else {
                need
            };
            let ix = h.alloc_u32_array(b).map_err(oom)?;
            h.set_object_slot(o, k, Value::internal(ix));
        }
        Ok(())
    }

    fn fill_patch(&mut self, off: u32, ty: &TypeInfo, fs: &[F]) -> Result<(), Refusal> {
        if off == 0 {
            return Ok(());
        }
        let o = Obj(off);
        for ((name, t), f) in ty.fields.iter().zip(fs) {
            match (name.as_str(), f) {
                ("proto", F::V(v)) => {
                    let p = self.val(*v)?;
                    let r = p.as_object().map_or(0, |p| p.offset());
                    self.vm
                        .heap_mut()
                        .arena_mut()
                        .set(off, Object::PROTO, Ref(r));
                }
                ("header", F::Flags(bits)) => {
                    let Ty::Flags(names) = t else { unreachable!() };
                    // A per-VM object can only have been made non-extensible.
                    let want = self.flags_of(names, *bits, schema::OBJECT_FLAGS)?;
                    if want & !crate::heap::cells::flags::OBJ_EXTENSIBLE != 0 {
                        return Err(refusal(format!("{}: header flags {want:#x}", ty.name)));
                    }
                    self.vm.heap_mut().snap_set_obj_flag(
                        o,
                        crate::heap::cells::flags::OBJ_EXTENSIBLE,
                        want != 0,
                    );
                }
                ("deleted", F::Vs(keys)) => {
                    for k in keys {
                        let k = self.key(*k)?;
                        let h = self.vm.heap_mut();
                        if h.get_own_property(o, k).is_some()
                            && !h.delete_own_property(o, k).map_err(oom)?
                        {
                            return Err(refusal(format!("{}: cannot delete a key", ty.name)));
                        }
                    }
                }
                ("props", F::Props(ps)) => {
                    for p in ps {
                        let (k, p) = self.prop(p)?;
                        let h = self.vm.heap_mut();
                        if h.get_own_property(o, k).is_some() {
                            let d = match p {
                                Property::Data { value, flags } => {
                                    PropertyDescriptor::data(value, flags)
                                }
                                Property::Accessor {
                                    getter,
                                    setter,
                                    flags,
                                } => PropertyDescriptor::accessor(getter, setter, flags),
                            };
                            if !h.define_own_property(o, k, &d).map_err(oom)? {
                                return Err(refusal(format!(
                                    "{}: cannot redefine {}",
                                    ty.name,
                                    h.atom_name(k)
                                )));
                            }
                        } else {
                            h.snap_prop_insert(o, k, &p).map_err(oom)?;
                        }
                    }
                }
                ("elements", F::Vs(vs)) => {
                    if !vs.is_empty() {
                        return Err(refusal(format!("{}: elements", ty.name)));
                    }
                }
                (n, _) => return Err(refusal(format!("{}: unknown field {n}", ty.name))),
            }
        }
        Ok(())
    }

    fn write_field(&mut self, at: u32, fi: &FieldInfo, t: &Ty, f: &F) -> Result<(), Refusal> {
        let ok = matches!(
            (fi.ty, t),
            (FieldTy::Value, Ty::Value)
                | (FieldTy::Ref, Ty::Ref)
                | (FieldTy::I32, Ty::Int)
                | (FieldTy::F64, Ty::F64)
                | (
                    FieldTy::U8 | FieldTy::U16 | FieldTy::U32 | FieldTy::U64,
                    Ty::Uint
                )
        );
        if !ok {
            return Err(refusal(format!(
                "field {} is a {:?} in this build, the snapshot has {:?}",
                fi.name, fi.ty, t
            )));
        }
        match f {
            F::V(v) => {
                let v = self.val(*v)?;
                let a = self.vm.heap_mut().arena_mut();
                match fi.ty {
                    FieldTy::Value => a.write(at, v),
                    _ => a.write(at, Ref(v.ref_offset().unwrap_or(0))),
                }
            }
            F::U(u) => {
                let a = self.vm.heap_mut().arena_mut();
                match fi.ty {
                    FieldTy::U8 => a.write(at, *u as u8),
                    FieldTy::U16 => a.write(at, *u as u16),
                    FieldTy::U32 => a.write(at, *u as u32),
                    _ => a.write(at, *u),
                }
            }
            F::I(i) => self.vm.heap_mut().arena_mut().write(at, *i as i32),
            F::F64(x) => self.vm.heap_mut().arena_mut().write(at, *x),
            _ => return Err(short(Short)),
        }
        Ok(())
    }

    fn fill_generic(
        &mut self,
        off: u32,
        spec: &'static schema::KindSpec,
        ty: &TypeInfo,
        fs: &[F],
    ) -> Result<(), Refusal> {
        let layout: &CellLayout = spec.kind.layout();
        for ((name, t), f) in ty.fields.iter().zip(fs) {
            // What the importer sets itself: a symbol's atom, a tail's count.
            if spec.skip.contains(&name.as_str()) || spec.count == Some(name.as_str()) {
                return Err(refusal(format!("{}: {name} is not state", ty.name)));
            }
            if name == "header"
                && let (Ty::Flags(names), F::Flags(bits)) = (t, f)
            {
                let fl = self.flags_of(names, *bits, spec.flags)?;
                self.vm.heap_mut().set_cell_flags(off, fl);
                continue;
            }
            if let (Ty::Tail(elem), F::Tail(items)) = (t, f) {
                let tail = layout
                    .tail
                    .as_ref()
                    .filter(|tl| tl.name == name)
                    .ok_or_else(|| refusal(format!("{}: no tail {name} in this build", ty.name)))?;
                if let Some(c) = spec.count {
                    let cf = layout.fields.iter().find(|f| f.name == c).unwrap();
                    self.write_field(off + cf.offset, cf, &Ty::Uint, &F::U(items.len() as u64))?;
                }
                for (i, item) in items.iter().enumerate() {
                    let at = off + tail.offset + i as u32 * tail.elem.size;
                    for ((en, et), ef) in elem.iter().zip(item) {
                        let Some(fi) = tail.elem.fields.iter().find(|x| x.name == en) else {
                            if is_zero(ef) {
                                continue;
                            }
                            return Err(refusal(format!(
                                "{}: no field {name}.{en} in this build (and the snapshot's is not 0)",
                                ty.name
                            )));
                        };
                        self.write_field(at + fi.offset, fi, et, ef)?;
                    }
                }
                continue;
            }
            if let F::HostFn(e) = f {
                let fi = layout
                    .fields
                    .iter()
                    .find(|x| x.name == name && is_host_call_fn(spec, x))
                    .ok_or_else(|| refusal(format!("{}.{name}: not a host function", ty.name)))?;
                let id = match self.snap.host.get(*e as usize) {
                    Some(HostEntry::Function(_)) => self.host_vals[*e as usize].1,
                    _ => return Err(short(Short)),
                };
                self.write_field(off + fi.offset, fi, &Ty::Uint, &F::U(u64::from(id)))?;
                continue;
            }
            let Some(fi) = layout.fields.iter().find(|x| x.name == name) else {
                // A field this build does not have: fine while it holds the zero state
                // (what a new cell starts with), else its meaning would be lost.
                if is_zero(f) {
                    continue;
                }
                return Err(refusal(format!(
                    "{}: no field {name} in this build (and the snapshot's is not 0)",
                    ty.name
                )));
            };
            self.write_field(off + fi.offset, fi, t, f)?;
        }
        Ok(())
    }

    /// The indexes and programs that were not stored.
    fn rebuild(&mut self) -> Result<(), Refusal> {
        for i in 0..self.cells.len() {
            let Kind::Object(spec) = self.kinds[i] else {
                continue;
            };
            let o = Obj(self.cells[i]);
            if spec.slots.iter().any(|(_, t)| matches!(t, SlotTy::Buckets)) {
                self.vm.heap_mut().collection_rehash(o);
            }
            if spec.class == ObjectClass::RegExp {
                let h = self.vm.heap_mut();
                let src = h
                    .object_slot(o, 1)
                    .as_string()
                    .ok_or_else(|| short(Short))?;
                let units = h.str_to_utf16(src);
                let flags = crate::regexp::Flags::from_bits(
                    h.object_slot(o, 2).as_int().unwrap_or(0) as u16,
                );
                let prog = crate::regexp::compile_within(&units, flags, true, h.unicode_props())
                    .map_err(|e| refusal(format!("a RegExp does not compile: {}", e.message())))?;
                let c = crate::builtins::regexp::program_cell(h, &prog).map_err(oom)?;
                h.set_object_slot(o, 0, Value::internal(c));
            }
        }
        for (k, (.., sites)) in self.snap.functions.iter().enumerate() {
            let t = self.functions[k];
            for &(i, v) in sites {
                let v = self.val(v)?;
                let h = self.vm.heap_mut();
                // The site must be a tagged template of this build's code too.
                let here = crate::bytecode::decode::insns(template::code(h, t))
                    .flatten()
                    .any(|n| n.op == crate::bytecode::Op::GetTemplateObject && n.imm(0) == i);
                let consts: Ref = h.arena().get(t, crate::heap::cells::FuncTemplate::CONSTS);
                let Some(c) = consts.get().filter(|_| here) else {
                    return Err(refusal(format!(
                        "function {k}: no tagged template at constant {i} in this build"
                    )));
                };
                h.value_array_set(c, i, v);
            }
        }
        Ok(())
    }

    /// The host call table: open async calls with distinct ids, their arguments and
    /// promises.
    fn check_host_calls(&self, r: Ref) -> Result<(), Refusal> {
        use crate::heap::cells::HostCall;
        let h = self.vm.heap();
        let bad = |w: &str| refusal(format!("the host call table: {w}"));
        let t = r
            .get()
            .filter(|&t| h.cell_kind(t) == CellKind::ValueArray)
            .ok_or_else(|| bad("not an array"))?;
        let mut ids = Vec::new();
        for i in 0..h.value_array_len(t) {
            let c = h
                .value_array_get(t, i)
                .as_internal()
                .filter(|&c| h.cell_kind(c) == CellKind::HostCall)
                .ok_or_else(|| bad("an entry is not a call"))?;
            let a = h.arena();
            let id: u32 = a.get(c, HostCall::ID);
            let args: Ref = a.get(c, HostCall::ARGS);
            let p: Value = a.get(c, HostCall::PROMISE);
            if id == 0 || ids.contains(&id) {
                return Err(bad("a call id is 0 or twice"));
            }
            ids.push(id);
            if !args
                .get()
                .is_some_and(|x| h.cell_kind(x) == CellKind::ValueArray)
            {
                return Err(bad("a call's arguments are not an array"));
            }
            if !p
                .as_object()
                .is_some_and(|o| h.object_class(o) == ObjectClass::Promise)
            {
                return Err(bad("an open call at an idle point without a promise"));
            }
        }
        Ok(())
    }

    /// The handle table: its free list and live count within its slots.
    fn check_handles(&self, r: Ref) -> Result<(), Refusal> {
        use crate::heap::cells::{HANDLE_LIVE, HandleSlot, HandleTable};
        let Some(t) = r.get() else {
            return Ok(());
        };
        let h = self.vm.heap();
        let bad = |w: &str| refusal(format!("the handle table: {w}"));
        if h.cell_kind(t) != CellKind::HandleTable {
            return Err(bad("not a handle table"));
        }
        let a = h.arena();
        let len: u32 = a.get(t, HandleTable::LEN);
        let (mut free, mut live) = (a.get::<u32>(t, HandleTable::FREE), 0);
        for i in 0..len {
            let next: u32 = a.get(HandleTable::SLOTS.at(t, i), HandleSlot::NEXT);
            if next == HANDLE_LIVE {
                live += 1;
            } else if next > len {
                return Err(bad("a free slot links out of the table"));
            }
        }
        if live != a.get::<u32>(t, HandleTable::LIVE) {
            return Err(bad("the live count is wrong"));
        }
        // The free list ends, within the table, on free slots.
        let mut steps = 0;
        while free != 0 {
            if free > len || steps > len {
                return Err(bad("the free list is broken"));
            }
            let next: u32 = a.get(HandleTable::SLOTS.at(t, free - 1), HandleSlot::NEXT);
            if next == HANDLE_LIVE {
                return Err(bad("the free list holds a live slot"));
            }
            free = next;
            steps += 1;
        }
        Ok(())
    }

    fn root(&mut self) -> Result<(), Refusal> {
        let snap = self.snap;
        let ty = &snap.types[snap.root.0 as usize];
        let i = state::interp(self.vm.heap());
        for ((name, _), f) in ty.fields.iter().zip(&snap.root.1) {
            match (name.as_str(), f) {
                ("nextCallId", F::U(u)) => {
                    self.vm
                        .heap_mut()
                        .arena_mut()
                        .set(i, Interp::NEXT_CALL_ID, *u as u32)
                }
                ("state" | "outcome", F::U(u)) => {
                    let s = match u {
                        1 => crate::interp::RunState::Done,
                        2 => crate::interp::RunState::Threw,
                        _ => crate::interp::RunState::Idle,
                    };
                    let f = if name == "state" {
                        Interp::STATE
                    } else {
                        Interp::OUTCOME
                    };
                    self.vm.heap_mut().arena_mut().set(i, f, s as u32)
                }
                ("result", F::V(v)) => {
                    let v = self.val(*v)?;
                    self.vm.heap_mut().arena_mut().set(i, Interp::RESULT, v)
                }
                ("throw", F::V(v)) => {
                    let v = self.val(*v)?;
                    self.vm.heap_mut().arena_mut().set(i, Interp::THROW, v)
                }
                ("rejections", F::V(v)) => {
                    let v = self.val(*v)?;
                    let r = Ref(v.ref_offset().unwrap_or(0));
                    self.vm.heap_mut().arena_mut().set(i, Interp::REJECTIONS, r)
                }
                ("reported", F::U(u)) => {
                    self.vm
                        .heap_mut()
                        .arena_mut()
                        .set(i, Interp::REPORTED, *u as u32)
                }
                ("hostCalls", F::V(v)) => {
                    let v = self.val(*v)?;
                    let r = Ref(v.ref_offset().unwrap_or(0));
                    self.check_host_calls(r)?;
                    self.vm.heap_mut().arena_mut().set(0, Roots::HOST_CALLS, r)
                }
                ("handles", F::V(v)) => {
                    if self.host.is_some() {
                        return Err(refusal(String::from(
                            "a snapshot of a VM, not of a host's run: load it into a fresh VM",
                        )));
                    }
                    let v = self.val(*v)?;
                    let r = Ref(v.ref_offset().unwrap_or(0));
                    self.check_handles(r)?;
                    self.vm.heap_mut().arena_mut().set(0, Roots::HANDLES, r)
                }
                ("named", F::Vs(vs)) => {
                    if self.host.is_none() {
                        return Err(refusal(String::from(
                            "a snapshot of a host's run: load it into the host's VM",
                        )));
                    }
                    for pair in vs.chunks(2) {
                        let [V::Str(n), v] = *pair else {
                            return Err(short(Short));
                        };
                        let name = self.text(n)?;
                        let v = self.val(v)?;
                        let h = self.vm.heap_mut().new_handle(v).map_err(oom)?;
                        self.loaded.handles.push((name, h));
                    }
                }
                ("hostData", F::V(V::Str(n))) => self.loaded.data = self.text(*n)?,
                ("frozenBuiltins" | "weakRefsStrong", F::U(u)) => {
                    let on = *u != 0;
                    let frozen = name == "frozenBuiltins";
                    let h = self.vm.heap_mut();
                    let now = if frozen {
                        h.frozen_builtins()
                    } else {
                        h.weak_refs_strong()
                    };
                    if self.host.is_some() && now != on {
                        return Err(refusal(format!(
                            "the snapshot's VM had {} {}, this one {}",
                            if frozen {
                                "frozen builtins"
                            } else {
                                "strong weak references"
                            },
                            if on { "on" } else { "off" },
                            if now { "on" } else { "off" },
                        )));
                    }
                    if frozen {
                        h.set_frozen_builtins(on);
                    } else {
                        h.set_weak_refs_strong(on);
                    }
                }
                ("global" | "globalLex" | "symbolRegistry", _) => {}
                ("random", F::Vs(vs)) => {
                    let mut vals = Vec::new();
                    for v in vs {
                        vals.push(self.val(*v)?);
                    }
                    let h = self.vm.heap_mut();
                    let rs = intrinsic(h, Intrinsic::RandomState).as_internal().unwrap();
                    if h.value_array_len(rs) != vals.len() as u32 {
                        return Err(refusal(String::from(
                            "the Math.random state differs in size",
                        )));
                    }
                    for (k, v) in vals.into_iter().enumerate() {
                        h.value_array_set(rs, k as u32, v);
                    }
                }
                (n, F::V(v)) => {
                    let which = match n {
                        "clock" => Intrinsic::Clock,
                        "timeZoneOffset" => Intrinsic::TimeZoneOffset,
                        "globalLexEpoch" => Intrinsic::GlobalLexEpoch,
                        "keptObjects" => Intrinsic::KeptObjects,
                        "registries" => Intrinsic::Registries,
                        "modules" => Intrinsic::Modules,
                        _ => return Err(refusal(format!("Vm: unknown field {n}"))),
                    };
                    let v = self.val(*v)?;
                    realm::set_intrinsic(self.vm.heap_mut(), which, v);
                }
                (n, _) => return Err(refusal(format!("Vm: unknown field {n}"))),
            }
        }
        Ok(())
    }
}

/// The bytes of the scripts' sources in a snapshot.
#[cfg(test)]
pub(super) fn sources_len(bytes: &[u8]) -> Option<usize> {
    let s = Snap::parse(bytes).ok()?;
    Some(s.scripts.iter().map(|x| x.3.map_or(0, |b| b.len())).sum())
}

/// A snapshot's sections and record types, for people.
pub fn describe(bytes: &[u8]) -> Result<String, Refusal> {
    let s = Snap::parse(bytes)?;
    let mut out = String::new();
    for w in s.marks.windows(2) {
        out += &format!("{:10} {:8} bytes\n", w[0].0, w[1].1 - w[0].1);
    }
    let strings: usize = s.strings.iter().map(|x| x.1.len()).sum();
    out += &format!(
        "{} strings ({strings} bytes of text), {} builtins, {} intrinsics, {} scripts, {} functions, {} records\n",
        s.strings.len(),
        s.bases.len(),
        s.intrinsics.len(),
        s.scripts.len(),
        s.functions.len(),
        s.records.len()
    );
    let mut names = Vec::new();
    for &b in &s.bases {
        let (two, d) = s.strings[b as usize];
        if !two {
            names.push(d.iter().map(|&c| c as char).collect::<String>());
        }
    }
    out += &format!("builtins: {}\n", names.join(" "));
    for (sc, a, b, pinned, _, sites) in &s.functions {
        out += &format!(
            "function {sc}:{a}..{b}{}{}\n",
            if *pinned { " (bytecode)" } else { "" },
            if sites.is_empty() {
                String::new()
            } else {
                format!(" ({} template objects)", sites.len())
            }
        );
    }
    let mut by: BTreeMap<&str, usize> = BTreeMap::new();
    for (t, _) in &s.records {
        *by.entry(&s.types[*t as usize].name).or_default() += 1;
    }
    for (t, n) in by {
        out += &format!("  {n:6} {t}\n");
    }
    for t in &s.types {
        let f: Vec<String> = t
            .fields
            .iter()
            .map(|(n, t)| format!("{n}: {t:?}"))
            .collect();
        out += &format!("type {} {{ {} }}\n", t.name, f.join(", "));
    }
    Ok(out)
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
std::thread_local! {
    /// The negative control: the gate must see a broken importer (tests.rs).
    pub(super) static BROKEN: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

/// The field types of a named-constant field agree: the snapshot's and this build's are
/// the same set, or neither is one (D3710).
fn constants_agree(ty: &str, n: &str, t: &Ty, here: SlotTy) -> Result<(), Refusal> {
    let ok = match (t, here) {
        (Ty::Enum(s, _), SlotTy::Enum(c)) | (Ty::Bits(s, _), SlotTy::Bits(c)) => *s == c.name,
        (Ty::Enum(..) | Ty::Bits(..), _) | (_, SlotTy::Enum(_) | SlotTy::Bits(_)) => false,
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(refusal(format!(
            "{ty}.{n}: the snapshot's field holds other constants than this build's"
        )))
    }
}

/// This build's value of the named constant(s) `i` (a member index or members' bits) of
/// field `n`.
fn constant(ty: &str, n: &str, t: &Ty, here: SlotTy, i: u64) -> Result<i32, Refusal> {
    let missing =
        |m: &str, set: &str| refusal(format!("{ty}.{n}: no {set} member {m} in this build"));
    match (t, here) {
        (Ty::Enum(set, names), SlotTy::Enum(c)) => {
            let m = names.get(i as usize).ok_or_else(|| short(Short))?;
            c.members
                .iter()
                .find(|(k, _)| k == m)
                .map(|(_, v)| *v)
                .ok_or_else(|| missing(m, set))
        }
        (Ty::Bits(set, names), SlotTy::Bits(c)) => {
            if i >> names.len().min(63) != 0 {
                return Err(short(Short));
            }
            let mut v = 0;
            for (k, m) in names.iter().enumerate() {
                if i & (1 << k) != 0 {
                    v |= c
                        .members
                        .iter()
                        .find(|(x, _)| x == m)
                        .map(|(_, b)| *b)
                        .ok_or_else(|| missing(m, set))?;
                }
            }
            Ok(v)
        }
        _ => Err(short(Short)),
    }
}
