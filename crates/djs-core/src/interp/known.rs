//! Known builtins (D3300): `Math.floor`, `String`, `Number.isInteger` without looking
//! them up.
//!
//! [`KNOWN`] lists globals and members of theirs that scripts use all the time. The base
//! image's build appends their values to the realm's prelude function table
//! (`Intrinsic::PreludeFunctions`, D2440) in the list's order, and names them in
//! `Intrinsic::KnownNames`: word 0 is the table index of the first, then one word per
//! value, a global's name atom with [`GLOBAL`] set followed by its members' atoms. Names
//! and atom ids are the same in every VM of a build, so what the compiler emits for a name
//! depends on nothing a script can change, and a compile-cache replay gives the code a
//! compile would. A heap without a realm (the dumper's recompile for a text's suspension
//! sites, the compiler's tests) goes by the list itself and numbers the values from 0:
//! the code has the same shape, which is all such a compile is for.
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

/// `KnownNames`: the word of a global (the words after it, without the bit, are its
/// members). Bit 31 of an atom id marks an index, which no global is named by.
const GLOBAL: u32 = 1 << 31;

/// The known builtins in table order: globals, each followed by its known members (own
/// data properties, each with a dot before it). Cheap functions that loops call: where
/// the lookup is a good part of the call. An entry costs the base image about six bytes.
#[cfg(any(djs_prelude, feature = "dump", test))]
const KNOWN: &[u8] = b"Math.floor.ceil.round.trunc.abs.sqrt.max.min.pow.sign.random.PI \
Number.isInteger.isFinite.isNaN String.fromCharCode Array.isArray parseInt parseFloat isNaN";

/// The table indexes of the known global `global` and of its member `member`, an atom's
/// id; twice the global's when `member` is not a known member of it ([`NO_MEMBER`]: none
/// is asked for).
#[inline(never)]
pub(crate) fn lookup(heap: &Heap, global: Atom, member: u32) -> Option<(u16, u16)> {
    if heap.realm().as_internal().is_none() {
        #[cfg(any(feature = "dump", test))]
        return by_text(heap, global, member);
        #[cfg(not(any(feature = "dump", test)))]
        return None;
    }
    let names = intrinsic(heap, Intrinsic::KnownNames).as_internal()?;
    let n = heap.u32_array_len(names);
    let mut i = 1;
    while heap.u32_array_get(names, i) != global.id() | GLOBAL {
        i += 1;
        if i >= n {
            return None;
        }
    }
    // (Word 0 is the first value's index, and the words are from 1.)
    let k = |i: u32| (heap.u32_array_get(names, 0) + i - 1) as u16;
    let mut m = i + 1;
    while m < n && heap.u32_array_get(names, m) & GLOBAL == 0 {
        if heap.u32_array_get(names, m) == member {
            return Some((k(i), k(m)));
        }
        m += 1;
    }
    Some((k(i), k(i)))
}

/// No member for [`lookup`]: no member's word has the bit.
pub(crate) const NO_MEMBER: u32 = GLOBAL;

/// [`lookup`] in a heap without a realm: by the names, the values numbered from 0.
#[cfg(any(feature = "dump", test))]
fn by_text(heap: &Heap, global: Atom, member: u32) -> Option<(u16, u16)> {
    let named = |a: Atom, name: &[u8]| a.is_string() && heap.atom_name(a).as_bytes() == name;
    let mut k = 0;
    for group in KNOWN.split(|&b| b == b' ') {
        let mut names = group.split(|&b| b == b'.');
        let found = named(global, names.next()?);
        let kx = k;
        for name in names {
            k += 1;
            if found && named(Atom::from_id(member), name) {
                return Some((kx, k));
            }
        }
        if found {
            return Some((kx, kx));
        }
        k += 1;
    }
    None
}

/// The table and the index of its first known value, in a realm that has them.
fn table(heap: &Heap) -> Option<(u32, u32)> {
    heap.realm().as_internal()?;
    let names = intrinsic(heap, Intrinsic::KnownNames).as_internal()?;
    let t = intrinsic(heap, Intrinsic::PreludeFunctions).as_internal()?;
    Some((t, heap.u32_array_get(names, 0)))
}

/// The builtins changed: `get_builtin` of a known builtin declines from now on.
pub(crate) fn unpristine(heap: &mut Heap) {
    // (The table never has fewer values than the prelude's functions.)
    if let Some((t, first)) = table(heap) {
        heap.arena_mut().set(t, ValueArray::LEN, first);
    }
}

/// Whether the known builtins are still the base image's (false without them).
#[cfg(test)]
pub(crate) fn pristine(heap: &Heap) -> bool {
    table(heap).is_some_and(|(t, first)| heap.value_array_len(t) > first)
}

/// The base image's build renumbered the atoms (`moved`: an id's new one): the names
/// (property keys all, so none was forgotten).
pub(crate) fn renumber(heap: &mut Heap, moved: impl Fn(u32) -> u32) {
    if let Some(names) = intrinsic(heap, Intrinsic::KnownNames).as_internal() {
        for i in 1..heap.u32_array_len(names) {
            let w = heap.u32_array_get(names, i);
            heap.u32_array_set(names, i, moved(w & !GLOBAL) | w & GLOBAL);
        }
    }
}

/// A global lexical binding `key` is declared: if a known global has that name, the
/// binding shadows it, which is a change of it.
#[inline(never)]
pub(crate) fn shadowed(heap: &mut Heap, key: Atom) {
    if lookup(heap, key, NO_MEMBER).is_some() {
        unpristine(heap);
    }
}

/// Appends the known builtins' values to the prelude function table, which the prelude
/// has filled, names them and marks their globals' bindings (`STORE_WATCHED`). A base
/// image keeps the table in its tail: a VM shortens it.
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
    let mut known: Vec<(u32, Value)> = Vec::new();
    for group in KNOWN.split(|&b| b == b' ') {
        let mut names = group.split(|&b| b == b'.');
        let (key, x) = data(heap, g, names.next().expect("a name"))?;
        heap.watch(g, key);
        known.push((key.id() | GLOBAL, x));
        for name in names {
            let o = x.as_object().expect("known builtin: not an object");
            let (key, f) = data(heap, o, name)?;
            known.push((key.id(), f));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{Vm, VmOptions};

    /// Every listed name resolves in a VM, in the list's order, and to the same code
    /// shape as in a heap without a realm (numbered from 0 there).
    #[test]
    fn a_vm_and_a_bare_heap_agree() {
        let mut vm = Vm::new(VmOptions::default()).unwrap();
        let mut bare = Heap::new().unwrap();
        let (_, first) = table(vm.heap()).expect("the known builtins");
        let mut both = |g: &str, m: Option<&str>| {
            let find = |heap: &mut Heap| {
                let g = heap.atom_str(g).unwrap();
                let m = m.map_or(NO_MEMBER, |m| heap.atom_str(m).unwrap().id());
                lookup(heap, g, m)
            };
            let (a, b) = (find(vm.heap_mut()), find(&mut bare));
            let shift = |k: u16| k + first as u16;
            assert_eq!(a, b.map(|(x, f)| (shift(x), shift(f))), "{g} {m:?}");
            a
        };
        let mut k = first as u16;
        for group in core::str::from_utf8(KNOWN).unwrap().split(' ') {
            let mut names = group.split('.');
            let g = names.next().unwrap();
            let kx = k;
            assert_eq!(both(g, None), Some((kx, kx)));
            assert_eq!(both(g, Some("nothing")), Some((kx, kx)));
            for m in names {
                k += 1;
                assert_eq!(both(g, Some(m)), Some((kx, k)));
            }
            k += 1;
        }
        // A member is known under its own global only; other names are not known.
        let (x, f) = both("Number", Some("floor")).unwrap();
        assert_eq!(x, f);
        assert_eq!(both("floor", None), None);
        assert_eq!(both("console", Some("log")), None);
        // (The index whose atom has the bits of `floor`'s.)
        let floor = vm.heap_mut().atom_str("floor").unwrap();
        let index = Atom::int(floor.id()).unwrap();
        let math = vm.heap_mut().atom_str("Math").unwrap();
        let (x, f) = lookup(vm.heap(), math, index.id()).unwrap();
        assert_eq!(x, f);
        assert!(pristine(vm.heap()));
        // A VM from a base image has them too (its build renumbers the atoms).
        vm.prepare_base_image().unwrap();
        let mut based = Vm::from_base(vm.bytes(), VmOptions::default()).unwrap();
        let math = based.heap_mut().atom_str("Math").unwrap();
        let floor = based.heap_mut().atom_str("floor").unwrap();
        let k = first as u16;
        assert_eq!(lookup(based.heap(), math, floor.id()), Some((k, k + 1)));
        assert!(pristine(based.heap()));
        unpristine(based.heap_mut());
        assert!(!pristine(based.heap()));
        unpristine(based.heap_mut());
        assert!(!pristine(based.heap()));
    }
}
