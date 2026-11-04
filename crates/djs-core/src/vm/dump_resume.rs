//! `$dump.resume(fn, {...})` (0171, docs/continuation-dumps.md, D2610): a suspended
//! activation of `fn` built from its description, at a site named by this build's
//! suspension map (D2563).
//!
//! ```js
//! $dump.resume(fn, {
//!   at: "await#2",              // the site, by kind and number in fn's text
//!   awaiting: p,                // an await's promise; none at a yield
//!   this: o, args: [...],       // the call's `this` and arguments
//!   slots: {                    // the frame's slots by name (D2721, D2731)
//!     x: 1, "y@40": $dump.TDZ, seen: $dump.cell(() => seen),
//!     "call.callee@41": f, "call.this@41": o,
//!     "for-of.iterator@18": it, "for-of.next@18": nextFn,
//!   },
//!   queue: [{ type: "next", value: v, promise: q }],   // an async generator's requests
//! })
//! ```
//!
//! `fn` must be made by the dump's own text: its map comes from compiling that text
//! again with recording on, so the slots and the resume pc are those of `fn`'s code.
//! Every slot the map names at the site must be given and nothing else
//! ([`crate::compiler::Site::named_slots`]): the bindings in scope by name (`name@scope`
//! where two of a name are in scope), the temporaries of the constructs in flight by
//! `construct@offset`. Each is written into the frame index this build's map gives its
//! name, so nothing depends on the order of the keys or on where this build keeps the
//! value (D2731). Hidden locals are made by the loader (`<state>`, `this`,
//! `new.target`, `<active>`, `<home>`) or given in `slots` (`arguments`). A captured
//! binding gets a new Box, or the one a `$dump.cell` names.

use alloc::string::String;
use alloc::vec::Vec;

use super::Vm;
use super::dump_load::{Given as GivenValue, Load, array_arg, entries_arg, given, string_arg};
use crate::bytecode::template::{debug_info, func_flags};
use crate::compiler::{Holds, Place, SiteKind};
use crate::heap::cells::{BoxCell, FuncTemplate};
use crate::heap::{CellKind, Heap, HeapError, ObjectClass};
use crate::interp::frame;
use crate::interp::run::{promise, suspend};
use crate::value::{Obj, Value};

/// A slot given by the dump, until `resume` uses it.
struct Given {
    key: String,
    value: Value,
    used: bool,
}

fn oom(e: HeapError) -> String {
    String::from(e.as_str())
}

/// What goes in a slot for the value `v`: the value, `uninitialized` for `$dump.TDZ`, and
/// for a captured binding (`boxed`) a Box: a new one, or the one `$dump.cell` named.
fn slot_value(heap: &mut Heap, v: Value, boxed: bool, name: &str) -> Result<Value, String> {
    let v = match (given(heap, v), boxed) {
        (GivenValue::Cell(b), true) => return Ok(Value::internal(b)),
        (GivenValue::Cell(_), false) => {
            return Err(cat!(
                "$dump.resume: ",
                name,
                " is not captured by a closure: give its value, not a cell"
            ));
        }
        (GivenValue::Tdz, _) => Value::UNINITIALIZED,
        (GivenValue::Value(v), _) => v,
    };
    if !boxed {
        return Ok(v);
    }
    let c = heap
        .alloc_cell(CellKind::BoxCell, BoxCell::LAYOUT.min_size)
        .map_err(oom)?;
    heap.arena_mut().set(c, BoxCell::VALUE, v);
    Ok(Value::internal(c))
}

impl Vm {
    /// The description's keys, checked: unknown ones refuse.
    fn resume_desc(&self, d: Value) -> Result<[Value; 6], String> {
        const KEYS: [&str; 6] = ["at", "awaiting", "this", "args", "slots", "queue"];
        let mut out = [Value::UNDEFINED; 6];
        for (k, v) in entries_arg(&self.heap, d, "$dump.resume")? {
            let i = KEYS
                .iter()
                .position(|n| *n == k)
                .ok_or_else(|| cat!("$dump.resume: unknown key ", k))?;
            out[i] = v;
        }
        Ok(out)
    }

    /// The site `at` of closure `f` in the dump's suspension maps: the indexes of its map
    /// and of the site in it.
    fn resume_site(&mut self, f: Obj, at: &str, load: &mut Load) -> Result<(usize, usize), String> {
        let t = self.heap.closure_template(f);
        let info = debug_info(&self.heap, t);
        let script = self.heap.get_handle(load.script).as_object().map(|s| {
            let st = self.heap.closure_template(s);
            debug_info(&self.heap, st).script
        });
        if script != Some(info.script) {
            return Err("$dump.resume: the function is not made by the dump's text".into());
        }
        if load.maps.is_none() {
            let maps = crate::compiler::suspension_maps(
                &mut self.heap,
                load.text.as_bytes(),
                &super::dump_load::dump_options(load.typescript),
            )
            .map_err(|e| cat!("$dump.resume: ", e.message))?;
            load.maps = Some(maps);
        }
        let maps = load.maps.as_deref().unwrap_or_default();
        let mi = maps
            .iter()
            .position(|m| m.start == info.src_start && m.end == info.src_end)
            .ok_or("$dump.resume: the function has no suspension sites")?;
        let si = maps[mi]
            .sites
            .iter()
            .position(|s| s.name == at)
            .ok_or_else(|| cat!("$dump.resume: ", maps[mi].name, " has no site ", at))?;
        if let Some(u) = &maps[mi].sites[si].unsupported {
            return Err(cat!("$dump.resume: ", at, " cannot be resumed: ", u));
        }
        Ok((mi, si))
    }

    /// `$dump.resume(fn, description)`: the activation's result promise (an async
    /// function) or its generator object.
    pub(super) fn primitive_resume(
        &mut self,
        args: &[Value],
        load: &mut Load,
    ) -> Result<Value, String> {
        let f = args
            .first()
            .and_then(|v| v.as_object())
            .filter(|&o| self.heap.object_class(o) == ObjectClass::Function)
            .ok_or("$dump.resume: not a function")?;
        let t = self.heap.closure_template(f);
        let a = self.heap.arena();
        let flags = u32::from(a.get::<u16>(t, FuncTemplate::FLAGS));
        let params = usize::from(a.get::<u16>(t, FuncTemplate::PARAMS));
        let n_locals = usize::from(a.get::<u16>(t, FuncTemplate::LOCALS));
        let captures = u32::from(a.get::<u16>(t, FuncTemplate::CAPTURES));
        let kind = match (
            flags & func_flags::ASYNC != 0,
            flags & func_flags::GENERATOR != 0,
        ) {
            (true, false) => suspend::ASYNC,
            (false, true) => suspend::GENERATOR,
            (true, true) => suspend::ASYNC_GENERATOR,
            _ => return Err("$dump.resume: not an async function or a generator".into()),
        };
        let [at, awaiting, this, call_args, slots, queue] =
            self.resume_desc(args.get(1).copied().unwrap_or(Value::UNDEFINED))?;
        let at = string_arg(&self.heap, at, "$dump.resume: at")?;
        let (mi, si) = self.resume_site(f, &at, load)?;
        let site = &load.maps.as_deref().unwrap_or_default()[mi].sites[si];
        if site.args.len() != params || site.locals.len() != n_locals {
            return Err(cat!(
                "$dump.resume: the map of ",
                at,
                " does not fit the function"
            ));
        }
        let at_yield = matches!(
            site.kind,
            SiteKind::Yield | SiteKind::AsyncYield | SiteKind::YieldStar
        );
        let awaited = promise::as_promise(&self.heap, awaiting);
        match (at_yield, awaited) {
            (true, _) if !awaiting.is_undefined() => {
                return Err(cat!(
                    "$dump.resume: ",
                    at,
                    " is a yield: nothing is awaited"
                ));
            }
            (false, None) => {
                return Err(cat!("$dump.resume: ", at, " awaits a promise: give it"));
            }
            _ => {}
        }
        let call_args = array_arg(&self.heap, call_args, "$dump.resume: args")?;
        let mut given: Vec<Given> = entries_arg(&self.heap, slots, "$dump.resume: slots")?
            .into_iter()
            .map(|(key, value)| Given {
                key,
                value,
                used: false,
            })
            .collect();

        let g = suspend::new_activation(&mut self.heap, f, kind).map_err(oom)?;
        let argc = call_args.len();
        let nargs = argc.max(params);
        let n_temps = site.operands.len();
        // The frame, written by place: callee, `this`, the arguments, the header, the
        // locals, the temporaries. What no slot names keeps its default: the call's
        // arguments, `undefined`.
        let hp_off = 2 + nargs;
        let lp = hp_off + frame::HEADER as usize;
        let mut frame = alloc::vec![Value::UNDEFINED; lp + n_locals + n_temps];
        frame[0] = Value::object(f);
        frame[1] = this;
        for (i, v) in frame[2..hp_off].iter_mut().enumerate() {
            *v = call_args.get(i).copied().unwrap_or(Value::UNDEFINED);
        }
        let mut header = [Value::int(0); frame::HEADER as usize];
        header[frame::ARGC as usize] = Value::int(argc as i32);
        header[frame::NEW_TARGET as usize] = Value::UNDEFINED;
        frame[hp_off..lp].copy_from_slice(&header);
        for slot in site.named_slots() {
            let found = given.iter_mut().find(|b| b.key == slot.name).map(|b| {
                b.used = true;
                b.value
            });
            let v = match (slot.holds, found) {
                (Holds::Hidden, _) if slot.name == "<state>" => Value::object(g),
                (Holds::Hidden, _) if slot.name == "this" => this,
                (_, Some(v)) => v,
                (Holds::Hidden, None) => match slot.name.as_str() {
                    "new.target" => Value::UNDEFINED,
                    "<active>" => Value::object(f),
                    "<home>" if flags & func_flags::HOME != 0 => self.heap.object_slot(f, captures),
                    n => {
                        return Err(cat!("$dump.resume: give the hidden local ", n, " in slots"));
                    }
                },
                (_, None) => {
                    return Err(cat!(
                        "$dump.resume: ",
                        at,
                        " has a slot ",
                        slot.name,
                        ": give it"
                    ));
                }
            };
            // A temporary's value as the frame holds it.
            let v = match (slot.construct(), slot.holds) {
                ("for-in.iterator", _) => super::dump_prims::for_in_iterator(&self.heap, v)
                    .ok_or_else(|| {
                        cat!("$dump.resume: a for-in loop's iterator is made by $dump.forIn")
                    })?,
                _ => v,
            };
            let index = match slot.place {
                Place::Arg(i) => 2 + usize::from(i),
                Place::Local(i) => lp + usize::from(i),
                Place::Temp(k) => lp + n_locals + usize::from(k),
            };
            let v = match slot.holds {
                Holds::Binding | Holds::Hidden => {
                    slot_value(&mut self.heap, v, slot.boxed, &slot.name)?
                }
                Holds::Temp(_) => v,
            };
            frame[index] = v;
        }
        if let Some(b) = given.iter().find(|b| !b.used) {
            return Err(cat!("$dump.resume: ", at, " has no slot ", b.key));
        }
        let hp_off = hp_off as u32;
        let state = if at_yield && kind == suspend::GENERATOR && site.kind == SiteKind::YieldStar {
            // A sync generator's `yield_star` passes the inner result through.
            suspend::SUSPENDED_RAW
        } else if at_yield {
            suspend::SUSPENDED
        } else {
            suspend::AWAITING
        };
        suspend::set_suspended(&mut self.heap, g, &frame, hp_off, site.resume_pc, state)
            .map_err(oom)?;
        self.resume_queue(g, kind, queue)?;
        if let Some(p) = awaited {
            // As `%AsyncAwait` subscribes it: the activation is the reaction's handler.
            promise::perform_then(
                &mut self.heap,
                p,
                Value::object(g),
                Value::object(g),
                Value::UNDEFINED,
            )
            .map_err(oom)?;
        }
        Ok(if kind == suspend::ASYNC {
            suspend::promise_of(&self.heap, g)
        } else {
            Value::object(g)
        })
    }

    /// An async generator's requests (D1421): `[{type, value, promise}]`, oldest first,
    /// each promise pending (the one its caller got).
    fn resume_queue(&mut self, g: Obj, kind: i32, queue: Value) -> Result<(), String> {
        let entries = array_arg(&self.heap, queue, "$dump.resume: queue")?;
        if !entries.is_empty() && kind != suspend::ASYNC_GENERATOR {
            return Err("$dump.resume: only async generators have a request queue".into());
        }
        for e in entries {
            let mut rec = [Value::UNDEFINED; 3];
            for (k, v) in entries_arg(&self.heap, e, "$dump.resume: request")? {
                match k.as_str() {
                    "type" => {
                        let t = string_arg(&self.heap, v, "$dump.resume: request type")?;
                        let n = ["next", "throw", "return"]
                            .iter()
                            .position(|x| *x == t)
                            .ok_or_else(|| cat!("$dump.resume: no request type ", t))?;
                        rec[0] = Value::int(n as i32);
                    }
                    "value" => rec[1] = v,
                    "promise" => rec[2] = v,
                    _ => return Err(cat!("$dump.resume: unknown request key ", k)),
                }
            }
            let pending = promise::as_promise(&self.heap, rec[2]).is_some_and(|p| {
                promise::state_of(&self.heap, p) == promise::PENDING
                    && promise::flags(&self.heap, p) & promise::FLAG_RESOLVED == 0
            });
            if rec[0].is_undefined() || !pending {
                return Err("$dump.resume: a request needs a type and a pending promise".into());
            }
            let r = self
                .heap
                .alloc_object(ObjectClass::Record, None, 3)
                .map_err(oom)?;
            for (i, v) in rec.into_iter().enumerate() {
                self.heap.set_object_slot(r, i as u32, v);
            }
            suspend::enqueue_request(&mut self.heap, g, Value::object(r)).map_err(oom)?;
        }
        Ok(())
    }
}
