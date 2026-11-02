//! The base image's content (0127): what a shipped image needs and a VM that compiled
//! the prelude still has.

use alloc::vec;

use crate::bytecode::decode::insns;
use crate::bytecode::imm::ImmKind;
use crate::bytecode::template::{debug_info, func_flags};
use crate::heap::cells::{Bytes, FuncTemplate, PropEntry, PropTable, Shape, Symbol, U32Array};
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
        if heap.arena().get(t, FuncTemplate::FLAGS) & func_flags::PRIVILEGED != 0 {
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
                if let Some(code) = a.get::<Ref>(at, FuncTemplate::CODE).get() {
                    let bytes = a.slice(Bytes::DATA.at(code, 0), a.get(code, Bytes::LEN));
                    for insn in insns(bytes) {
                        let insn = insn.expect("the prelude's bytecode decodes");
                        for (k, kind) in insn.info().imms.iter().enumerate() {
                            if *kind == ImmKind::Atom {
                                mark(insn.imm(k));
                            }
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
        &[("length", C), ("name", C), ("prototype", W)],
        &[("length", C), ("name", C), ("prototype", 0)],
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

/// Names the engine interns while it runs, whatever the script (D2067): the realm's
/// `Error` hooks read when an error is made, and the names every compilation interns.
const ENGINE_ATOMS: &[&str] = &[
    "prepareStackTrace",
    "stackTraceLimit",
    "undefined",
    "this",
    "arguments",
    "new.target",
    "<home>",
    "<active>",
    "<completion>",
];
