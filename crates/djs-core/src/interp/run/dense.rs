//! Leaves for the element loops of Array builtins over dense arrays (0080).
//!
//! A present element in an Array's dense storage is an own, writable, enumerable,
//! configurable data property, so HasProperty and Get of it run no user code and do
//! not reach the prototype chain. When every element a loop would touch is such an
//! element (and the loop's other steps cannot run user code either), the loop runs
//! here in one call; anything else (a hole, an index past the dense storage, another
//! kind of object) makes the leaf decline with nothing changed, and the builtin runs
//! its general loop, which re-reads everything (the reads here had no effect).

use alloc::vec::Vec;

use super::*;
use crate::heap::StrView;
use crate::value::Str;

/// The dense elements `o[from .. to)` of an Array, if all are present.
fn dense_range(heap: &Heap, o: Obj, from: u32, to: u32) -> Option<Vec<Value>> {
    if heap.object_class(o) != ObjectClass::Array || to > heap.dense_len(o) {
        return None;
    }
    let mut out = Vec::with_capacity(to.saturating_sub(from) as usize);
    for i in from..to {
        out.push(heap.get_element_fast(o, i)?);
    }
    Some(out)
}

/// An integral Number in [0, 2^32) as a u32.
fn index(v: f64) -> Option<u32> {
    ((0.0..4294967296.0).contains(&v) && (v as u32) as f64 == v).then_some(v as u32)
}

impl Exec<'_> {
    /// `%ArraySearchDense(o, search, from, len, mode)` (builtins/intrinsics.d.ts).
    pub(super) fn array_search_dense(
        &self,
        o: Obj,
        search: Value,
        from: f64,
        len: f64,
        mode: i32,
    ) -> f64 {
        const DECLINE: f64 = -2.0;
        if self.heap.object_class(o) != ObjectClass::Array {
            return DECLINE;
        }
        let (Some(from), Some(len)) = (index(from), index(len)) else {
            return DECLINE;
        };
        let el_ok = |i: u32| self.heap.get_element_fast(o, i);
        let same_value_zero = |v: Value| match (v.as_number(), search.as_number()) {
            (Some(a), Some(b)) => a == b || (a.is_nan() && b.is_nan()),
            _ => conv::strict_equals(self.heap, v, search),
        };
        match mode {
            // indexOf / includes: [from, len) must all be dense up to the match.
            0 | 2 => {
                if len > self.heap.dense_len(o) {
                    return DECLINE;
                }
                for i in from..len {
                    let Some(v) = el_ok(i) else {
                        return DECLINE;
                    };
                    let hit = if mode == 0 {
                        conv::strict_equals(self.heap, v, search)
                    } else {
                        same_value_zero(v)
                    };
                    if hit {
                        return i as f64;
                    }
                }
                -1.0
            }
            // lastIndexOf: from `from` down to 0.
            1 => {
                if from >= self.heap.dense_len(o) {
                    return DECLINE;
                }
                for i in (0..=from).rev() {
                    let Some(v) = el_ok(i) else {
                        return DECLINE;
                    };
                    if conv::strict_equals(self.heap, v, search) {
                        return i as f64;
                    }
                }
                -1.0
            }
            _ => DECLINE,
        }
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
        if fin < k || self.heap.object_class(a) != ObjectClass::Array {
            return Ok(false);
        }
        let Some(values) = dense_range(self.heap, o, k, fin) else {
            return Ok(false);
        };
        for (n, v) in values.into_iter().enumerate() {
            let n = n as u32;
            // The first store checks what makes all of them plain appends (an
            // extensible Array, no index properties on it or its prototypes) and
            // declines with nothing changed; those do not change in between.
            if !self.heap.set_element_fast(a, n, v)? {
                if n == 0 {
                    return Ok(false);
                }
                let key = self.heap.atom_from_index(n)?;
                self.heap.create_data_property(a, key, v)?;
            }
        }
        Ok(true)
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
        let mut props = Vec::with_capacity(keys.len());
        for k in keys {
            if k.is_symbol() {
                continue;
            }
            match self.heap.get_own_property_alloc(o, k)? {
                Some(Property::Data { value, flags }) if flags.enumerable() => {
                    props.push((k, value))
                }
                Some(Property::Accessor { flags, .. }) if flags.enumerable() => {
                    return Ok(Value::UNDEFINED);
                }
                _ => {}
            }
        }
        let proto = intrinsic_obj(self.heap, Intrinsic::ArrayProto);
        let mut out = Vec::with_capacity(props.len());
        for (k, v) in props {
            out.push(if kind == 1 {
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

    /// `%ArrayJoinDense(o, len, sep)`: the joined string, or undefined.
    pub(super) fn array_join_dense(&mut self, o: Obj, len: f64, sep: Str) -> Result<Value, Fault> {
        let Some(len) = index(len) else {
            return Ok(Value::UNDEFINED);
        };
        let Some(values) = dense_range(self.heap, o, 0, len) else {
            return Ok(Value::UNDEFINED);
        };
        // Symbols throw and objects run user code: the general loop does those.
        if values
            .iter()
            .any(|v| v.as_object().is_some() || v.as_symbol().is_some())
        {
            return Ok(Value::UNDEFINED);
        }
        let sep = units_of(self.heap, sep)?;
        let mut out: Vec<u16> = Vec::new();
        let mut buf = String::new();
        for (i, v) in values.into_iter().enumerate() {
            if i > 0 {
                out.extend_from_slice(&sep);
            }
            if let Some(s) = v.as_string() {
                out.extend(units_of(self.heap, s)?);
            } else if let Some(n) = v.as_int() {
                use core::fmt::Write;
                buf.clear();
                let _ = write!(buf, "{n}");
                out.extend(buf.bytes().map(u16::from));
            } else if !v.is_nullish() {
                let s = conv::to_string(self.heap, v)?;
                out.extend(units_of(self.heap, s)?);
            }
        }
        let s = if out.iter().all(|&u| u < 256) {
            let bytes: Vec<u8> = out.iter().map(|&u| u as u8).collect();
            self.heap.alloc_latin1(&bytes)?
        } else {
            self.heap.alloc_utf16(&out)?
        };
        Ok(Value::string(s))
    }
}

/// The code units of `s`.
fn units_of(heap: &mut Heap, s: Str) -> Result<Vec<u16>, Fault> {
    Ok(match heap.str_view_flat(s)? {
        StrView::Latin1(b) => b.iter().map(|&c| u16::from(c)).collect(),
        view @ StrView::Utf16(_) => view.units().collect(),
    })
}
