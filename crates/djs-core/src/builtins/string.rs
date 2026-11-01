//! String leaf intrinsics on the heap representation (one-byte and two-byte strings,
//! ropes walked in chunks where that avoids flattening).

use alloc::vec::Vec;

use crate::heap::Heap;
use crate::interp::conv::Fault;
use crate::interp::{Intrinsic, realm};
use crate::value::{Obj, Str, Value};

/// IsLessThan for two strings, as -1, 0 or 1 (code unit order).
pub(crate) fn compare(heap: &Heap, a: Str, b: Str) -> f64 {
    match heap.str_units(a).cmp(heap.str_units(b)) {
        core::cmp::Ordering::Less => -1.0,
        core::cmp::Ordering::Equal => 0.0,
        core::cmp::Ordering::Greater => 1.0,
    }
}

/// The code points of `s` as an array of one- or two-unit strings (lone surrogates on
/// their own), as the String iterator yields them.
pub(crate) fn code_points(heap: &mut Heap, s: Str) -> Result<Obj, Fault> {
    let units: Vec<u16> = heap.str_units(s).collect();
    let mut values = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let n = if (0xD800..0xDC00).contains(&units[i])
            && units
                .get(i + 1)
                .is_some_and(|u| (0xDC00..0xE000).contains(u))
        {
            2
        } else {
            1
        };
        values.push(Value::string(heap.alloc_utf16(&units[i..i + n])?));
        i += n;
    }
    let proto = realm::intrinsic_obj(heap, Intrinsic::ArrayProto);
    Ok(heap.new_array_from_values(Some(proto), &values)?)
}
