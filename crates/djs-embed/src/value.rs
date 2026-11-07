//! Values across the boundary: [`Value`] goes in (call arguments, host results),
//! [`ValueRef`] reads one out (host call arguments, a call's result).

use std::borrow::Cow;

use djs_core::heap::{Heap, HeapError, ObjectClass, StrView};
use djs_core::interp::{Intrinsic, realm};
use djs_core::value::Value as Raw;
use djs_core::vm::Vm;
use djs_core::wire::{EncodeError, WireError};

use crate::{Data, Error};

/// A value the host gives the VM: a call argument or a host function's result. Strings and
/// bytes are copied straight into the VM's heap (one copy; ASCII strings as a memcpy).
/// The heap is a single arena the VM owns, so a `Uint8Array` cannot wrap host-owned bytes:
/// each `Bytes` is a copy.
#[derive(Clone, Copy, Debug)]
pub enum Value<'a> {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    Str(&'a str),
    /// A new `Uint8Array` with a copy of the bytes
    Bytes(&'a [u8]),
    /// Plain data, built in the VM
    Data(&'a Data),
    /// A wire buffer (docs/wire.md), decoded in the VM
    Wire(&'a [u8]),
}

impl From<bool> for Value<'_> {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<f64> for Value<'_> {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl From<i32> for Value<'_> {
    fn from(n: i32) -> Self {
        Value::Number(n.into())
    }
}

impl From<u32> for Value<'_> {
    fn from(n: u32) -> Self {
        Value::Number(n.into())
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Value::Str(s)
    }
}

impl<'a> From<&'a String> for Value<'a> {
    fn from(s: &'a String) -> Self {
        Value::Str(s)
    }
}

impl<'a> From<&'a [u8]> for Value<'a> {
    fn from(b: &'a [u8]) -> Self {
        Value::Bytes(b)
    }
}

impl<'a> From<&'a Data> for Value<'a> {
    fn from(d: &'a Data) -> Self {
        Value::Data(d)
    }
}

/// Integral numbers that fit are the VM's int form (as its decoders make them).
fn number(n: f64) -> Raw {
    let i = n as i32;
    if i as f64 == n && !(i == 0 && n.is_sign_negative()) {
        Raw::int(i)
    } else {
        Raw::number(n)
    }
}

fn heap_error(e: HeapError) -> Error {
    match e {
        HeapError::OutOfMemory | HeapError::HostOutOfMemory => Error::OutOfMemory,
        e => Error::Data(e.as_str().to_owned()),
    }
}

pub(crate) fn wire_error(e: WireError) -> Error {
    match e {
        WireError::Heap(e) => heap_error(e),
        e => Error::Data(e.text()),
    }
}

/// Makes `v` in the VM. Allocates but never collects: values made earlier stay valid.
pub(crate) fn to_raw(vm: &mut Vm, v: &Value<'_>) -> Result<Raw, Error> {
    Ok(match *v {
        Value::Undefined => Raw::UNDEFINED,
        Value::Null => Raw::NULL,
        Value::Bool(b) => Raw::bool(b),
        Value::Number(n) => number(n),
        Value::Str(s) => Raw::string(vm.heap_mut().alloc_str(s).map_err(heap_error)?),
        Value::Bytes(b) => {
            let heap = vm.heap_mut();
            let bp = realm::intrinsic_obj(heap, Intrinsic::ArrayBufferProto);
            let p = realm::intrinsic_obj(heap, Intrinsic::Uint8ArrayProto);
            Raw::object(
                heap.new_uint8_array(b, Some(bp), Some(p))
                    .map_err(heap_error)?,
            )
        }
        Value::Data(d) => vm.decode_wire(&d.to_wire()).map_err(wire_error)?,
        Value::Wire(b) => vm.decode_wire(b).map_err(wire_error)?,
    })
}

/// A VM value, read without running JS: a host call's argument or a call's result. It
/// borrows the instance, which cannot run while one is alive.
#[derive(Clone, Copy)]
pub struct ValueRef<'a> {
    pub(crate) heap: &'a Heap,
    pub(crate) raw: Raw,
    /// The cap of [`ValueRef::to_wire`] and [`ValueRef::to_data`] in `bytes`
    pub(crate) max_bytes: usize,
}

impl std::fmt::Debug for ValueRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_data() {
            Ok(d) => write!(f, "ValueRef({d:?})"),
            Err(_) => write!(f, "ValueRef({})", self.type_name()),
        }
    }
}

impl<'a> ValueRef<'a> {
    pub fn is_undefined(&self) -> bool {
        self.raw.is_undefined()
    }

    pub fn is_null(&self) -> bool {
        self.raw.is_null()
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.raw.as_bool()
    }

    pub fn as_number(&self) -> Option<f64> {
        self.raw.as_number()
    }

    /// A string as UTF-8: borrowed from the heap when it is ASCII in one piece, else
    /// converted (lone surrogates become U+FFFD).
    pub fn as_str(&self) -> Option<Cow<'a, str>> {
        let s = self.raw.as_string()?;
        if let Some(StrView::Latin1(b)) = self.heap.str_view(s)
            && b.is_ascii()
        {
            return std::str::from_utf8(b).ok().map(Cow::Borrowed);
        }
        Some(Cow::Owned(self.heap.str_to_string(s)))
    }

    /// The bytes of a `Uint8Array` or `ArrayBuffer`, borrowed from the heap (empty once
    /// detached).
    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        let o = self.raw.as_object()?;
        let heap = self.heap;
        match heap.object_class(o) {
            ObjectClass::ArrayBuffer => Some(heap.buffer_data(o)),
            ObjectClass::TypedArray if heap.ta_kind(o) == djs_core::heap::typed::Kind::Uint8 => {
                let n = heap.ta_length(o).unwrap_or(0) as usize;
                let at = heap.ta_byte_offset(o) as usize;
                Some(
                    heap.buffer_data(heap.ta_buffer(o))
                        .get(at..at + n)
                        .unwrap_or(&[]),
                )
            }
            _ => None,
        }
    }

    /// The plain data as a wire buffer (docs/wire.md). Refuses what is not plain data
    /// ([`Error::Data`] naming the path): functions, accessors, proxies, class instances,
    /// symbols, BigInts and every object other than arrays, plain objects and
    /// `Uint8Array`s, and cycles.
    pub fn to_wire(&self) -> Result<Vec<u8>, Error> {
        djs_core::wire::encode(self.heap, self.raw, self.max_bytes)
            .map_err(|e: EncodeError| Error::Data(e.text()))
    }

    /// The plain data as a [`Data`] tree (through the wire form; the same refusals as
    /// [`ValueRef::to_wire`]).
    pub fn to_data(&self) -> Result<Data, Error> {
        Data::from_wire(&self.to_wire()?)
    }

    /// The JS type: `"undefined"`, `"object"`, `"function"`, ... as `typeof` says.
    pub fn type_name(&self) -> &'static str {
        use djs_core::value::Kind;
        match self.raw.kind() {
            Kind::Undefined => "undefined",
            Kind::Null => "object",
            Kind::Bool => "boolean",
            Kind::Number => "number",
            Kind::String => "string",
            Kind::Symbol => "symbol",
            Kind::BigInt => "bigint",
            Kind::Object => {
                let o = self.raw.as_object().expect("an object");
                if self.heap.is_callable(o) {
                    "function"
                } else {
                    "object"
                }
            }
            _ => "undefined",
        }
    }
}
