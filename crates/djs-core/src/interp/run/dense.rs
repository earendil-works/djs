//! The leaf intrinsics over dense arrays of the builtin language (0080, D1801): fuel and
//! argument checks around builtins/dense.rs, plus Object.values/entries and typed
//! arrays from dense arrays.
use crate::PushOut;

use alloc::vec::Vec;

use super::*;
use crate::builtins::dense::{self, dense_range, index};
use crate::value::Str;

impl Exec<'_> {
    /// `%ArrayFillDense(o, value, k, fin)`: the index the general loop continues from.
    pub(super) fn array_fill_dense(
        &mut self,
        o: Obj,
        value: Value,
        k: f64,
        fin: f64,
    ) -> Result<f64, Fault> {
        let (Some(k), Some(fin)) = (index(k), index(fin)) else {
            return Ok(k);
        };
        Ok(dense::fill(self.heap, o, value, k, fin)? as f64)
    }

    /// `%ArraySearchDense(o, search, from, len, mode)` (builtins/intrinsics.d.ts).
    pub(super) fn array_search_dense(
        &mut self,
        o: Obj,
        search: Value,
        from: f64,
        len: f64,
        mode: i32,
    ) -> f64 {
        let (Some(from), Some(len)) = (index(from), index(len)) else {
            return dense::DECLINE;
        };
        dense::search(self.heap, o, search, from, len, mode)
    }

    /// `%ArraySliceDense(o, k, fin, a)`.
    pub(super) fn array_slice_dense(
        &mut self,
        o: Obj,
        k: f64,
        fin: f64,
        a: Obj,
    ) -> Result<bool, Fault> {
        let (Some(k), Some(fin)) = (index(k), index(fin)) else {
            return Ok(false);
        };
        dense::slice(self.heap, o, k, fin, a)
    }

    /// `%EnumerableOwnDataProperties(o, kind)`: Object.values (kind 1) / entries (2)
    /// in one step when no enumerable string-keyed property of `o` is an accessor and
    /// `o` has no exotic [[GetOwnProperty]] that could run code (a Proxy): then the
    /// per-key Get runs no user code and the enumerable checks cannot change in
    /// between. Undefined, for the general loop, otherwise.
    pub(super) fn enumerable_own_data(&mut self, o: Obj, kind: i32) -> Result<Value, Fault> {
        if !matches!(
            self.heap.object_class(o),
            ObjectClass::Ordinary
                | ObjectClass::Array
                | ObjectClass::Function
                | ObjectClass::Error
                | ObjectClass::TypedArray
        ) {
            return Ok(Value::UNDEFINED);
        }
        let keys = self.heap.own_keys(o);
        let mut props = crate::buf::vec_with_capacity(keys.len());
        for k in keys {
            if k.is_symbol() {
                continue;
            }
            match self.heap.get_own_property_alloc(o, k)? {
                Some(Property::Data { value, flags }) if flags.enumerable() => {
                    props.push_out((k, value))
                }
                Some(Property::Accessor { flags, .. }) if flags.enumerable() => {
                    return Ok(Value::UNDEFINED);
                }
                _ => {}
            }
        }
        let proto = intrinsic_obj(self.heap, Intrinsic::ArrayProto);
        let mut out = crate::buf::vec_with_capacity(props.len());
        for (k, v) in props {
            out.push_out(if kind == 1 {
                v
            } else {
                let key = self.heap.atom_to_value(k)?;
                Value::object(self.heap.new_array_from_values(Some(proto), &[key, v])?)
            });
        }
        Ok(Value::object(
            self.heap.new_array_from_values(Some(proto), &out)?,
        ))
    }

    /// `%ObjectFromEntriesDense(iterable)` (builtins/intrinsics.d.ts): when the Array's
    /// iteration is the original one, each step reads a present dense element, and
    /// reading "0" and "1" of an entry whose elements are present and dense is what Get
    /// does; with a string key nothing in the loop runs user code. So once every entry
    /// is checked, the properties are defined in one go. Undefined otherwise.
    pub(super) fn object_from_entries_dense(&mut self, v: Value) -> Result<Value, Fault> {
        let Some(a) = v.as_object().filter(|_| self.iteration_unmodified(v)) else {
            return Ok(Value::UNDEFINED);
        };
        let n = self.heap.array_length(a);
        if (0..n).any(|i| entry_pair(self.heap, a, i).is_none()) {
            return Ok(Value::UNDEFINED);
        }
        self.heap.charge_elements(n as usize);
        let proto = intrinsic_obj(self.heap, Intrinsic::ObjectProto);
        let o = self.heap.new_object(Some(proto))?;
        for i in 0..n {
            if let Some((k, x)) = entry_pair(self.heap, a, i) {
                let k = self.heap.intern(k)?;
                self.heap.create_data_property(o, k, x)?;
            }
        }
        Ok(Value::object(o))
    }

    /// `%TypedArraySetFromDense(o, values)`.
    pub(super) fn typed_array_set_from_dense(&mut self, o: Obj, values: Obj) -> bool {
        if self.heap.object_class(o) != ObjectClass::TypedArray
            || self.heap.object_class(values) != ObjectClass::Array
        {
            return false;
        }
        let n = self.heap.array_length(values);
        if self.heap.ta_length(o).is_none_or(|len| len < n) {
            return false;
        }
        let kind = self.heap.ta_kind(o);
        let Some(elems) = dense_range(self.heap, values, 0, n).and_then(|vals| {
            vals.into_iter()
                .map(|v| self.heap.element_of(kind, v))
                .collect::<Option<Vec<_>>>()
        }) else {
            return false;
        };
        for (i, e) in elems.into_iter().enumerate() {
            self.heap.ta_set(o, i as u32, e);
        }
        true
    }

    /// `%ArrayPushDense(o, v)`. Inline: push's hot path (D2255).
    #[inline]
    pub(super) fn array_push_dense(&mut self, o: Obj, v: Value) -> Result<f64, Fault> {
        Ok(dense::push(self.heap, o, v)?.map_or(-1.0, f64::from))
    }

    /// `%ArrayJoinDense(o, len, sep)`: the joined string, or undefined.
    pub(super) fn array_join_dense(&mut self, o: Obj, len: f64, sep: Str) -> Result<Value, Fault> {
        let Some(len) = index(len) else {
            return Ok(Value::UNDEFINED);
        };
        Ok(dense::join(self.heap, o, len, sep)?.map_or(Value::UNDEFINED, Value::string))
    }
}

/// Element `i` of `a` as an entry of Object.fromEntries' one-step path: a present dense
/// element that is an object whose "0" (a string) and "1" are present dense elements.
#[inline(never)]
fn entry_pair(heap: &crate::heap::Heap, a: Obj, i: u32) -> Option<(Str, Value)> {
    let e = heap.get_element_fast(a, i)?.as_object()?;
    Some((
        heap.get_element_fast(e, 0)?.as_string()?,
        heap.get_element_fast(e, 1)?,
    ))
}
