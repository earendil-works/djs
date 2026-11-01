//! The JSON and RegExp leaf intrinsics (0033), dispatched from `call_intrinsic` for the
//! ids its own table does not handle. Kept apart so the big match there stays someone
//! else's merge conflict.

use super::intrinsics::{bad_argument, id};
use super::*;
use crate::builtins::{json, regexp as re};
use crate::regexp::{ExecLimits, ExecResult, ExecState, Flags};

const JSON_PARSE: u16 = id("JsonParse");
const JSON_STRINGIFY_FAST: u16 = id("JsonStringifyFast");
const JSON_QUOTE: u16 = id("JsonQuote");
const CREATE_DATA_PROPERTY: u16 = id("CreateDataProperty");
const IS_REGEXP_OBJECT: u16 = id("IsRegExpObject");
const REGEXP_CREATE: u16 = id("RegExpCreate");
const REGEXP_SOURCE: u16 = id("RegExpSource");
const REGEXP_FLAGS: u16 = id("RegExpFlags");
const REGEXP_HAS_FLAG: u16 = id("RegExpHasFlag");
const REGEXP_ESCAPED_SOURCE: u16 = id("RegExpEscapedSource");
const REGEXP_EXEC_RAW: u16 = id("RegExpExecRaw");
const CREATE_REGEXP_STRING_ITERATOR: u16 = id("CreateRegExpStringIterator");
const REGEXP_STRING_ITERATOR_SLOT: u16 = id("RegExpStringIteratorSlot");
const REGEXP_STRING_ITERATOR_FINISH: u16 = id("RegExpStringIteratorFinish");

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
            IS_REGEXP_OBJECT => Value::bool(re::as_regexp(self.heap, a0).is_some()),
            REGEXP_CREATE => {
                let proto = a0.as_object();
                let (Some(p), Some(f)) = (arg(self, 1).as_string(), arg(self, 2).as_string())
                else {
                    return bad_argument(name);
                };
                Value::object(re::create(self.heap, proto, p, f)?)
            }
            REGEXP_SOURCE | REGEXP_FLAGS | REGEXP_HAS_FLAG | REGEXP_ESCAPED_SOURCE => {
                let Some(r) = re::as_regexp(self.heap, a0) else {
                    return bad_argument(name);
                };
                match id {
                    REGEXP_SOURCE => Value::string(re::source(self.heap, r)),
                    REGEXP_FLAGS => Value::string(re::flags_string(self.heap, r)?),
                    REGEXP_ESCAPED_SOURCE => Value::string(re::escaped_source(self.heap, r)?),
                    _ => {
                        let Some(c) = arg(self, 1)
                            .as_string()
                            .and_then(|f| self.heap.char_code_at(f, 0))
                        else {
                            return bad_argument(name);
                        };
                        Value::bool(re::has_flag(self.heap, r, c))
                    }
                }
            }
            REGEXP_EXEC_RAW if argc == 4 => return self.regexp_exec_raw(name, base, next),
            CREATE_REGEXP_STRING_ITERATOR => {
                let (Some(r), Some(s)) = (a0.as_object(), arg(self, 1).as_string()) else {
                    return bad_argument(name);
                };
                let mut flags = 0;
                if arg(self, 2).as_bool() == Some(true) {
                    flags |= re::IT_GLOBAL;
                }
                if arg(self, 3).as_bool() == Some(true) {
                    flags |= re::IT_UNICODE;
                }
                Value::object(re::string_iterator(self.heap, r, s, flags)?)
            }
            REGEXP_STRING_ITERATOR_SLOT => {
                let it = re::as_string_iterator(self.heap, a0)?;
                let flags = self.heap.object_slot(it, re::IT_FLAGS).as_int().unwrap_or(0);
                match arg(self, 1).as_int() {
                    Some(0) => self.heap.object_slot(it, re::IT_REGEXP),
                    Some(1) => self.heap.object_slot(it, re::IT_STRING),
                    Some(2) => Value::bool(flags & re::IT_GLOBAL != 0),
                    Some(3) => Value::bool(flags & re::IT_UNICODE != 0),
                    Some(4) => Value::bool(flags & re::IT_DONE != 0),
                    _ => return bad_argument(name),
                }
            }
            REGEXP_STRING_ITERATOR_FINISH => {
                let it = re::as_string_iterator(self.heap, a0)?;
                let flags = self.heap.object_slot(it, re::IT_FLAGS).as_int().unwrap_or(0);
                self.heap
                    .set_object_slot(it, re::IT_FLAGS, Value::int(flags | re::IT_DONE));
                Value::UNDEFINED
            }
            _ => return self.bigint_intrinsic(id, name, base, argc, next),
        };
        self.sp = base;
        self.push(v);
        self.pc = next;
        Ok(None)
    }

    /// `%RegExpExecRaw(r, s, lastIndex, state)`: RegExpBuiltinExec after `lastIndex` was
    /// read (ToLength), D1332. Runs the matcher with the remaining fuel. When the fuel
    /// runs out mid-match, the matcher's state goes into a U32Array in the `state`
    /// argument slot (callers pass `undefined`) and `run` returns `OutOfFuel` with `pc`
    /// still at this instruction, so the next `run` executes it again and resumes the
    /// match from that state. Everything is in the arena: a heap image taken while
    /// suspended resumes the same match.
    fn regexp_exec_raw(&mut self, name: &str, base: u32, next: u32) -> Step {
        let state_slot = base + 3;
        let (Some(r), Some(s), Some(last)) = (
            re::as_regexp(self.heap, self.get(base)),
            self.get(base + 1).as_string(),
            self.get(base + 2).as_number(),
        ) else {
            return bad_argument(name);
        };
        let flags = re::flags(self.heap, r);
        let g_or_y = flags.contains(Flags::GLOBAL) || flags.contains(Flags::STICKY);
        let last = if g_or_y { last } else { 0.0 };
        let flat = self.heap.flatten(s)?;
        let len = self.heap.str_len(flat) as f64;
        let v = if last > len {
            if g_or_y {
                re::set_last_index(self.heap, r, Value::int(0))?;
            }
            Value::NULL
        } else {
            let prog = re::program(self.heap, r);
            let pending = match self.get(state_slot).as_internal() {
                Some(cell) => {
                    let n = self.heap.u32_array_len(cell);
                    let words: alloc::vec::Vec<u32> =
                        (0..n).map(|i| self.heap.u32_array_get(cell, i)).collect();
                    match ExecState::from_words(&words) {
                        Some(st) => Some(st),
                        None => return bad_argument(name),
                    }
                }
                None => None,
            };
            let budget = self.fuel.max(1) as u64;
            let mut fuel = budget;
            let limits = ExecLimits::default();
            let result = {
                let view = self.heap.str_view(flat).expect("flat string");
                let input = re::input(view);
                match pending {
                    Some(st) => crate::regexp::resume(&prog, input, st, &mut fuel, limits),
                    None => crate::regexp::exec(&prog, input, last as usize, &mut fuel, limits),
                }
            };
            self.fuel -= (budget - fuel) as i64;
            match result {
                ExecResult::OutOfFuel(st) => {
                    let words = st.to_words();
                    let cell = self.heap.alloc_u32_array(words.len() as u32)?;
                    for (i, &w) in words.iter().enumerate() {
                        self.heap.u32_array_set(cell, i as u32, w);
                    }
                    self.set(state_slot, Value::internal(cell));
                    self.flush();
                    return Ok(Some(Status::OutOfFuel));
                }
                ExecResult::StackOverflow => {
                    return Err(Fault::Error(
                        ErrorKind::InternalError,
                        String::from("out of memory in regexp execution"),
                    ));
                }
                ExecResult::NoMatch => {
                    if g_or_y {
                        re::set_last_index(self.heap, r, Value::int(0))?;
                    }
                    Value::NULL
                }
                ExecResult::Match(caps) => {
                    if g_or_y {
                        let (_, end) = caps.get(0).expect("group 0 matched");
                        re::set_last_index(self.heap, r, Value::int(end as i32))?;
                    }
                    re::match_result(self.heap, &prog, s, &caps)?
                }
            }
        };
        self.sp = base;
        self.push(v);
        self.pc = next;
        Ok(None)
    }
}
