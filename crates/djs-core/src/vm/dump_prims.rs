//! The loader's helper primitives (0182, D2622): what a format 2 dump used to define as
//! JavaScript helpers on `$dump` at its top. Each one makes builtin state directly, so a
//! dump holds only its data, its functions and `$dump.*` calls.
//!
//! | primitive | makes |
//! |---|---|
//! | `builtin(path)` | the base builtin at a registry path (D2371) |
//! | `advance(it, n)` | a fresh array, string, Map or Set iterator, stepped `n` times |
//! | `fromSync(it, next)` | an async-from-sync iterator over `it` |
//! | `closeThrow(it)` | its handler that closes `it` and rethrows |
//! | `spent(k)` | a resolve (0) or reject (1) function whose promise is settled |
//! | `rejected(e)` | a promise rejected with `e`, already handled |
//! | `bytes(s)` | an ArrayBuffer of base64 `s` |
//! | `fn(o)` | the method, getter or setter of a one-member object literal |
//! | `keep(o, keys)` | deletes the own properties a class's evaluation made that it no longer has |
//! | `random(lo, hi)` | sets `Math.random`'s state (the hash key stays the VM's, D2624) |
//! | `using(resources, state)` | the resource Record of a statement list's `using` declarations |
//!
//! Stepping an iterator here runs no user code (no `length` getter, no element getter):
//! the original run did that already.

use alloc::string::String;
use alloc::vec::Vec;

use super::Vm;
use super::dump_load::{array_arg, entries_arg};
use crate::heap::{Atom, ObjectClass, Property, atoms};
use crate::interp::realm::{Intrinsic, intrinsic_obj};
use crate::interp::run::{new_builtin_closure, promise};
use crate::value::Value;

/// A String Iterator's kind (interp/run/iter.rs).
const STRING_KIND: i32 = 3;
/// `AsyncFromSyncIteratorCloseThrow`'s index among the builtin closures.
const FROM_SYNC_CLOSE: usize = 14;

/// The resource Record of `using` declarations (builtins/disposable.ts `UsingCreate`):
/// its slot count, and the slots `$dump.using` names in its state (slot 2 is the
/// resources, an Array of `[value, method, kind]` triples).
pub(super) const USING_SLOTS: u32 = 11;
pub(super) const USING_FIELDS: [(&str, u32); 8] = [
    ("error", 3),
    ("hasError", 4),
    ("needsAwait", 5),
    ("hasAwaited", 6),
    ("next", 7),
    ("value", 8),
    ("pending", 9),
    ("hasPending", 10),
];

fn oom(e: crate::heap::HeapError) -> String {
    String::from(e.as_str())
}

impl Vm {
    pub(super) fn primitive_builtin(&mut self, args: &[Value]) -> Result<Value, String> {
        let path = args
            .first()
            .and_then(|v| v.as_string())
            .map(|s| self.heap.str_to_string(s))
            .ok_or("$dump.builtin: not a path")?;
        let base_end = self.heap.base_end();
        let v = super::registry::resolve_reachable(&self.heap, &path).filter(|v| {
            let cell = v
                .as_object()
                .map(|o| o.offset())
                .or_else(|| v.as_symbol().map(|s| s.offset()));
            cell.is_some_and(|c| c < base_end)
        });
        v.ok_or_else(|| cat!("$dump.builtin: no builtin at ", path))
    }

    pub(super) fn primitive_advance(&mut self, args: &[Value]) -> Result<Value, String> {
        let it = args
            .first()
            .and_then(|v| v.as_object())
            .ok_or("$dump.advance: not an iterator")?;
        let n = args
            .get(1)
            .and_then(|v| v.as_number())
            .filter(|n| *n >= 0.0 && *n == (*n as u64) as f64)
            .ok_or("$dump.advance: not a count")? as u64;
        let h = &mut self.heap;
        match h.object_class(it) {
            ObjectClass::CollectionIterator => {
                for _ in 0..n {
                    if h.collection_iterator_next(it).is_none() {
                        break;
                    }
                }
            }
            ObjectClass::ArrayIterator => {
                let target = h.object_slot(it, 0);
                let index = h.object_slot(it, 1).as_number().unwrap_or(0.0) as u64;
                let kind = h.object_slot(it, 2).as_int();
                if target.is_undefined() {
                    return Ok(Value::object(it));
                }
                let (at, done) = if kind == Some(STRING_KIND) {
                    let s = target.as_string().ok_or("$dump.advance: not a string")?;
                    let units: Vec<u16> = h.str_units(s).collect();
                    let mut at = index as usize;
                    let mut done = false;
                    for _ in 0..n {
                        if at >= units.len() {
                            done = true;
                            break;
                        }
                        let pair = (0xD800..0xDC00).contains(&units[at])
                            && units
                                .get(at + 1)
                                .is_some_and(|u| (0xDC00..0xE000).contains(u));
                        at += if pair { 2 } else { 1 };
                    }
                    (at as u64, done)
                } else {
                    let o = target.as_object().ok_or("$dump.advance: not an object")?;
                    let len = match h.object_class(o) {
                        ObjectClass::Array => u64::from(h.array_length(o)),
                        ObjectClass::TypedArray => u64::from(
                            h.ta_length(o)
                                .ok_or("$dump.advance: a detached typed array")?,
                        ),
                        // An array-like (an arguments object): its data `length`.
                        _ => match h.get_own_property(o, atoms::LENGTH) {
                            Some(Property::Data { value, .. }) => value
                                .as_number()
                                .filter(|l| *l >= 0.0)
                                .map_or(0, |l| l as u64),
                            _ => {
                                return Err(
                                    "$dump.advance: an array-like without a data length".into()
                                );
                            }
                        },
                    };
                    // `next` at the end marks the iterator done.
                    (
                        index.saturating_add(n).min(len),
                        index.saturating_add(n) > len,
                    )
                };
                // An int where it fits, as `next` keeps it (readers take the int).
                let at = match i32::try_from(at) {
                    Ok(i) => Value::int(i),
                    Err(_) => Value::number(at as f64),
                };
                h.set_object_slot(it, 1, at);
                if done {
                    h.set_object_slot(it, 0, Value::UNDEFINED);
                }
            }
            _ => return Err("$dump.advance: not an array, string, Map or Set iterator".into()),
        }
        Ok(Value::object(it))
    }

    pub(super) fn primitive_from_sync(&mut self, args: &[Value]) -> Result<Value, String> {
        let it = args.first().copied().unwrap_or(Value::UNDEFINED);
        if it.as_object().is_none() {
            return Err("$dump.fromSync: not an iterator".into());
        }
        let next = args.get(1).copied().unwrap_or(Value::UNDEFINED);
        let proto = intrinsic_obj(&self.heap, Intrinsic::AsyncFromSyncIteratorProto);
        let r = self
            .heap
            .alloc_object(ObjectClass::Record, Some(proto), 2)
            .map_err(oom)?;
        self.heap.set_object_slot(r, 0, it);
        self.heap.set_object_slot(r, 1, next);
        Ok(Value::object(r))
    }

    pub(super) fn primitive_close_throw(&mut self, args: &[Value]) -> Result<Value, String> {
        let it = args.first().copied().unwrap_or(Value::UNDEFINED);
        if it.as_object().is_none() {
            return Err("$dump.closeThrow: not an iterator".into());
        }
        let f = new_builtin_closure(&mut self.heap, FROM_SYNC_CLOSE, it, 1)
            .map_err(oom)?
            .ok_or("$dump.closeThrow: internal")?;
        Ok(Value::object(f))
    }

    /// `$dump.using(resources, state)`: a `using` resource Record. The disposal code
    /// trusts its shape, so it is checked: triples with kinds 0-4, booleans, an index.
    pub(super) fn primitive_using(&mut self, args: &[Value]) -> Result<Value, String> {
        let what = "$dump.using";
        let res = args.first().copied().unwrap_or(Value::UNDEFINED);
        let a = res
            .as_object()
            .filter(|&o| self.heap.object_class(o) == ObjectClass::Array)
            .ok_or("$dump.using: the resources are not an array")?;
        let items = array_arg(&self.heap, res, what)?;
        let kind_ok = |v: &Value| v.as_int().is_some_and(|k| (0..=4).contains(&k));
        if items.len() % 3 != 0 || !items.chunks(3).all(|t| kind_ok(&t[2])) {
            return Err("$dump.using: the resources are not [value, method, kind] triples".into());
        }
        let r = self
            .heap
            .alloc_object(ObjectClass::Record, None, USING_SLOTS)
            .map_err(oom)?;
        for i in 0..USING_SLOTS {
            self.heap.set_object_slot(r, i, Value::UNDEFINED);
        }
        self.heap.set_object_slot(r, 2, Value::object(a));
        let state = args.get(1).copied().unwrap_or(Value::UNDEFINED);
        for (k, v) in entries_arg(&self.heap, state, what)? {
            let slot = USING_FIELDS
                .iter()
                .find(|f| f.0 == k)
                .map(|f| f.1)
                .ok_or_else(|| cat!("$dump.using: unknown key ", k))?;
            let ok = match slot {
                4 | 5 | 6 | 10 => v.as_bool().is_some(),
                // The next triple's index, -3 past the first.
                7 => v
                    .as_int()
                    .is_some_and(|i| i >= -3 && i % 3 == 0 && i64::from(i) < items.len() as i64),
                _ => true,
            };
            if !ok {
                return Err(cat!("$dump.using: a bad ", k));
            }
            self.heap.set_object_slot(r, slot, v);
        }
        Ok(Value::object(r))
    }

    /// A resolving function whose resolution is claimed already: calling it does
    /// nothing, as for a settled promise's.
    pub(super) fn primitive_spent(&mut self, args: &[Value]) -> Result<Value, String> {
        let kind = match args.first().and_then(|v| v.as_int()) {
            Some(k @ (0 | 1)) => k as usize,
            _ => return Err("$dump.spent: not 0 (resolve) or 1 (reject)".into()),
        };
        let r = self
            .heap
            .alloc_object(ObjectClass::Record, None, 2)
            .map_err(oom)?;
        self.heap.set_object_slot(r, 0, Value::UNDEFINED);
        self.heap.set_object_slot(r, 1, Value::TRUE);
        let f = new_builtin_closure(&mut self.heap, kind, Value::object(r), 1)
            .map_err(oom)?
            .ok_or("$dump.spent: internal")?;
        Ok(Value::object(f))
    }

    pub(super) fn primitive_random(&mut self, args: &[Value]) -> Result<Value, String> {
        let half = |i: usize| {
            args.get(i)
                .and_then(|v| v.as_int())
                .map(|h| h as u32 as u64)
        };
        let (Some(lo), Some(hi)) = (half(0), half(1)) else {
            return Err("$dump.random: not two int32 halves".into());
        };
        crate::builtins::number::set_math_seed(&mut self.heap, lo | hi << 32);
        Ok(Value::UNDEFINED)
    }

    pub(super) fn primitive_rejected(&mut self, args: &[Value]) -> Result<Value, String> {
        let e = args.first().copied().unwrap_or(Value::UNDEFINED);
        let proto = intrinsic_obj(&self.heap, Intrinsic::PromiseProto);
        let p = promise::alloc(&mut self.heap, Some(proto)).map_err(oom)?;
        promise::mark_handled(&mut self.heap, p);
        promise::settle(&mut self.heap, p, false, e).map_err(oom)?;
        Ok(Value::object(p))
    }

    #[cfg(feature = "full")]
    pub(super) fn primitive_bytes(&mut self, args: &[Value]) -> Result<Value, String> {
        let units: Vec<u16> = args
            .first()
            .and_then(|v| v.as_string())
            .map(|s| self.heap.str_units(s).collect())
            .ok_or("$dump.bytes: not a string")?;
        let bytes = crate::builtins::dom::atob(&units).ok_or("$dump.bytes: not base64")?;
        let bp = intrinsic_obj(&self.heap, Intrinsic::ArrayBufferProto);
        let tp = intrinsic_obj(&self.heap, Intrinsic::Uint8ArrayProto);
        let view = self
            .heap
            .new_uint8_array(&bytes, Some(bp), Some(tp))
            .map_err(oom)?;
        Ok(self.heap.object_slot(view, 0))
    }

    pub(super) fn primitive_fn(&mut self, args: &[Value]) -> Result<Value, String> {
        let o = args
            .first()
            .and_then(|v| v.as_object())
            .ok_or("$dump.fn: not an object")?;
        let k = *self
            .heap
            .own_keys(o)
            .first()
            .ok_or("$dump.fn: an empty object")?;
        let f = match self.heap.get_own_property(o, k) {
            Some(Property::Data { value, .. }) => value,
            Some(Property::Accessor { getter, setter, .. }) => {
                if getter.is_undefined() {
                    setter
                } else {
                    getter
                }
            }
            None => Value::UNDEFINED,
        };
        Ok(f)
    }

    /// Deletes the own properties of `o` (a class or its prototype, as the class's text
    /// made them) that are not in `keys`, besides `length`, `name`, `prototype` and
    /// `constructor`; every key must be there.
    pub(super) fn primitive_keep(&mut self, args: &[Value]) -> Result<Value, String> {
        let o = args
            .first()
            .and_then(|v| v.as_object())
            .ok_or("$dump.keep: not an object")?;
        let given = array_arg(
            &self.heap,
            args.get(1).copied().unwrap_or(Value::UNDEFINED),
            "$dump.keep",
        )?;
        let mut keys: Vec<Atom> = Vec::new();
        for v in given {
            let k = if let Some(s) = v.as_symbol() {
                self.heap.symbol_atom(s)
            } else if let Some(s) = v.as_string() {
                let name = self.heap.str_to_string(s);
                self.heap.atom_str(&name).map_err(oom)?
            } else {
                return Err("$dump.keep: a key that is not a string or symbol".into());
            };
            keys.push(k);
        }
        let fixed = [
            atoms::LENGTH,
            atoms::NAME,
            atoms::PROTOTYPE,
            atoms::CONSTRUCTOR,
        ];
        for k in self.heap.own_keys(o) {
            if fixed.contains(&k) || keys.contains(&k) {
                continue;
            }
            if !self.heap.delete_own_property(o, k).map_err(oom)? {
                let name = self.heap.atom_name(k);
                return Err(cat!("$dump.keep: cannot delete ", name));
            }
        }
        for k in keys {
            if self.heap.get_own_property(o, k).is_none() {
                let name = self.heap.atom_name(k);
                return Err(cat!("$dump.keep: the class does not make ", name));
            }
        }
        Ok(Value::UNDEFINED)
    }
}
