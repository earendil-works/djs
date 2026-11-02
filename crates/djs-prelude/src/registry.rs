//! The builtin registry of this build (D2371): stable ids for the builtin objects,
//! functions and symbols and for the intrinsics, keyed by spec path
//! (djs_core::vm::registry has the format and the paths). The committed table is
//! tests/builtins/registry.tsv; build.rs adds each id's cell in this build's base image.
//!
//! - id <-> path: [`path`], [`id_of_path`], [`id_of_intrinsic`];
//! - id <-> cell of the base image: [`base_offset`], [`id_of_base_offset`]. A VM's builtins
//!   sit at these offsets until a full collection moves them; for any heap,
//!   `registry().name_cells(heap)` resolves the paths on that heap instead.

use alloc::vec::Vec;

use djs_core::bytecode::intrinsics::INTRINSICS;
use djs_core::vm::registry::{Entry, EntryKind, Registry};

include!(concat!(env!("OUT_DIR"), "/registry_offsets.rs"));

/// The registry file, as committed.
pub static TEXT: &str = include_str!("../../../tests/builtins/registry.tsv");

/// The parsed registry. Empty if the file does not parse (the registry test reports it).
pub fn registry() -> Registry {
    Registry::parse(TEXT).unwrap_or_default()
}

/// The entries, parsed once per call: hold on to [`registry`] for repeated lookups.
fn entries() -> Vec<Entry> {
    registry().entries
}

/// The path (value entries) or name (intrinsic entries) of an id, live or tombstoned.
pub fn path(id: u32) -> Option<alloc::string::String> {
    entries().into_iter().nth(id as usize).map(|e| e.path)
}

/// The live id of a path or intrinsic name.
pub fn id_of_path(kind: EntryKind, path: &str) -> Option<u32> {
    registry().id_of(kind, path)
}

/// The registry id of an intrinsic of this build (an index into `INTRINSICS`).
pub fn id_of_intrinsic(intrinsic: u16) -> Option<u32> {
    let name = INTRINSICS.get(intrinsic as usize)?.name;
    registry().id_of(EntryKind::Intrinsic, name)
}

/// The cell of a value entry in this build's base image.
pub fn base_offset(id: u32) -> Option<u32> {
    BASE_OFFSETS
        .get(id as usize)
        .copied()
        .filter(|&o| o != u32::MAX)
}

/// The id of the builtin at a cell of this build's base image.
pub fn id_of_base_offset(offset: u32) -> Option<u32> {
    // Few lookups, mostly by tools: a scan of ~1,500 words.
    BASE_OFFSETS
        .iter()
        .position(|&o| o == offset)
        .map(|i| i as u32)
}
