//! Known builtins (D3300): `Math.floor`, `String`, `Number.isInteger` without looking
//! them up.
//!
//! [`KNOWN`] lists globals and members of theirs that scripts use all the time. A realm
//! names them in `Intrinsic::KnownNames`: word 0 is the index their values start at in
//! the realm's prelude function table (`Intrinsic::PreludeFunctions`, D2440), then one
//! word per value in the list's order, a global's name atom with [`GLOBAL`] set followed
//! by its members' atoms. Names and atom ids are the same in every VM of a build, so what
//! the compiler emits for a name depends on nothing a script can change. A heap without a
//! realm (the compiler's tests) goes by the list itself and numbers the values from 0:
//! the code has the same shape, which is all such a compile is for.
//!
//! The values are in the table only where a change of a builtin is noticed: the base
//! image's build appends them ([`arm`]) when the realm becomes the builtin prefix. A VM
//! without a prefix (`Vm::new` without frozen builtins) has the names and the same code,
//! whose `get_builtin` declines.
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
#[cfg(any(djs_prelude, test))]
const KNOWN: &[u8] = b"Math.floor.ceil.round.trunc.abs.sqrt.max.min.pow.sign.random.PI \
Number.isInteger.isFinite.isNaN String.fromCharCode Array.isArray parseInt parseFloat isNaN";

/// The table indexes of the known global `global` and of its member `member`, an atom's
/// id; twice the global's when `member` is not a known member of it ([`NO_MEMBER`]: none
/// is asked for).
#[inline(never)]
pub(crate) fn lookup(heap: &Heap, global: Atom, member: u32) -> Option<(u16, u16)> {
    if heap.realm().as_internal().is_none() {
        return by_text(heap, global, member);
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

/// What prelude function table slot `k` holds as a known builtin: the global's name
/// atom and, for a member of it, the member's (the logical snapshot's bytecode hash names
/// `get_builtin` by them, D3711). None for a prelude function's slot.
#[cfg(feature = "snapshot")]
pub(crate) fn name_of(heap: &Heap, k: u32) -> Option<(Atom, Option<Atom>)> {
    let names = intrinsic(heap, Intrinsic::KnownNames).as_internal()?;
    let w = k.checked_sub(heap.u32_array_get(names, 0))? + 1;
    if w >= heap.u32_array_len(names) {
        return None;
    }
    let word = heap.u32_array_get(names, w);
    if word & GLOBAL != 0 {
        return Some((Atom::from_id(word & !GLOBAL), None));
    }
    let g = (1..w)
        .rev()
        .map(|i| heap.u32_array_get(names, i))
        .find(|x| x & GLOBAL != 0)?;
    Some((Atom::from_id(g & !GLOBAL), Some(Atom::from_id(word))))
}

/// No member for [`lookup`]: no member's word has the bit.
pub(crate) const NO_MEMBER: u32 = GLOBAL;

/// [`lookup`] in a heap without a realm, where only tests compile.
#[cfg(not(test))]
fn by_text(_: &Heap, _: Atom, _: u32) -> Option<(u16, u16)> {
    None
}

/// [`lookup`] in a heap without a realm: by the names, the values numbered from 0.
#[cfg(test)]
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

/// Names the known builtins (after the prelude ran): `KnownNames`. Their values are not
/// in the table yet, so every `get_builtin` of one declines: [`arm`] adds them.
#[cfg(djs_prelude)]
pub(crate) fn build(heap: &mut Heap) -> Result<(), crate::heap::HeapError> {
    let Some(short) = intrinsic(heap, Intrinsic::PreludeFunctions).as_internal() else {
        return Ok(());
    };
    let words = KNOWN.iter().filter(|&&b| b == b' ' || b == b'.').count() as u32 + 2;
    let names = heap.alloc_u32_array(words)?;
    heap.u32_array_set(names, 0, heap.value_array_len(short));
    let mut i = 1;
    for group in KNOWN.split(|&b| b == b' ') {
        let mut flag = GLOBAL;
        for name in group.split(|&b| b == b'.') {
            let key = heap.atom_str(core::str::from_utf8(name).expect("ASCII"))?;
            heap.u32_array_set(names, i, key.id() | flag);
            (i, flag) = (i + 1, 0);
        }
    }
    let names = crate::value::Value::internal(names);
    super::realm::set_intrinsic(heap, Intrinsic::KnownNames, names);
    Ok(())
}

/// Appends the known builtins' values, as they are now, to the prelude function table
/// and marks their globals' bindings (`STORE_WATCHED`): for the base image's build, when
/// everything there is becomes the builtin prefix, whose first write ends this
/// ([`unpristine`]). A base image keeps the table in its tail: a VM shortens it. Nothing
/// happens if a listed name is no longer an own data property (a host changed the realm
/// before making it a base): the names stay, the values are never there.
#[cfg(djs_prelude)]
pub(crate) fn arm(heap: &mut Heap) -> Result<(), crate::heap::HeapError> {
    use alloc::vec::Vec;

    use crate::heap::Property;
    use crate::value::Value;

    let Some((short, first)) = table(heap) else {
        return Ok(());
    };
    let Some(names) = intrinsic(heap, Intrinsic::KnownNames).as_internal() else {
        return Ok(());
    };
    let g = super::realm::intrinsic_obj(heap, Intrinsic::Global);
    let mut values: Vec<Value> = Vec::new();
    let mut owner = None;
    for i in 1..heap.u32_array_len(names) {
        let w = heap.u32_array_get(names, i);
        let o = if w & GLOBAL != 0 { Some(g) } else { owner };
        let key = Atom::from_id(w & !GLOBAL);
        let Some(Property::Data { value, .. }) = o.and_then(|o| heap.get_own_property(o, key))
        else {
            return Ok(());
        };
        if w & GLOBAL != 0 {
            owner = value.as_object();
        }
        values.push(value);
    }
    let n = first + values.len() as u32;
    let table = heap.alloc_value_array(n)?;
    for i in 0..n {
        let v = match i.checked_sub(first) {
            Some(k) => values[k as usize],
            None => heap.value_array_get(short, i),
        };
        heap.value_array_set(table, i, v);
    }
    heap.arena_mut().set(table, ValueArray::LEN, n);
    super::realm::set_intrinsic(heap, Intrinsic::PreludeFunctions, Value::internal(table));
    for i in 1..heap.u32_array_len(names) {
        let w = heap.u32_array_get(names, i);
        if w & GLOBAL != 0 {
            heap.watch(g, Atom::from_id(w & !GLOBAL));
        }
    }
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
        // (Without a prefix the values are not there.)
        assert!(!pristine(vm.heap()));
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
