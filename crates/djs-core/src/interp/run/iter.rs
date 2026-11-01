//! Iteration without iterator result objects (0080, D1800).
//!
//! A step of the iterator protocol calls `next` and reads `done` and `value` from the
//! result. When `next` is the original `%ArrayIteratorPrototype%.next` (or the String
//! Iterator's) and the iterator is one of its objects, that result would be a fresh
//! ordinary object with own data properties `value` and `done`: reading them back runs
//! no user code, so producing the value directly is indistinguishable. Only `next`
//! itself matters here, not how the iterator was obtained: the protocol calls the
//! `next` value read once at the start, which is the one checked on every step.
//!
//! The fast step covers the cases where the original `next` runs no user code either:
//! a present dense element of an Array (holes and anything beyond take the real `next`,
//! whose `Get` may reach the prototype chain or a getter), an in-bounds Number element
//! of a typed array, and a code point of a string. The last step (`done`) also takes
//! the real `next`, which finishes the iterator and allocates the one final result.

use super::intrinsics::{
    ARRAY_ITERATOR_NEXT, ARRAY_PROTOTYPE_VALUES, STRING_ITERATOR_NEXT, STRING_PROTOTYPE_ITERATOR,
    id,
};
use super::*;
use crate::builtins::string;
use crate::heap::{Lookup, atoms};

/// The kind slot of a String Iterator (D1346).
const STRING_KIND: i32 = 3;

const GET_ITERATOR: u16 = id("GetIterator");
const ITERATOR_CLOSE: u16 = id("IteratorClose");

impl Exec<'_> {
    /// `%GetIterator(obj)` and `%IteratorClose(it)` of for-of and array destructuring
    /// without a frame, where they run no user code (D1800): GetIterator of an Array or
    /// a string whose @@iterator is the original (a data property, as D1351 checks), and
    /// IteratorClose of an Array or String Iterator without a `return` method on its
    /// chain. `None`, with nothing changed, otherwise.
    pub(super) fn iter_intrinsic_fast(
        &mut self,
        id: u16,
        base: u32,
        argc: u32,
    ) -> Result<Option<Value>, Fault> {
        if argc != 1 {
            return Ok(None);
        }
        let a0 = self.get(base);
        match id {
            GET_ITERATOR => {
                let (holder, original, proto, target, kind) = if a0.as_string().is_some() {
                    (
                        intrinsic_obj(self.heap, Intrinsic::StringProto),
                        STRING_PROTOTYPE_ITERATOR,
                        Intrinsic::StringIteratorProto,
                        a0,
                        STRING_KIND,
                    )
                } else {
                    match a0.as_object() {
                        Some(o) if self.heap.object_class(o) == ObjectClass::Array => (
                            o,
                            ARRAY_PROTOTYPE_VALUES,
                            Intrinsic::ArrayIteratorProto,
                            a0,
                            1,
                        ),
                        _ => return Ok(None),
                    }
                };
                let method = match self.heap.find_property(holder, atoms::SYM_ITERATOR) {
                    Some((_, Property::Data { value, .. })) => value,
                    _ => return Ok(None),
                };
                if method != realm::installed_intrinsic(self.heap, original) {
                    return Ok(None);
                }
                let proto = intrinsic_obj(self.heap, proto);
                let it = self.heap.alloc_object(
                    ObjectClass::ArrayIterator,
                    Some(proto),
                    ObjectClass::ArrayIterator.slots(),
                )?;
                self.heap.set_object_slot(it, 0, target);
                self.heap.set_object_slot(it, 1, Value::int(0));
                self.heap.set_object_slot(it, 2, Value::int(kind));
                Ok(Some(Value::object(it)))
            }
            ITERATOR_CLOSE => {
                let Some(it) = a0
                    .as_object()
                    .filter(|&o| self.heap.object_class(o) == ObjectClass::ArrayIterator)
                else {
                    return Ok(None);
                };
                match self.heap.lookup_property(it, atoms::RETURN) {
                    Lookup::Absent => Ok(Some(Value::UNDEFINED)),
                    Lookup::Found(_, Property::Data { value, .. }) if value.is_nullish() => {
                        Ok(Some(Value::UNDEFINED))
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    /// The next step of iterator `it` with `next` method `f`, if it can skip the
    /// protocol (module docs); the iterator has then advanced (or finished). `Slow`,
    /// with nothing changed, when the caller must call `next`.
    pub(super) fn iter_fast_next(&mut self, it: Value, f: Value) -> Result<FastStep, Fault> {
        let Some(it) = it.as_object() else {
            return Ok(FastStep::Slow);
        };
        if self.heap.object_class(it) != ObjectClass::ArrayIterator || f.as_object().is_none() {
            return Ok(FastStep::Slow);
        }
        let target = self.heap.object_slot(it, 0);
        let (Some(index), Some(kind)) = (
            self.heap.object_slot(it, 1).as_number(),
            self.heap.object_slot(it, 2).as_int(),
        ) else {
            return Ok(FastStep::Slow);
        };
        let original = if kind == STRING_KIND {
            STRING_ITERATOR_NEXT
        } else {
            ARRAY_ITERATOR_NEXT
        };
        // Index slots stay below 2^32 for the targets handled here.
        if f != realm::installed_intrinsic(self.heap, original) || index >= u32::MAX as f64 {
            return Ok(FastStep::Slow);
        }
        if target.is_undefined() {
            return Ok(FastStep::Done);
        }
        let i = index as u32;
        if kind == STRING_KIND {
            let Some(s) = target.as_string() else {
                return Ok(FastStep::Slow);
            };
            if i >= self.heap.str_len(s) {
                self.heap.set_object_slot(it, 0, Value::UNDEFINED);
                return Ok(FastStep::Done);
            }
            let s = self.heap.flatten(s)?;
            let n = match string::code_point_at(self.heap, s, i) {
                Some(c) if c >= 65536.0 => 2,
                Some(_) => 1,
                None => return Ok(FastStep::Slow),
            };
            let cp = string::substring(self.heap, s, i, i + n)?;
            self.heap
                .set_object_slot(it, 1, conv::number((i + n) as f64));
            return Ok(FastStep::Value(Value::string(cp)));
        }
        let Some(o) = target.as_object() else {
            return Ok(FastStep::Slow);
        };
        let len = match self.heap.object_class(o) {
            ObjectClass::Array => self.heap.array_length(o),
            // Detached or out of bounds: the real `next` throws.
            ObjectClass::TypedArray => match self.heap.ta_length(o) {
                Some(n) => n,
                None => return Ok(FastStep::Slow),
            },
            _ => return Ok(FastStep::Slow),
        };
        if i >= len {
            self.heap.set_object_slot(it, 0, Value::UNDEFINED);
            return Ok(FastStep::Done);
        }
        // A hole or a non-dense element may reach a getter or the prototype chain, and
        // a BigInt element allocates: the real `next` reads those.
        let Some(v) = self.heap.get_element_fast(o, i) else {
            return Ok(FastStep::Slow);
        };
        let r = match kind {
            0 => conv::number(i as f64),
            1 => v,
            2 => {
                let proto = intrinsic_obj(self.heap, Intrinsic::ArrayProto);
                let pair = [conv::number(i as f64), v];
                Value::object(self.heap.new_array_from_values(Some(proto), &pair)?)
            }
            _ => return Ok(FastStep::Slow),
        };
        self.heap
            .set_object_slot(it, 1, conv::number(i as f64 + 1.0));
        Ok(FastStep::Value(r))
    }

    /// CreateIterResultObject(undefined, true).
    pub(super) fn done_result(&mut self) -> Result<Value, Fault> {
        let proto = intrinsic_obj(self.heap, Intrinsic::ObjectProto);
        let o = self.heap.new_object_from_entries(
            Some(proto),
            &[atoms::VALUE, atoms::DONE],
            &[Value::UNDEFINED, Value::TRUE],
        )?;
        Ok(Value::object(o))
    }
}

/// The outcome of [`Exec::iter_fast_next`].
pub(super) enum FastStep {
    /// The next value: no result object needed.
    Value(Value),
    /// The iterator is done (and finished).
    Done,
    /// Call `next`.
    Slow,
}
