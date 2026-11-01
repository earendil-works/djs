//! The JSON and RegExp leaf intrinsics (0033), dispatched from `call_intrinsic` for the
//! ids its own table does not handle. Kept apart so the big match there stays someone
//! else's merge conflict.

use super::intrinsics::{bad_argument, id};
use super::*;
use crate::builtins::json;

const JSON_PARSE: u16 = id("JsonParse");
const JSON_STRINGIFY_FAST: u16 = id("JsonStringifyFast");
const JSON_QUOTE: u16 = id("JsonQuote");
const CREATE_DATA_PROPERTY: u16 = id("CreateDataProperty");

/// Fuel for work proportional to a string's length: one unit per 16 code units (the
/// leaves run to completion, so they pay afterwards).
fn length_fuel(len: u32) -> i64 {
    (len / 16) as i64
}

impl Exec<'_> {
    /// `call_intrinsic` for the JSON and RegExp intrinsics; `base` is the first
    /// argument's slot. Anything else is not supported.
    pub(super) fn json_regexp_intrinsic(
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
            JSON_PARSE => {
                let Some(s) = a0.as_string() else {
                    return bad_argument(name);
                };
                self.fuel -= length_fuel(self.heap.str_len(s));
                json::parse(self.heap, s)?
            }
            JSON_STRINGIFY_FAST => {
                let Some(gap) = arg(self, 1).as_string() else {
                    return bad_argument(name);
                };
                let r = json::stringify_fast(self.heap, a0, gap)?;
                if let Some(s) = r.as_string() {
                    self.fuel -= length_fuel(self.heap.str_len(s));
                }
                r
            }
            JSON_QUOTE => {
                let Some(s) = a0.as_string() else {
                    return bad_argument(name);
                };
                self.fuel -= length_fuel(self.heap.str_len(s));
                Value::string(json::quote(self.heap, s)?)
            }
            CREATE_DATA_PROPERTY => {
                let Some(o) = a0.as_object() else {
                    return bad_argument(name);
                };
                let key = conv::to_property_key(self.heap, arg(self, 1))?;
                Value::bool(self.heap.create_data_property(o, key, arg(self, 2))?)
            }
            _ => return self.bigint_intrinsic(id, name, base, argc, next),
        };
        self.sp = base;
        self.push(v);
        self.pc = next;
        Ok(None)
    }
}
