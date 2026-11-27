//! Logical snapshots (D3700): djs's persistence format. A waiting VM as a graph of
//! named-field records, keyed by bytecode hashes instead of the build (issues 0300, 0301;
//! docs/heap-snapshots-revisited.md).
//!
//! **When**: at an idle point only (no frames on the stack, no queued jobs, nothing in the
//! host inbox; open async host calls are fine), the moments a dump is taken.
//!
//! **What**: one record per reachable cell above the builtin prefix, in a self-describing
//! binary format:
//! - a header: the format version, the record types used with their fields' names and
//!   types and a digest of each, the string table, the builtins referred to by registry
//!   path, the intrinsics by name, the scripts (name, options, parameters, source), and the
//!   functions referred to as (script, source span, bytecode hash);
//! - the records, each a type id and its fields in the type's order;
//! - the `Vm` record: the interpreter's per-run state, the host call table and the handle
//!   table (both by the host's ids), the realm's per-VM values.
//!
//! Cells are written by kind name and named fields: the generic kinds (`ValueArray`,
//! `BoxCell`, `HostCall`, ...) straight from their layout in `heap/cells.rs`, objects by
//! class with their slots named in `schema.rs`, their properties as (key, attributes,
//! value) in storage order (shapes and property tables are rebuilt), Map/Set indexes and
//! RegExp programs rebuilt. Strings are content; atoms are their strings; symbols records.
//! Code is never stored: the importer compiles the scripts again and refuses a function
//! whose bytecode hash differs, so a frame's pc and registers can travel as they are.
//! The per-VM objects of a fresh VM (the global object, the global lexical record, the
//! `Symbol.for` registry) are patches over the importer's own.
//!
//! **Import** makes a fresh VM from the base, compiles the scripts, checks the hashes,
//! allocates every record, fills them in, rebuilds the indexes, and installs the root.

mod check;
mod export;
mod hash;
mod import;
mod schema;
#[cfg(test)]
mod tests;
mod wire;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::{Vm, VmOptions};
use crate::heap::{Property, Ref};
use crate::value::Value;
use wire::Writer;

pub use import::{SnapshotStats, describe};

/// The format version: bumped whenever the encoding changes.
pub const FORMAT: u8 = 1;
const MAGIC: &[u8; 6] = b"DJSNAP";

/// What a build knows about its base image, made once per base: the builtins' registry
/// paths, and the per-VM objects of a fresh VM (what patches are relative to).
pub struct BaseInfo {
    pub(crate) image: Vec<u8>,
    pub(crate) opts: VmOptions,
    /// Builtin cell offset -> registry path.
    pub(crate) paths: BTreeMap<u32, String>,
    /// Registry path -> builtin cell offset.
    pub(crate) cells: BTreeMap<String, u32>,
    /// The fresh VM's global object, global lexical record and `Symbol.for` registry: their
    /// properties in storage order (atom id, property).
    pub(crate) fresh: [Vec<(u32, Property)>; 3],
    pub(crate) fresh_global: Value,
}

impl BaseInfo {
    pub fn new(image: &[u8], opts: VmOptions) -> Result<BaseInfo, String> {
        let vm = Vm::from_base(image, opts).map_err(|e| alloc::format!("{e:?}"))?;
        let h = vm.heap();
        let end = h.base_end();
        let mut paths = BTreeMap::new();
        let mut cells = BTreeMap::new();
        for (c, p) in super::registry::reachable_names(h) {
            if c < end {
                cells.insert(p.clone(), c);
                paths.insert(c, p);
            }
        }
        use crate::interp::realm::{Intrinsic, global_lex, intrinsic};
        let objs = [
            intrinsic(h, Intrinsic::Global).as_object(),
            Some(global_lex(h)),
            intrinsic(h, Intrinsic::SymbolRegistry).as_object(),
        ];
        let fresh = objs.map(|o| match o {
            Some(o) => (0..h.snap_prop_slots(o))
                .filter_map(|i| h.snap_prop(o, i))
                .map(|(k, p)| (k.id(), p))
                .collect(),
            None => Vec::new(),
        });
        Ok(BaseInfo {
            image: image.to_vec(),
            opts,
            paths,
            cells,
            fresh,
            fresh_global: intrinsic(h, Intrinsic::Global),
        })
    }
}

/// How a snapshot is written.
#[derive(Clone, Copy, Debug)]
pub struct SnapshotOptions {
    /// Store the scripts' sources (else their hashes: the importer is given the sources,
    /// as a host that keeps its prelude and scripts can).
    pub sources: bool,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        SnapshotOptions { sources: true }
    }
}

/// The hash a snapshot names a source by when it does not store it.
pub fn source_hash(src: &[u8]) -> u64 {
    let mut f = wire::Fnv::new();
    f.bytes(src);
    f.0
}

/// Why a snapshot was not made or not loaded: the first few reasons, each with the path
/// to what could not be written or read.
#[derive(Debug, Clone)]
pub struct Refusal(pub Vec<String>);

impl Vm {
    /// A logical snapshot of this VM, which must be idle.
    pub fn snapshot(&self, base: &BaseInfo) -> Result<Vec<u8>, Refusal> {
        self.snapshot_with(base, &SnapshotOptions::default())
    }

    /// [`Vm::snapshot`] with options.
    pub fn snapshot_with(
        &self,
        base: &BaseInfo,
        opts: &SnapshotOptions,
    ) -> Result<Vec<u8>, Refusal> {
        let h = self.heap();
        export::refuse_unless_idle(h).map_err(|e| Refusal(alloc::vec![e]))?;
        let mut ex = export::Exporter::new(h, base);
        let mut root = Writer::default();
        ex.root(&mut root);
        // Every record first: a frame found late pins its function.
        ex.drain();
        let mut fns = Writer::default();
        let n = ex.functions.len();
        for k in 0..n {
            let t = ex.functions[k];
            ex.function_entry(&mut fns, t);
        }
        // The template objects are frozen arrays of strings.
        ex.drain();
        if ex.functions.len() != n {
            ex.refusals
                .push(String::from("a template object refers to a function"));
        }
        if !ex.refusals.is_empty() {
            ex.refusals.truncate(20);
            return Err(Refusal(ex.refusals));
        }
        Ok(container(&mut ex, &fns, &root, opts))
    }

    /// Loads a snapshot into a fresh VM made from `base`.
    pub fn from_snapshot(base: &BaseInfo, bytes: &[u8]) -> Result<Vm, Refusal> {
        import::load(base, bytes, &[], None).map(|(vm, _)| vm)
    }

    /// [`Vm::from_snapshot`] of a snapshot written without sources: `sources` are the
    /// candidates, found by their hashes.
    pub fn from_snapshot_with(
        base: &BaseInfo,
        bytes: &[u8],
        sources: &[&[u8]],
    ) -> Result<Vm, Refusal> {
        import::load(base, bytes, sources, None).map(|(vm, _)| vm)
    }

    /// [`Vm::from_snapshot`], counting what it did.
    pub fn from_snapshot_stats(
        base: &BaseInfo,
        bytes: &[u8],
    ) -> Result<(Vm, SnapshotStats), Refusal> {
        let mut st = SnapshotStats::default();
        import::load(base, bytes, &[], Some(&mut st)).map(|(vm, _)| (vm, st))
    }
}

fn container(
    ex: &mut export::Exporter<'_>,
    fns: &Writer,
    root: &Writer,
    opts: &SnapshotOptions,
) -> Vec<u8> {
    let h = ex.h();
    // Names first: they add strings.
    let scripts: Vec<(u32, u32, u32, Vec<u8>)> = ex
        .scripts
        .clone()
        .iter()
        .map(|&s| {
            use crate::heap::cells::Script;
            let a = h.arena();
            let name: Ref = a.get(s, Script::NAME);
            let params: Ref = a.get(s, Script::PARAMS);
            let text = |r: Ref| {
                r.get()
                    .map(|o| h.str_to_string(crate::value::Str(o)))
                    .unwrap_or_default()
            };
            let (n, p) = (text(name), text(params));
            let src = crate::bytecode::template::script_source(h, s)
                .unwrap_or_default()
                .to_vec();
            let bits = a.get::<u32>(s, Script::OPTIONS);
            (ex.str_lit(&n), ex.str_lit(&p), bits, src)
        })
        .collect();
    let bases: Vec<u32> = ex
        .bases
        .clone()
        .iter()
        .map(|o| {
            let p = ex.base_path(*o);
            ex.str_lit(&p)
        })
        .collect();
    // Builtin closure kinds by name. Their records' slots are positional and mean what
    // the builtins' code makes of them: not checked here (a hash of that code refused
    // snapshots at every compiler change that touched the builtins, 0300).
    let intrinsics: Vec<u32> = ex
        .intrinsics
        .clone()
        .iter()
        .map(|&i| ex.str_lit(crate::bytecode::intrinsics::intrinsic_name(i)))
        .collect();
    let mut w = Writer::default();
    w.buf.extend_from_slice(MAGIC);
    w.byte(FORMAT);
    let sets = export::flagsets();
    w.u(sets.len() as u64);
    for (n, f) in sets {
        w.str(n);
        w.u(f.len() as u64);
        for (name, _) in f {
            w.str(name);
        }
    }
    w.u(ex.types.len() as u64);
    for t in &ex.types {
        w.str(&t.name);
        w.u(t.fields.len() as u64);
        let mut d = wire::Fnv::new();
        d.str(&t.name);
        for (n, ty) in &t.fields {
            w.str(n);
            w.str(ty);
            d.str(n);
            d.str(ty);
        }
        w.u64_raw(d.0);
    }
    w.u(ex.strings.len() as u64);
    for (two, data) in &ex.strings {
        w.u((data.len() as u64) << 1 | u64::from(*two));
        w.buf.extend_from_slice(data);
    }
    w.u(bases.len() as u64);
    for b in bases {
        w.u(u64::from(b));
    }
    w.u(intrinsics.len() as u64);
    for i in intrinsics {
        w.u(u64::from(i));
    }
    w.u(scripts.len() as u64);
    for (n, p, bits, src) in &scripts {
        w.u(u64::from(*n));
        w.u(u64::from(*p));
        w.u(u64::from(*bits));
        if opts.sources {
            w.byte(0);
            w.bytes(src);
        } else {
            w.byte(1);
            w.u64_raw(source_hash(src));
        }
    }
    w.u(ex.functions.len() as u64);
    w.buf.extend_from_slice(&fns.buf);
    w.u(u64::from(ex.record_count));
    w.buf.extend_from_slice(&ex.records.buf);
    w.buf.extend_from_slice(&root.buf);
    w.buf
}
