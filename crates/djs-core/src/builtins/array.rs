//! Array leaf intrinsics: the primitive sort (ARCH "The leaf rule": `sort` on
//! primitives without a comparator stays leaf).
use crate::PushOut;

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
    let mut values = crate::buf::vec_with_capacity(n as usize);
    for i in 0..n {
        match heap.get_element_fast(list, i) {
            Some(v) if !matches!(v.kind(), Kind::Object | Kind::Symbol) => values.push_out(v),
            _ => return Ok(false),
        }
    }
    let mut keyed: Vec<(Vec<u16>, Value)> = crate::buf::vec_with_capacity(values.len());
    let mut undefined = 0;
    for v in values {
        if v.is_undefined() {
            undefined += 1;
            continue;
        }
        let s = conv::to_string(heap, v)?;
        keyed.push_out((heap.str_units(s).collect(), v));
    }
    // A stable sort by code units, as IsLessThan compares strings.
    crate::sort::sort_by(&mut keyed, |a, b| a.0 < b.0);
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

/// `%SortNumbers(list)`: a typed array's default sort (23.2.3.30 with no comparator):
/// numeric, -0 before +0, NaN last; stable.
pub(crate) fn sort_numbers(heap: &mut Heap, list: Obj) -> Result<(), Fault> {
    let n = heap.array_length(list);
    if n > 0
        && heap
            .get_element_fast(list, 0)
            .is_some_and(|v| v.as_bigint().is_some())
    {
        // A BigInt typed array's elements (0046): sorted by value.
        let mut values: Vec<(crate::bigint::BigInt, Value)> = (0..n)
            .filter_map(|i| heap.get_element_fast(list, i))
            .map(|v| (heap.bigint(v), v))
            .collect();
        crate::sort::sort_by(&mut values, |a, b| a.0 < b.0);
        for (i, (_, v)) in values.into_iter().enumerate() {
            heap.set_element_fast(list, i as u32, v)?;
        }
        return Ok(());
    }
    let mut values: Vec<f64> = (0..n)
        .map(|i| {
            heap.get_element_fast(list, i)
                .and_then(|v| v.as_number())
                .unwrap_or(f64::NAN)
        })
        .collect();
    // crate::sort, not core's sorts (D1432).
    crate::sort::sort_by(&mut values, |a, b| match (a.is_nan(), b.is_nan()) {
        (_, true) => !a.is_nan(),
        (true, false) => false,
        _ if a == b && *a == 0.0 => a.is_sign_negative() && !b.is_sign_negative(),
        _ => a < b,
    });
    for (i, x) in values.into_iter().enumerate() {
        heap.set_element_fast(list, i as u32, crate::interp::conv::number(x))?;
    }
    Ok(())
}
