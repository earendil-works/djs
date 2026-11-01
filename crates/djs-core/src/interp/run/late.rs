//! Leaf intrinsics of the remaining builtins (0051): iterator helpers. The intrinsic
//! fallback chain runs `call_intrinsic` → `json_regexp_intrinsic` → `bigint_intrinsic` →
//! [`Exec::late_intrinsic`].

use super::intrinsics::{bad_argument, id};
use super::*;

const CREATE_ITERATOR_HELPER: u16 = id("CreateIteratorHelper");
const THIS_ITERATOR_HELPER: u16 = id("ThisIteratorHelper");

/// The kind (slot 0) of `Iterator.from`'s wrapper; the other kinds are helpers
/// (builtins/iterator.ts).
const KIND_WRAP: i32 = 0;

impl Exec<'_> {
    /// `call_intrinsic` for the leaves of 0051; the end of the fallback chain.
    pub(super) fn late_intrinsic(
        &mut self,
        id: u16,
        name: &str,
        base: u32,
        argc: u32,
        next: u32,
    ) -> Step {
        let arg = |ex: &Self, i: u32| {
            if i < argc {
                ex.get(base + i)
            } else {
                Value::UNDEFINED
            }
        };
        let a0 = arg(self, 0);
        let v = match id {
            CREATE_ITERATOR_HELPER => {
                let Some(kind) = a0.as_int() else {
                    return bad_argument(name);
                };
                let proto = if kind == KIND_WRAP {
                    Intrinsic::WrapForValidIteratorProto
                } else {
                    Intrinsic::IteratorHelperProto
                };
                let proto = intrinsic_obj(self.heap, proto);
                let slots = ObjectClass::IteratorHelper.slots();
                let o = self
                    .heap
                    .alloc_object(ObjectClass::IteratorHelper, Some(proto), slots)?;
                // kind, state (suspended-start), iterator, next method, the rest unset.
                let init = [a0, Value::int(0), arg(self, 1), arg(self, 2)];
                for i in 0..slots {
                    let v = init.get(i as usize).copied().unwrap_or(Value::UNDEFINED);
                    self.heap.set_object_slot(o, i, v);
                }
                Value::object(o)
            }
            THIS_ITERATOR_HELPER => {
                let wrap = arg(self, 1) == Value::TRUE;
                let ok = a0.as_object().is_some_and(|o| {
                    self.heap.object_class(o) == ObjectClass::IteratorHelper
                        && (self.heap.object_slot(o, 0) == Value::int(KIND_WRAP)) == wrap
                });
                if !ok {
                    let msg = if wrap {
                        "Iterator Wrap object expected"
                    } else {
                        "Iterator Helper object expected"
                    };
                    return type_error(msg);
                }
                a0
            }
            _ => {
                return Err(Fault::Error(
                    ErrorKind::InternalError,
                    format!("not supported yet: %{name}"),
                ));
            }
        };
        self.sp = base;
        self.push(v);
        self.pc = next;
        Ok(None)
    }
}
