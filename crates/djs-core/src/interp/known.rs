//! Known builtins (D3300): `Math.floor`, `String`, `Number.isInteger` without looking
//! them up.
//!
//! The base image's build picks the builtins with a fast path (`@fast`, `@rust`) that
//! scripts reach through a global: such a function itself (`String`, `parseInt`), or an
//! object with such functions among its own properties (`Math`) and those functions. Their
//! values follow the prelude's functions in the realm's prelude function table
//! (`Intrinsic::PreludeFunctions`, D2440), and `Intrinsic::KnownNames` names them: word 0
//! is the table index of the first, then one word per value in table order, a global's
//! name atom followed by its functions' with [`MEMBER`] set. Names and atom ids are the
//! same in every VM of a build, so what the compiler emits for a name depends on nothing a
//! script can change.
//!
//! For a reference to such a name the compiler emits `get_builtin k` (two for a method
//! call: the function, then the object as `this`), a jump over the ordinary lookup, and
//! the ordinary lookup. `get_builtin` is the guard: while the builtins are pristine the
//! table's length covers the known values. The first change to one of them shortens the
//! table to the prelude's functions ([`unpristine`]), for good; from then on the
//! instruction declines and the interpreter continues at the ordinary lookup, which it
//! also patches a jump to. "A change" is:
//! - a property operation that changes an object of the builtin prefix (the latch of
//!   D3290: any builtin, not only the known ones);
//! - a change of a known global's binding in the global object, which lives above the
//!   prefix (`STORE_WATCHED` on its entry, heap/object.rs), or a global lexical binding
//!   of its name.
//!
//! Pristine, the table's value is what the lookup would find, with no getter or proxy on
//! the way, so pushing it at the place of the lookup is not observable.

use super::realm::{Intrinsic, intrinsic};
use crate::heap::cells::ValueArray;
use crate::heap::{Atom, Heap};

/// `KnownNames`: a function of the global named before it.
const MEMBER: u32 = 1 << 31;

/// The table indexes of the known global `global`, and of its known function `member`
/// when that is given and known.
#[inline(never)]
pub(crate) fn lookup(
    heap: &Heap,
    global: Atom,
    member: Option<Atom>,
) -> Option<(u16, Option<u16>)> {
    // (A realm's atoms of the base come first: a script's own names end here.)
    if global.id() >= heap.atom_base_count() {
        return None;
    }
    let names = intrinsic(heap, Intrinsic::KnownNames).as_internal()?;
    let (n, first) = (heap.u32_array_len(names), heap.u32_array_get(names, 0));
    let (mut want, mut found, mut i) = (global.id(), None, 1);
    while i < n {
        let name = heap.u32_array_get(names, i);
        if name == want {
            let k = Some((first + i - 1) as u16);
            match (found, member) {
                (Some(g), _) => return Some((g, k)),
                (None, None) => return Some((k?, None)),
                (None, Some(m)) => (found, want) = (k, m.id() | MEMBER),
            }
        } else if found.is_some() && name & MEMBER == 0 {
            break;
        }
        i += 1;
    }
    Some((found?, None))
}

/// The builtins changed: `get_builtin` of a known builtin declines from now on.
pub(crate) fn unpristine(heap: &mut Heap) {
    if heap.realm().as_internal().is_none() {
        return;
    }
    let (Some(names), Some(t)) = (
        intrinsic(heap, Intrinsic::KnownNames).as_internal(),
        intrinsic(heap, Intrinsic::PreludeFunctions).as_internal(),
    ) else {
        return;
    };
    let first = heap.u32_array_get(names, 0);
    if heap.value_array_len(t) > first {
        heap.arena_mut().set(t, ValueArray::LEN, first);
    }
}

/// Whether the known builtins are still the base image's (false without them).
#[cfg(test)]
pub(crate) fn pristine(heap: &Heap) -> bool {
    let (Some(names), Some(t)) = (
        intrinsic(heap, Intrinsic::KnownNames).as_internal(),
        intrinsic(heap, Intrinsic::PreludeFunctions).as_internal(),
    ) else {
        return false;
    };
    heap.value_array_len(t) > heap.u32_array_get(names, 0)
}

/// Builds the table and the names (the base image's build, before its last collection).
/// The table belongs to the image's tail: a VM shortens it.
#[cfg(djs_prelude)]
pub(crate) fn build(heap: &mut Heap) -> Result<(), crate::heap::HeapError> {
    use alloc::vec::Vec;

    use super::realm::{self, intrinsic_obj};
    use crate::bytecode::template::func_flags;
    use crate::heap::cells::FuncTemplate;
    use crate::heap::{ObjectClass, Property};
    use crate::value::Value;

    fn fast(heap: &Heap, v: Value) -> bool {
        v.as_object().is_some_and(|o| {
            heap.object_class(o) == ObjectClass::Function && {
                let t = heap.closure_template(o);
                u32::from(heap.arena().get::<u16>(t, FuncTemplate::FLAGS)) & func_flags::FAST != 0
            }
        })
    }
    let data = |heap: &Heap, o, key| match heap.get_own_property(o, key) {
        Some(Property::Data { value, .. }) => Some(value),
        _ => None,
    };
    let short = intrinsic(heap, Intrinsic::PreludeFunctions)
        .as_internal()
        .expect("the prelude function table");
    let g = intrinsic_obj(heap, Intrinsic::Global);
    let mut known: Vec<(u32, Value)> = Vec::new();
    for key in heap.own_keys(g) {
        let Some(x) = data(heap, g, key) else {
            continue;
        };
        let Some(o) = x.as_object().filter(|&o| o != g) else {
            continue;
        };
        let members: Vec<(u32, Value)> = heap
            .own_keys(o)
            .into_iter()
            .filter(|k| k.id() & (MEMBER | crate::heap::atom::SYM_BIT) == 0)
            .filter_map(|k| data(heap, o, k).map(|f| (k.id() | MEMBER, f)))
            .filter(|&(_, f)| fast(heap, f))
            .collect();
        if fast(heap, x) || !members.is_empty() {
            known.push((key.id(), x));
            known.extend(members);
            heap.watch(g, key);
        }
    }
    let first = heap.value_array_len(short);
    let n = first + known.len() as u32;
    let names = heap.alloc_u32_array(known.len() as u32 + 1)?;
    let table = heap.alloc_value_array(n)?;
    heap.arena_mut().set(table, ValueArray::LEN, n);
    heap.u32_array_set(names, 0, first);
    for i in 0..first {
        let v = heap.value_array_get(short, i);
        heap.value_array_set(table, i, v);
    }
    for (i, &(name, v)) in known.iter().enumerate() {
        heap.u32_array_set(names, i as u32 + 1, name);
        heap.value_array_set(table, first + i as u32, v);
    }
    realm::set_intrinsic(heap, Intrinsic::KnownNames, Value::internal(names));
    realm::set_intrinsic(heap, Intrinsic::PreludeFunctions, Value::internal(table));
    Ok(())
}
