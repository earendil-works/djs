//! Tab completion from the heap (0102): names of globals and of properties along a
//! dotted path, read with the heap's own-property reads only. No getter, proxy trap or
//! other JS runs: a path through an accessor or a proxy completes nothing.

use djs_core::heap::{Heap, ObjectClass, Property};
use djs_core::interp::{Intrinsic, intrinsic};
use djs_core::value::{Obj, Value};
use djs_core::vm::Vm;

/// Completions for the text before the cursor: the number of characters of the word being
/// completed (they are replaced) and the candidates, sorted.
pub fn complete(vm: &Vm, before: &str) -> (usize, Vec<String>) {
    let is_part = |c: char| c.is_alphanumeric() || c == '_' || c == '$' || c == '.';
    let start = before
        .char_indices()
        .rev()
        .take_while(|&(_, c)| is_part(c))
        .last()
        .map_or(before.len(), |(i, _)| i);
    let expr = &before[start..];
    let (path, word) = match expr.rfind('.') {
        Some(i) => (&expr[..i], &expr[i + 1..]),
        None => ("", expr),
    };
    let n = word.chars().count();
    // A number (`1.5`), or a path that does not start with a name.
    if expr.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
        return (n, Vec::new());
    }
    let heap = vm.heap();
    let roots: Vec<Obj> = if path.is_empty() {
        vec![vm.global_lexical(), vm.global()]
    } else {
        match resolve(vm, path) {
            Some(o) => vec![o],
            None => return (n, Vec::new()),
        }
    };
    let mut names = Vec::new();
    for o in roots {
        names.extend(chain_names(heap, o));
    }
    names.retain(|k| k.starts_with(word) && is_identifier(k));
    names.sort();
    names.dedup();
    (n, names)
}

/// The object a dotted path names, through data properties only.
fn resolve(vm: &Vm, path: &str) -> Option<Obj> {
    let heap = vm.heap();
    let mut parts = path.split('.');
    let first = parts.next()?;
    let mut v = data_property(heap, vm.global_lexical(), first)
        .filter(|v| !v.is_uninitialized())
        .or_else(|| data_property(heap, vm.global(), first))?;
    for p in parts {
        let o = object_of(heap, v)?;
        v = data_property(heap, o, p)?;
    }
    object_of(heap, v)
}

/// An object to read properties of: the value, or a primitive's prototype.
fn object_of(heap: &Heap, v: Value) -> Option<Obj> {
    if let Some(o) = v.as_object() {
        return (heap.object_class(o) != ObjectClass::Proxy).then_some(o);
    }
    let proto = if v.as_string().is_some() {
        Intrinsic::StringProto
    } else if v.is_number() {
        Intrinsic::NumberProto
    } else if v.as_bool().is_some() {
        Intrinsic::BooleanProto
    } else {
        return None;
    };
    intrinsic(heap, proto).as_object()
}

/// The value of property `name` along `o`'s prototype chain, if it is a data property
/// (an accessor or a proxy on the way gives `None`).
fn data_property(heap: &Heap, o: Obj, name: &str) -> Option<Value> {
    let mut cur = Some(o);
    while let Some(c) = cur {
        if heap.object_class(c) == ObjectClass::Proxy {
            return None;
        }
        for k in heap.own_keys(c) {
            if !k.is_symbol() && heap.atom_name(k) == name {
                return match heap.get_own_property(c, k)? {
                    Property::Data { value, .. } => Some(value),
                    Property::Accessor { .. } => None,
                };
            }
        }
        cur = heap.get_prototype_of(c);
    }
    None
}

/// The string keys of `o` and its prototypes.
fn chain_names(heap: &Heap, o: Obj) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = Some(o);
    while let Some(c) = cur {
        if heap.object_class(c) == ObjectClass::Proxy {
            break;
        }
        for k in heap.own_keys(c) {
            if !k.is_symbol() {
                out.push(heap.atom_name(k));
            }
        }
        cur = heap.get_prototype_of(c);
    }
    out
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}
