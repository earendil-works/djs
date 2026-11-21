//! Known builtins (D3300): `Math.floor`, `String`, `Number.isInteger` without looking
//! them up.
//!
//! [`KNOWN`] lists globals and members of theirs that scripts use all the time. When the
//! prelude has run, their values follow its own functions in the realm's prelude function
//! table (`Intrinsic::PreludeFunctions`, D2440), in the list's order, and
//! `Intrinsic::KnownFirst` is the first one's index. What the compiler emits for a name
//! depends on the list alone, so every compile of a text gives the same code, whatever
//! the heap (a compile-cache replay, the dumper's recompile in a scratch heap).
//!
//! For a reference to a listed name the compiler emits `get_builtin k` (two for a method
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

/// The known builtins in table order: globals, each followed by its known members (own
/// data properties, each with a dot before it).
const KNOWN: &[u8] = b"Math.floor.ceil.round.trunc.abs.sqrt.max.min.pow.sign.random.log.PI \
Number.isInteger.isFinite.isNaN.parseFloat.parseInt String.fromCharCode Array.isArray.from \
Object.keys.values.entries.assign.fromEntries.freeze JSON.stringify.parse Date.now \
Promise.all.resolve Symbol.iterator parseInt parseFloat isNaN Boolean Map Set Error RegExp";

/// Whether atom `a` is the string `name`.
fn named(heap: &Heap, a: Atom, name: &[u8]) -> bool {
    heap.atom_string_of(a)
        .and_then(|s| heap.str_view(s))
        .is_some_and(|v| matches!(v, crate::heap::StrView::Latin1(b) if b == name))
}

/// The table indexes of the known global `global`, and of its member `member` when that
/// is given and known. In a heap without a realm (a compile for its code's shape only)
/// they count from 0.
#[inline(never)]
pub(crate) fn lookup(
    heap: &Heap,
    global: Atom,
    member: Option<Atom>,
) -> Option<(u16, Option<u16>)> {
    let mut k = 0;
    if heap.realm().as_internal().is_some() {
        // (A VM's own atoms come after the base's, which hold every listed name.)
        let base = heap.atom_base_count();
        if base != 0 && global.id() >= base {
            return None;
        }
        k = intrinsic(heap, Intrinsic::KnownFirst).as_int()? as u16;
    }
    for group in KNOWN.split(|&b| b == b' ') {
        let mut names = group.split(|&b| b == b'.');
        let found = named(heap, global, names.next()?);
        let kx = k;
        for name in names {
            k += 1;
            if found && member.is_some_and(|m| named(heap, m, name)) {
                return Some((kx, Some(k)));
            }
        }
        if found {
            return Some((kx, None));
        }
        k += 1;
    }
    None
}

/// The builtins changed: `get_builtin` of a known builtin declines from now on.
pub(crate) fn unpristine(heap: &mut Heap) {
    if heap.realm().as_internal().is_none() {
        return;
    }
    if let (Some(first), Some(t)) = (
        intrinsic(heap, Intrinsic::KnownFirst).as_int(),
        intrinsic(heap, Intrinsic::PreludeFunctions).as_internal(),
    ) && heap.value_array_len(t) > first as u32
    {
        heap.arena_mut().set(t, ValueArray::LEN, first as u32);
    }
}

/// Whether the known builtins are still the base image's (false without them).
#[cfg(test)]
pub(crate) fn pristine(heap: &Heap) -> bool {
    matches!((
        intrinsic(heap, Intrinsic::KnownFirst).as_int(),
        intrinsic(heap, Intrinsic::PreludeFunctions).as_internal(),
    ), (Some(first), Some(t)) if heap.value_array_len(t) > first as u32)
}

/// Appends the known builtins' values to the prelude function table, which the prelude
/// has filled, and marks their globals' bindings (`STORE_WATCHED`). A base image keeps
/// the table in its tail: a VM shortens it.
#[cfg(djs_prelude)]
pub(crate) fn build(heap: &mut Heap) -> Result<(), crate::heap::HeapError> {
    use alloc::vec::Vec;

    use super::realm::{self, intrinsic_obj};
    use crate::heap::Property;
    use crate::value::Value;

    let data = |heap: &mut Heap, o, name: &[u8]| {
        let name = core::str::from_utf8(name).expect("ASCII");
        let key = heap.atom_str(name)?;
        match heap.get_own_property(o, key) {
            Some(Property::Data { value, .. }) => Ok((key, value)),
            _ => panic!("known builtin {name}: not an own data property"),
        }
    };
    let Some(short) = intrinsic(heap, Intrinsic::PreludeFunctions).as_internal() else {
        return Ok(());
    };
    let g = intrinsic_obj(heap, Intrinsic::Global);
    let mut known: Vec<Value> = Vec::new();
    for group in KNOWN.split(|&b| b == b' ') {
        let mut names = group.split(|&b| b == b'.');
        let (key, x) = data(heap, g, names.next().expect("a name"))?;
        heap.watch(g, key);
        known.push(x);
        for name in names {
            let o = x.as_object().expect("known builtin: not an object");
            known.push(data(heap, o, name)?.1);
        }
    }
    let first = heap.value_array_len(short);
    let n = first + known.len() as u32;
    let table = heap.alloc_value_array(n)?;
    heap.arena_mut().set(table, ValueArray::LEN, n);
    for i in 0..n {
        let v = match i.checked_sub(first) {
            Some(k) => known[k as usize],
            None => heap.value_array_get(short, i),
        };
        heap.value_array_set(table, i, v);
    }
    realm::set_intrinsic(heap, Intrinsic::KnownFirst, Value::int(first as i32));
    realm::set_intrinsic(heap, Intrinsic::PreludeFunctions, Value::internal(table));
    Ok(())
}
