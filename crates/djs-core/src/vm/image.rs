//! The base image's content (0127): what a shipped image needs and a VM that compiled
//! the prelude still has.

use alloc::vec;

use crate::bytecode::decode::insns;
use crate::bytecode::imm::ImmKind;
use crate::bytecode::template::{code, debug_info, func_flags};
use crate::heap::cells::{FuncTemplate, PropEntry, PropTable, Shape, Symbol, U32Array};
use crate::heap::{CellKind, Heap, HeapError, Ref};

/// Drops the prelude's internal names (D2253): the names of privileged functions
/// (`ArrayPrototypeMap`) and the atoms of its locals and parameters, which compiling the
/// prelude interned and nothing refers to. User code can't observe either: a builtin's
/// `name` is the installed property (D1433), and stack traces skip prelude frames
/// (D1980). The atoms that stay are the ones the heap refers to: property keys (property
/// tables, shape descriptors), symbols, function and local names of templates, the atom
/// immediates of the bytecode, and the global declarations' atom ids in constant pools
/// (int32 values: every int32 constant counts, a few false keeps at most).
///
/// The forgotten strings are garbage for the collection that follows.
pub(super) fn drop_internal_names(heap: &mut Heap) -> Result<u32, HeapError> {
    let templates: alloc::vec::Vec<u32> = heap
        .cells()
        .filter(|c| c.kind == CellKind::FuncTemplate)
        .map(|c| c.offset)
        .collect();
    for &t in &templates {
        let flags = heap.arena().get(t, FuncTemplate::FLAGS);
        if flags & (func_flags::PRIVILEGED | func_flags::LAZY_NAME) == func_flags::PRIVILEGED {
            heap.arena_mut().set(t, FuncTemplate::NAME, 0u32);
        }
    }
    // The names the engine interns at run time (D2067), so that using it interns none.
    let mut engine = vec![];
    for name in ENGINE_ATOMS {
        engine.push(heap.atom_str(name)?.id());
    }
    let mut keep = vec![false; heap.atom_count() as usize];
    let mut mark = |id: u32| {
        // String and symbol atoms (integer atoms have bit 31): their table index.
        if id & (1 << 31) == 0
            && let Some(k) = keep.get_mut((id & ((1 << 30) - 1)) as usize)
        {
            *k = true;
        }
    };
    for id in engine {
        mark(id);
    }
    let a = heap.arena();
    for c in heap.cells() {
        let at = c.offset;
        match c.kind {
            CellKind::Symbol => mark(a.get(at, Symbol::ATOM)),
            CellKind::PropTable => {
                for i in 0..a.get(at, PropTable::COUNT) {
                    mark(a.get(PropTable::ENTRIES.at(at, i), PropEntry::KEY));
                }
            }
            CellKind::Shape => {
                if let Some(d) = a.get::<Ref>(at, Shape::DESCS).get() {
                    for i in 0..a.read::<u32>(U32Array::DATA.at(d, 0)) {
                        mark(a.read(U32Array::DATA.at(d, 1 + 2 * i)));
                    }
                }
            }
            CellKind::FuncTemplate => {
                mark(a.get(at, FuncTemplate::NAME));
                if let Some(n) = debug_info(heap, at).names.get() {
                    for i in 0..a.get(n, U32Array::LEN) {
                        mark(a.read(U32Array::DATA.at(n, i)));
                    }
                }
                for insn in insns(code(heap, at)) {
                    let insn = insn.expect("the prelude's bytecode decodes");
                    for (k, kind) in insn.info().imms.iter().enumerate() {
                        if *kind == ImmKind::Atom {
                            mark(insn.imm(k));
                        }
                    }
                }
                if let Some(consts) = a.get::<Ref>(at, FuncTemplate::CONSTS).get() {
                    for i in 0..heap.value_array_len(consts) {
                        if let Some(n) = heap.value_array_get(consts, i).as_int() {
                            mark(n as u32);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    heap.forget_string_atoms(&keep)
}

/// D2445: the builtin closures whose `length` and `name` are the ones `%Install*`
/// defined get them lazily from their template (`Heap::make_builtin_props_lazy`), if no
/// other closure shares it. Before the base's shape tree is replaced: the rebuilt
/// shapes are the prelude's. Returns how many.
pub(super) fn lazy_builtin_props(heap: &mut Heap) -> Result<u32, HeapError> {
    use crate::heap::ObjectClass;
    use alloc::collections::BTreeMap;
    let mut closures: alloc::vec::Vec<(crate::value::Obj, u32)> = vec![];
    let mut per_template: BTreeMap<u32, u32> = BTreeMap::new();
    for c in heap.cells().filter(|c| c.kind == CellKind::Object) {
        let o = crate::value::Obj(c.offset);
        if heap.object_class(o) != ObjectClass::Function {
            continue;
        }
        if let Some(t) = heap.object_slot(o, 0).as_internal() {
            closures.push((o, t));
            *per_template.entry(t).or_default() += 1;
        }
    }
    let mut n = 0;
    for (o, t) in closures {
        if per_template[&t] == 1
            && heap.arena().get(t, FuncTemplate::FLAGS) & func_flags::PRIVILEGED != 0
            && heap.make_builtin_props_lazy(o)?
        {
            n += 1;
        }
    }
    Ok(n)
}

/// The shapes of the objects that builtins and the engine create with named properties,
/// as key chains with their stored flags (D2067). The base has them in every VM's shape
/// tree, so using a builtin adds no shape, whatever the order. From a survey of builtin
/// calls (0148): iterator results; RegExp match results and their `indices`; property
/// descriptors; functions, classes and their prototypes; RegExp objects; errors;
/// `Promise.allSettled`, `Promise.withResolvers` and `Proxy.revocable` results; template
/// objects.
pub(super) const BUILTIN_RESULT_SHAPES: &[&[(&str, u32)]] = {
    const W: u32 = 1;
    const ALL: u32 = 7;
    const WC: u32 = 5;
    const C: u32 = 4;
    &[
        &[("value", ALL), ("done", ALL)],
        &[
            ("index", ALL),
            ("input", ALL),
            ("groups", ALL),
            ("indices", ALL),
        ],
        &[("groups", ALL)],
        &[
            ("value", ALL),
            ("writable", ALL),
            ("enumerable", ALL),
            ("configurable", ALL),
        ],
        &[
            ("get", ALL),
            ("set", ALL),
            ("enumerable", ALL),
            ("configurable", ALL),
        ],
        // Builtin and bound closures; a closure's own `length` and `name` are lazy
        // (D2445), so its `prototype` is its first stored key.
        &[("length", C), ("name", C)],
        &[("prototype", W)],
        &[("prototype", 0)],
        &[("constructor", WC)],
        &[("lastIndex", W)],
        &[("message", WC), ("cause", WC)],
        &[("message", WC), ("errors", WC)],
        &[("message", WC), ("cause", WC), ("errors", WC)],
        &[("cause", WC)],
        &[("errors", WC)],
        &[("status", ALL), ("value", ALL)],
        &[("status", ALL), ("reason", ALL)],
        &[("promise", ALL), ("resolve", ALL), ("reject", ALL)],
        &[("proxy", ALL), ("revoke", ALL)],
        &[("raw", 0)],
    ]
};

/// Room for a VM's own atoms in the base image's tail (D2431): a session adding up to
/// this many writes them in place (a fresh REPL adds about 16).
pub(super) const SESSION_ATOMS: u32 = 32;

/// Names the engine interns while it runs, whatever the script (D2067): the realm's
/// `Error` hooks, read when an error is made. (The names every compilation interns are
/// interned by a session's first compilation, before anything else it does.)
pub(super) const ENGINE_ATOMS: &[&str] = &["prepareStackTrace", "stackTraceLimit"];
