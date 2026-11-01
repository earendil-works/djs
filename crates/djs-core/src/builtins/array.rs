//! Array leaf intrinsics: the primitive sort (ARCH "The leaf rule": `sort` on
//! primitives without a comparator stays leaf).

use alloc::vec::Vec;

use crate::heap::{Heap, ObjectClass};
use crate::interp::conv::{self, Fault};
use crate::value::{Kind, Obj, Value};

/// `%SortPrimitives(list)`: sorts a dense array of non-symbol primitives by
/// SortCompare without a comparator (undefined last, then by ToString, stable). Returns
/// false without touching the list if an element is an object, a symbol or a hole.
pub(crate) fn sort_primitives(heap: &mut Heap, list: Obj) -> Result<bool, Fault> {
    if heap.object_class(list) != ObjectClass::Array {
        return Ok(false);
    }
    let n = heap.array_length(list);
    let mut values = Vec::with_capacity(n as usize);
    for i in 0..n {
        match heap.get_element_fast(list, i) {
            Some(v) if !matches!(v.kind(), Kind::Object | Kind::Symbol) => values.push(v),
            _ => return Ok(false),
        }
    }
    let mut keyed: Vec<(Vec<u16>, Value)> = Vec::with_capacity(values.len());
    let mut undefined = 0;
    for v in values {
        if v.is_undefined() {
            undefined += 1;
            continue;
        }
        let s = conv::to_string(heap, v)?;
        keyed.push((heap.str_units(s).collect(), v));
    }
    // A stable sort by code units, as IsLessThan compares strings.
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    let mut i = 0;
    for (_, v) in keyed {
        heap.set_element_fast(list, i, v)?;
        i += 1;
    }
    for _ in 0..undefined {
        heap.set_element_fast(list, i, Value::UNDEFINED)?;
        i += 1;
    }
    Ok(true)
}
