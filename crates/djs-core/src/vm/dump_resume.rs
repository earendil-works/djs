//! `$dump.resume(fn, {...})` (0171, docs/continuation-dumps.md, D2610): a suspended
//! activation of `fn` built from its description, at a site named by this build's
//! suspension map (D2563).
//!
//! ```js
//! $dump.resume(fn, {
//!   at: "await#2",              // the site, by kind and number in fn's text
//!   awaiting: p,                // an await's promise; none at a yield
//!   this: o, args: [...],       // the call's `this` and arguments
//!   bindings: { x: 1, "y@40": $dump.TDZ, seen: $dump.cell(() => seen) },
//!   operands: [["call.callee", f], ["call.this", o]],   // bottom first
//!   loops: [{ kind: "for-of", iterator: it, next: nextFn }],   // outermost first
//!   queue: [{ type: "next", value: v, promise: q }],   // an async generator's requests
//! })
//! ```
//!
//! `fn` must be made by the dump's own text: its map comes from compiling that text
//! again with recording on, so the slots and the resume pc are those of `fn`'s code.
//! Everything the map has at the site must be given and nothing else: the bindings in
//! scope by name (`name@scope` where two of a name are in scope), the operands by role,
//! the loops by kind. Hidden locals are made by the loader (`<state>`, `this`,
//! `new.target`, `<active>`, `<home>`) or given in `bindings` (`arguments`). A captured
//! binding gets a new Box, or the one a `$dump.cell` names.

use alloc::string::String;
use alloc::vec::Vec;

use super::Vm;
use super::dump_load::{Given, Load, array_arg, entries_arg, given, string_arg};
use crate::bytecode::template::{debug_info, func_flags};
use crate::compiler::{Loop, Role, Site, SiteKind, Slot};
use crate::heap::cells::{BoxCell, FuncTemplate};
use crate::heap::{CellKind, Heap, HeapError, ObjectClass};
use crate::interp::frame;
use crate::interp::run::{promise, suspend};
use crate::value::{Obj, Value};

/// `role` as the map names it: `call.arg[2]`.
pub(super) fn role_name(role: Role) -> String {
    match role.parts() {
        (n, Some(i)) => cat!(n, "[", i, "]"),
        (n, None) => String::from(n),
    }
}

/// A binding given by the dump, until `resume` uses it.
struct Binding {
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
        (Given::Cell(b), true) => return Ok(Value::internal(b)),
        (Given::Cell(_), false) => {
            return Err(cat!(
                "$dump.resume: ",
                name,
                " is not captured by a closure: give its value, not a cell"
            ));
        }
        (Given::Tdz, _) => Value::UNINITIALIZED,
        (Given::Value(v), _) => v,
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
    fn resume_desc(&self, d: Value) -> Result<[Value; 8], String> {
        const KEYS: [&str; 8] = [
            "at", "awaiting", "this", "args", "bindings", "operands", "loops", "queue",
        ];
        let mut out = [Value::UNDEFINED; 8];
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
        let [
            at,
            awaiting,
            this,
            call_args,
            bindings,
            operands,
            loops,
            queue,
        ] = self.resume_desc(args.get(1).copied().unwrap_or(Value::UNDEFINED))?;
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
        let at_yield = matches!(site.kind, SiteKind::Yield | SiteKind::AsyncYield);
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
        let mut given_bindings: Vec<Binding> =
            entries_arg(&self.heap, bindings, "$dump.resume: bindings")?
                .into_iter()
                .map(|(key, value)| Binding {
                    key,
                    value,
                    used: false,
                })
                .collect();
        let mut lookup = |name: &str, scope: Option<u32>| -> Option<Value> {
            let b = given_bindings.iter_mut().find(|b| {
                b.key == name
                    || scope.is_some_and(|s| {
                        b.key.strip_prefix(name).and_then(|r| r.strip_prefix('@'))
                            == Some(cat!(s).as_str())
                    })
            })?;
            b.used = true;
            Some(b.value)
        };

        let g = suspend::new_activation(&mut self.heap, f, kind).map_err(oom)?;
        let argc = call_args.len();
        let nargs = argc.max(params);
        let mut frame = Vec::with_capacity(2 + nargs + frame::HEADER as usize + n_locals);
        frame.push(Value::object(f));
        frame.push(this);
        for i in 0..nargs {
            let v = match site.args.get(i) {
                Some(Slot::Binding { name, scope }) => {
                    let v = lookup(name, Some(*scope))
                        .ok_or_else(|| cat!("$dump.resume: no value for binding ", name))?;
                    slot_value(&mut self.heap, v, false, name)?
                }
                _ => call_args.get(i).copied().unwrap_or(Value::UNDEFINED),
            };
            frame.push(v);
        }
        let hp_off = frame.len() as u32;
        let mut header = [Value::int(0); frame::HEADER as usize];
        header[frame::ARGC as usize] = Value::int(argc as i32);
        header[frame::NEW_TARGET as usize] = Value::UNDEFINED;
        frame.extend_from_slice(&header);
        for (i, slot) in site.locals.iter().enumerate() {
            let boxed = site.boxed.get(i).copied().unwrap_or(false);
            let (name, v) = match slot {
                Slot::Binding { name, scope } => (
                    name,
                    lookup(name, Some(*scope))
                        .ok_or_else(|| cat!("$dump.resume: no value for binding ", name))?,
                ),
                Slot::Hidden(name) => {
                    let v = match name.as_str() {
                        "<state>" => Value::object(g),
                        "this" => this,
                        n => match lookup(n, None) {
                            Some(v) => v,
                            None => match n {
                                "new.target" => Value::UNDEFINED,
                                "<active>" => Value::object(f),
                                "<home>" if flags & func_flags::HOME != 0 => {
                                    self.heap.object_slot(f, captures)
                                }
                                _ => {
                                    return Err(cat!(
                                        "$dump.resume: give the hidden local ",
                                        n,
                                        " in bindings"
                                    ));
                                }
                            },
                        },
                    };
                    (name, v)
                }
                _ => {
                    frame.push(Value::UNDEFINED);
                    continue;
                }
            };
            let v = slot_value(&mut self.heap, v, boxed, name)?;
            frame.push(v);
        }
        if let Some(b) = given_bindings.iter().find(|b| !b.used) {
            return Err(cat!("$dump.resume: ", at, " has no binding ", b.key));
        }
        self.resume_operands(site, operands, loops, &mut frame)?;
        let state = if at_yield {
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

    /// Pushes the operands of `site`, bottom first: loop records from `loops`, the other
    /// roles from `operands`, each checked against the map.
    fn resume_operands(
        &self,
        site: &Site,
        operands: Value,
        loops: Value,
        frame: &mut Vec<Value>,
    ) -> Result<(), String> {
        let h = &self.heap;
        let ops = array_arg(h, operands, "$dump.resume: operands")?;
        let loops = array_arg(h, loops, "$dump.resume: loops")?;
        let (mut oi, mut li) = (0, 0);
        for &role in &site.operands {
            let v = match role {
                Role::LoopIterator(l) | Role::LoopNext(l) => {
                    if l == Loop::In {
                        return Err("$dump.resume: for-in loops cannot be resumed yet".into());
                    }
                    let lp = loops
                        .get(li)
                        .copied()
                        .ok_or_else(|| cat!("$dump.resume: ", site.name, " is in more loops"))?;
                    let mut kind = None;
                    let mut pick = Value::UNDEFINED;
                    let want = if matches!(role, Role::LoopNext(_)) {
                        li += 1;
                        "next"
                    } else {
                        "iterator"
                    };
                    for (k, v) in entries_arg(h, lp, "$dump.resume: loop")? {
                        match k.as_str() {
                            "kind" => kind = Some(string_arg(h, v, "$dump.resume: loop kind")?),
                            "iterator" | "next" => {
                                if k == want {
                                    pick = v;
                                }
                            }
                            _ => return Err(cat!("$dump.resume: unknown loop key ", k)),
                        }
                    }
                    if kind.as_deref() != Some(l.name()) {
                        return Err(cat!(
                            "$dump.resume: loop ",
                            li,
                            " of ",
                            site.name,
                            " is a ",
                            l.name()
                        ));
                    }
                    pick
                }
                _ => {
                    let want = role_name(role);
                    let pair = ops
                        .get(oi)
                        .map(|&p| array_arg(h, p, "$dump.resume: operand"))
                        .transpose()?
                        .ok_or_else(|| cat!("$dump.resume: ", site.name, " needs ", want))?;
                    oi += 1;
                    let (Some(&name), Some(&v), 2) = (pair.first(), pair.get(1), pair.len()) else {
                        return Err("$dump.resume: an operand is a [role, value] pair".into());
                    };
                    let name = string_arg(h, name, "$dump.resume: operand role")?;
                    if name != want {
                        return Err(cat!(
                            "$dump.resume: operand ",
                            oi - 1,
                            " of ",
                            site.name,
                            " is ",
                            want,
                            ", not ",
                            name
                        ));
                    }
                    v
                }
            };
            frame.push(v);
        }
        if oi != ops.len() || li != loops.len() {
            return Err(cat!(
                "$dump.resume: ",
                site.name,
                " has fewer operands or loops than given"
            ));
        }
        Ok(())
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
