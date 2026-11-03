//! What is in flight when a VM is idle (0172, docs/continuation-dumps.md, D2561, D2562,
//! D2610): open host calls, pending promises with their reactions, suspended
//! activations and the iterators of their loops, written as the calls that recreate
//! them.
//!
//! - An open call is `$dump.pending(id, name, args)`, by the name the host gives its
//!   function (`DumpOptions::host_fn_name`).
//! - A pending promise is written by what made it: an open call, an activation (its
//!   result promise), a `then` on another promise (its derived promise), a combinator
//!   (`Promise.all` ...), or `new Promise` whose resolving functions the dump hands to
//!   whoever held them; else `new Promise(() => {})`, which never settles, as nothing can
//!   settle it.
//! - Its reactions are written in their order, each as the call that adds it: an await
//!   is the awaiting activation's `$dump.resume`, a `then`/`catch`/`finally` the call
//!   again, a combinator element the combinator called again with the elements that
//!   settled in place. A dump whose dependencies would add them in another order is
//!   refused.
//! - An activation is `$dump.resume(fn, {...})` with the bindings, operands and loops its
//!   function's suspension map names at the site where it stopped; the map comes from
//!   compiling the function's script again, with the options it was compiled with.

use alloc::string::String;
use alloc::vec::Vec;

use super::*;
use crate::compiler::{CompileOptions, FunctionMap, Loop, Role, Site, SiteKind, Slot};
use crate::heap::cells::{HostCall, Script};
use crate::interp::run::{builtin_closure_kind, promise, suspend};

/// Builtin closure kinds (`NewBuiltinClosure`, interp/run/intrinsics.rs).
const RESOLVE: usize = 0;
const REJECT: usize = 1;
const ALL_ELEMENT: usize = 3;
const SETTLED_FULFILLED: usize = 4;
const SETTLED_REJECTED: usize = 5;
const ANY_REJECT: usize = 6;
const THEN_FINALLY: usize = 7;
const CATCH_FINALLY: usize = 8;
const FROM_SYNC_UNWRAP: usize = 13;
const FROM_SYNC_CLOSE: usize = 14;

pub(super) struct Flight<'a> {
    pub host_fn_name: &'a dyn Fn(u32) -> Option<String>,
    calls: Vec<Call>,
    handles: Vec<(String, Value)>,
    pends: Vec<Pend>,
    acts: Vec<Act>,
    combos: Vec<Combo>,
    clos: Vec<Clo>,
    /// Suspension maps by script cell.
    maps: Vec<(u32, Option<Vec<FunctionMap>>)>,
    /// Promises in the order the dump made them: the end writes their other reactions.
    made: Vec<usize>,
}

impl<'a> Flight<'a> {
    pub fn new(host_fn_name: &'a dyn Fn(u32) -> Option<String>) -> Flight<'a> {
        Flight {
            host_fn_name,
            calls: Vec::new(),
            handles: Vec::new(),
            pends: Vec::new(),
            acts: Vec::new(),
            combos: Vec::new(),
            clos: Vec::new(),
            maps: Vec::new(),
            made: Vec::new(),
        }
    }

    /// Whether anything is in flight (the header says so).
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.calls.len(), self.acts.len(), self.pends.len())
    }
}

struct Call {
    id: u32,
    name: String,
    args: Vec<Value>,
    promise: Value,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Creator {
    Unknown,
    Call(usize),
    Act(usize),
    /// The derived promise of reaction `j` of pending promise `p`.
    Then(usize, usize),
    Combo(usize),
    /// `new Promise`, handing out its resolving functions.
    Resolvers,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Consumer {
    Then,
    /// An element of a combinator whose promise settled already: its function does
    /// nothing any more, a reaction with handlers that do nothing.
    Dead,
    Await(usize),
    Finally,
    Element(usize),
}

#[derive(Clone, Copy)]
struct Reaction {
    cap: Value,
    f: Value,
    r: Value,
    consumer: Consumer,
    busy: bool,
}

struct Pend {
    node: usize,
    reactions: Vec<Reaction>,
    creator: Creator,
    /// The reactions the dump added so far.
    cursor: usize,
    made: bool,
    busy: bool,
    /// The name of its resolving functions (`Creator::Resolvers`).
    resolvers: String,
}

struct Act {
    node: usize,
    s: suspend::Suspended,
    site: Option<Site>,
    /// The awaited promise and the reaction's index there.
    awaiting: Option<(usize, usize)>,
    state: State,
}

struct Combo {
    /// The closure kind of its elements: ALL_ELEMENT, SETTLED_*, ANY_REJECT.
    kind: usize,
    cap: Value,
    values: Value,
    remaining: Value,
    /// Pending elements: index, promise, reaction.
    elements: Vec<(u32, usize, usize)>,
    state: State,
}

/// A builtin closure: its kind and data.
struct Clo {
    node: usize,
    kind: usize,
    data: Value,
}

impl Dumper<'_> {
    // ---------------------------------------------------------------- roots

    /// The roots of what is in flight: the open host calls and the host's handles. A
    /// dump needs an idle VM (D2561).
    pub(super) fn flight_roots(&mut self, opts: &DumpOptions<'_>) {
        let vm = self.vm;
        let h = self.h;
        use crate::interp::RunState;
        if !matches!(
            vm.state(),
            RunState::Idle | RunState::Done | RunState::Threw
        ) {
            self.refuse("the VM".into(), "is not idle (a call is running or parked)");
        } else if promise::has_jobs(h) {
            self.refuse("the VM".into(), "is not idle (jobs are queued)");
        } else if promise::has_inbox(h) {
            self.refuse(
                "the VM".into(),
                "is not idle (a host result was delivered and not applied yet)",
            );
        }
        for id in vm.pending_host_calls() {
            let Some(info) = vm.host_call(id) else {
                continue;
            };
            let path = cat!("<call ", id, ">");
            let Some(name) = (self.fl.host_fn_name)(info.fn_id) else {
                self.refuse(path, "a call of a host function the host does not name");
                continue;
            };
            let promise = crate::interp::run::host_call_cell(h, id)
                .map_or(Value::UNDEFINED, |c| h.arena().get(c, HostCall::PROMISE));
            for (i, &a) in info.args.iter().enumerate() {
                self.note(a, || cat!(path, ".args[", i, "]"));
            }
            self.note(promise, || path.clone());
            self.fl.calls.push_out(Call {
                id,
                name,
                args: info.args,
                promise,
            });
        }
        for &(name, v) in opts.handles {
            self.note(v, || cat!("<handle ", name, ">"));
            self.fl.handles.push_out((String::from(name), v));
        }
    }

    // ---------------------------------------------------------------- discovery

    pub(super) fn visit_pending(&mut self, n: usize, o: Obj) {
        let h = self.h;
        let path = self.nodes[n].path.clone();
        let mut reactions = Vec::new();
        if let Some(r) = h.object_slot(o, 2).as_internal() {
            for j in 0..h.value_array_len(r) / 3 {
                let get = |k| h.value_array_get(r, 3 * j + k);
                let (cap, f, rj) = (get(0), get(1), get(2));
                let p = cat!(path, "<reaction ", j, ">");
                self.edge(n, cap, || cat!(p, ".promise"));
                self.edge(n, f, || cat!(p, ".onFulfilled"));
                if rj != f {
                    self.edge(n, rj, || cat!(p, ".onRejected"));
                }
                reactions.push_out(Reaction {
                    cap,
                    f,
                    r: rj,
                    consumer: Consumer::Then,
                    busy: false,
                });
            }
        }
        self.nodes[n].fl = self.fl.pends.len();
        self.fl.pends.push_out(Pend {
            node: n,
            reactions,
            creator: Creator::Unknown,
            cursor: 0,
            made: false,
            busy: false,
            resolvers: String::new(),
        });
    }

    /// The suspension maps of the script of template `t`, compiled again in the scratch
    /// heap with the options it was compiled with.
    fn script_maps(&mut self, t: u32) -> Option<&[FunctionMap]> {
        let h = self.h;
        let script = template::debug_info(h, t).script.get()?;
        let i = match self.fl.maps.iter().position(|m| m.0 == script) {
            Some(i) => i,
            None => {
                let a = h.arena();
                let src = a
                    .get::<Ref>(script, Script::SOURCE)
                    .get()
                    .map(|s| h.str_to_string(crate::value::Str(s)));
                let name = a
                    .get::<Ref>(script, Script::NAME)
                    .get()
                    .map(|s| h.str_to_string(crate::value::Str(s)))
                    .unwrap_or_default();
                let params = a
                    .get::<Ref>(script, Script::PARAMS)
                    .get()
                    .map(|s| h.str_to_string(crate::value::Str(s)))
                    .unwrap_or_default();
                let params: Vec<&str> = params.split(',').filter(|p| !p.is_empty()).collect();
                let bits = a.get::<u32>(script, Script::OPTIONS);
                let maps = match (src, self.scratch.as_mut()) {
                    (Some(src), Some(scratch)) => {
                        let opts = CompileOptions::from_bits(&name, &params, bits);
                        crate::compiler::suspension_maps(scratch, src.as_bytes(), &opts).ok()
                    }
                    _ => None,
                };
                self.fl.maps.push_out((script, maps));
                self.fl.maps.len() - 1
            }
        };
        self.fl.maps[i].1.as_deref()
    }

    pub(super) fn visit_activation(&mut self, n: usize, g: Obj) {
        let h = self.h;
        let path = self.nodes[n].path.clone();
        let s = suspend::read_activation(h, g);
        self.nodes[n].fl = self.fl.acts.len();
        let site = match s.state {
            suspend::AWAITING | suspend::SUSPENDED => self.activation_site(&path, &s),
            suspend::COMPLETED => {
                // A finished generator of the same kind and prototype.
                let proto = h.get_prototype_of(g).map_or(Value::NULL, Value::object);
                self.edge(n, proto, || cat!(path, ".[[Prototype]]"));
                None
            }
            suspend::START => {
                // Called again: its parameters must run no code.
                let simple = s
                    .frame
                    .first()
                    .and_then(|f| f.as_object())
                    .and_then(|f| crate::builtins::function::source_slice(h, f))
                    .is_some_and(|src| simple_params(&src));
                if !simple {
                    self.refuse(
                        path.clone(),
                        "a generator that has not started or has finished, whose parameters run code",
                    );
                }
                let fp = |i: usize| s.frame.get(i).copied().unwrap_or(Value::UNDEFINED);
                let argc = s
                    .frame
                    .get(s.hp as usize + crate::interp::frame::ARGC as usize)
                    .and_then(|v| v.as_int())
                    .unwrap_or(0) as usize;
                self.edge(n, fp(0), || cat!(path, "<function>"));
                self.edge(n, fp(1), || cat!(path, "<this>"));
                for i in 0..argc {
                    self.edge(n, fp(2 + i), || cat!(path, "<args>[", i, "]"));
                }
                self.check_generator_proto(&path, g, fp(0));
                None
            }
            _ => {
                self.refuse(path.clone(), "a generator in the middle of a yield*");
                None
            }
        };
        if site.is_some() && s.kind != suspend::ASYNC {
            let f = s.frame.first().copied().unwrap_or(Value::UNDEFINED);
            self.check_generator_proto(&path, g, f);
        }
        if let Some(site) = &site {
            let hp = s.hp as usize;
            let argc = s
                .frame
                .get(hp + crate::interp::frame::ARGC as usize)
                .and_then(|v| v.as_int())
                .unwrap_or(0) as usize;
            let lp = hp + crate::interp::frame::HEADER as usize;
            let fp = |i: usize| s.frame.get(i).copied().unwrap_or(Value::UNDEFINED);
            self.edge(n, fp(0), || cat!(path, "<function>"));
            self.edge(n, fp(1), || cat!(path, "<this>"));
            for i in 0..argc {
                self.edge(n, fp(2 + i), || cat!(path, "<args>[", i, "]"));
            }
            for (i, slot) in site.locals.iter().enumerate() {
                let name = match slot {
                    Slot::Binding { name, .. } => name.as_str(),
                    Slot::Hidden(name) if !computed_hidden(name) => name.as_str(),
                    _ => continue,
                };
                let v = fp(lp + i);
                // A box's path ends in `~name`: its factory's variable.
                let sep = if v.as_internal().is_some() { "~" } else { "." };
                if let Some(b) = self.edge(n, v, || cat!(path, sep, name))
                    && v.as_internal().is_some()
                {
                    self.nodes[b].frame_box = true;
                }
            }
            for (k, role) in site.operands.iter().enumerate() {
                let v = fp(lp + site.locals.len() + k);
                if matches!(role, Role::LoopNext(Loop::Await))
                    && k > 0
                    && fp(lp + site.locals.len() + k - 1)
                        .as_object()
                        .is_some_and(|o| self.is_from_sync(o))
                {
                    // The dump's own iterator brings its `next`.
                    continue;
                }
                self.edge(n, v, || {
                    cat!(path, "<", super::super::dump_resume::role_name(*role), ">")
                });
            }
            if s.kind == suspend::ASYNC {
                self.edge(n, s.promise, || cat!(path, "<promise>"));
            }
            for (k, &r) in s.queue.iter().enumerate() {
                if let Some(r) = r.as_object() {
                    self.edge(n, h.object_slot(r, 1), || {
                        cat!(path, "<queue ", k, ">.value")
                    });
                    self.edge(n, h.object_slot(r, 2), || cat!(path, "<queue ", k, ">"));
                }
            }
        }
        self.fl.acts.push(Act {
            node: n,
            s,
            site,
            awaiting: None,
            state: State::New,
        });
    }

    /// A generator object whose prototype is not its function's `prototype` (which
    /// `$dump.resume` and a new call give it) is refused.
    fn check_generator_proto(&mut self, path: &str, g: Obj, f: Value) {
        let h = self.h;
        let want = f
            .as_object()
            .and_then(|f| h.get_own_property(f, crate::heap::atoms::PROTOTYPE))
            .and_then(|p| p.value())
            .and_then(|v| v.as_object());
        if h.get_prototype_of(g) != want || !h.own_keys(g).is_empty() {
            self.refuse(
                path.into(),
                "a generator object with another prototype or properties of its own",
            );
        }
    }

    /// The site where activation `s` stopped, checked against its frame.
    fn activation_site(&mut self, path: &str, s: &suspend::Suspended) -> Option<Site> {
        let h = self.h;
        let Some(f) = s.frame.first().and_then(|v| v.as_object()) else {
            self.refuse(path.into(), "an activation without its function");
            return None;
        };
        let t = h.closure_template(f);
        let info = template::debug_info(h, t);
        let (a, b) = (info.src_start, info.src_end);
        let params = usize::from(h.arena().get::<u16>(t, FuncTemplate::PARAMS));
        let Some(maps) = self.script_maps(t) else {
            self.refuse(
                path.into(),
                "an activation whose script cannot be compiled again",
            );
            return None;
        };
        let site = maps
            .iter()
            .find(|m| m.start == a && m.end == b)
            .and_then(|m| m.sites.iter().find(|st| st.resume_pc == s.pc))
            .cloned();
        let Some(site) = site else {
            self.refuse(
                path.into(),
                "an activation stopped where its map has no site",
            );
            return None;
        };
        if let Some(u) = &site.unsupported {
            let why = cat!(
                "an activation at ",
                site.name,
                ", which dumps cannot resume yet: ",
                u
            );
            self.refuse(path.into(), &why);
            return None;
        }
        let at_yield = matches!(site.kind, SiteKind::Yield | SiteKind::AsyncYield);
        let hp = s.hp as usize;
        let argc = s
            .frame
            .get(hp + crate::interp::frame::ARGC as usize)
            .and_then(|v| v.as_int())
            .unwrap_or(-1);
        let fits = argc >= 0
            && hp == 2 + params.max(argc as usize)
            && site.args.len() == params
            && s.frame.len()
                == hp
                    + crate::interp::frame::HEADER as usize
                    + site.locals.len()
                    + site.operands.len()
            && at_yield == (s.state == suspend::SUSPENDED);
        if !fits {
            self.refuse(
                path.into(),
                "an activation whose frame does not fit its map",
            );
            return None;
        }
        Some(site)
    }

    pub(super) fn visit_builtin_closure(&mut self, n: usize, o: Obj) {
        let h = self.h;
        let path = self.nodes[n].path.clone();
        let kind = builtin_closure_kind(h, o).unwrap_or(usize::MAX);
        let data = h.object_slot(o, 1);
        match kind {
            RESOLVE | REJECT | ALL_ELEMENT | SETTLED_FULFILLED | SETTLED_REJECTED | ANY_REJECT
            | THEN_FINALLY | CATCH_FINALLY | FROM_SYNC_UNWRAP | FROM_SYNC_CLOSE => {
                self.edge(n, data, || cat!(path, "<data>"));
            }
            _ => self.refuse(path, "an internal function of a builtin"),
        }
        self.nodes[n].fl = self.fl.clos.len();
        self.fl.clos.push_out(Clo {
            node: n,
            kind,
            data,
        });
    }

    pub(super) fn visit_iterator(&mut self, n: usize, o: Obj) {
        let h = self.h;
        let path = self.nodes[n].path.clone();
        if h.object_class(o) == ObjectClass::ArrayIterator {
            self.edge(n, h.object_slot(o, 0), || cat!(path, "<target>"));
        } else if let Some((entries, _)) = h.collection_iterator_position(o) {
            // (Written again if only the iterator holds the collection.)
            for (i, (k, v)) in h.collection_entries_of(entries).into_iter().enumerate() {
                self.edge(n, k, || cat!(path, "<entries>[", i, "]"));
                self.edge(n, v, || cat!(path, "<entries>[", i, "]"));
            }
        }
        let extra = self.h.own_keys(o).len();
        if extra > 0 {
            self.refuse(path, "an iterator with properties of its own");
        }
        self.nodes[n].fl = 0;
    }

    // ---------------------------------------------------------------- analysis

    fn pend_of(&self, v: Value) -> Option<usize> {
        let n = self.node_of(v)?;
        let o = self.nodes[n].v.as_object()?;
        (self.h.object_class(o) == ObjectClass::Promise && self.nodes[n].fl != NO_FLIGHT)
            .then_some(self.nodes[n].fl)
    }

    fn clo_of(&self, v: Value) -> Option<&Clo> {
        let n = self.node_of(v)?;
        let o = self.nodes[n].v.as_object()?;
        (self.h.object_class(o) == ObjectClass::BuiltinClosure && self.nodes[n].fl != NO_FLIGHT)
            .then(|| &self.fl.clos[self.nodes[n].fl])
    }

    fn act_of(&self, v: Value) -> Option<usize> {
        let n = self.node_of(v)?;
        let o = self.nodes[n].v.as_object()?;
        (self.h.object_class(o) == ObjectClass::Generator && self.nodes[n].fl != NO_FLIGHT)
            .then_some(self.nodes[n].fl)
    }

    /// Who made each pending promise, and what each reaction is.
    fn analyze_flight(&mut self) {
        let h = self.h;
        for c in 0..self.fl.calls.len() {
            if let Some(p) = self.pend_of(self.fl.calls[c].promise) {
                self.fl.pends[p].creator = Creator::Call(c);
            }
        }
        for a in 0..self.fl.acts.len() {
            if self.fl.acts[a].s.kind == suspend::ASYNC
                && let Some(p) = self.pend_of(self.fl.acts[a].s.promise)
                && self.fl.pends[p].creator == Creator::Unknown
            {
                self.fl.pends[p].creator = Creator::Act(a);
            }
        }
        for p in 0..self.fl.pends.len() {
            for j in 0..self.fl.pends[p].reactions.len() {
                let Reaction { cap, f, r, .. } = self.fl.pends[p].reactions[j];
                let path = cat!(self.nodes[self.fl.pends[p].node].path, "<reaction ", j, ">");
                let kind = |d: &Self, v: Value| d.clo_of(v).map(|c| (c.kind, c.data));
                let consumer = if f == r
                    && let Some(a) = self.act_of(f)
                {
                    if self.fl.acts[a].s.state != suspend::AWAITING {
                        self.refuse(
                            path.clone(),
                            "an await of an activation that is not awaiting",
                        );
                    }
                    self.fl.acts[a].awaiting = Some((p, j));
                    Consumer::Await(a)
                } else if let Some((k, data)) = kind(self, f)
                    .filter(|c| matches!(c.0, ALL_ELEMENT | SETTLED_FULFILLED))
                    .or_else(|| kind(self, r).filter(|c| c.0 == ANY_REJECT))
                {
                    match self.combo_element(k, data, p, j) {
                        Some(c) => Consumer::Element(c),
                        None => {
                            self.refuse(path.clone(), "an element of a keyed promise combinator");
                            Consumer::Then
                        }
                    }
                } else if kind(self, f).map(|c| c.0) == Some(THEN_FINALLY)
                    && kind(self, r).map(|c| c.0) == Some(CATCH_FINALLY)
                {
                    Consumer::Finally
                } else {
                    for v in [f, r] {
                        if let Some((k, _)) = kind(self, v)
                            && !matches!(k, RESOLVE | REJECT | FROM_SYNC_UNWRAP | FROM_SYNC_CLOSE)
                        {
                            self.refuse(path.clone(), "a reaction of a builtin in flight");
                        }
                    }
                    Consumer::Then
                };
                if !cap.is_undefined() && promise::as_promise(h, cap).is_none() {
                    self.refuse(path.clone(), "a then on a promise of another constructor");
                }
                self.fl.pends[p].reactions[j].consumer = consumer;
                if let Some(d) = self.pend_of(cap)
                    && self.fl.pends[d].creator == Creator::Unknown
                {
                    self.fl.pends[d].creator = Creator::Then(p, j);
                }
            }
        }
        for c in 0..self.fl.combos.len() {
            if let Some(p) = self.pend_of(self.fl.combos[c].cap)
                && self.fl.pends[p].creator == Creator::Unknown
            {
                self.fl.pends[p].creator = Creator::Combo(c);
            }
            // Every element that has not settled must be found (`remaining` counts them).
            let rem = self.fl.combos[c]
                .remaining
                .as_object()
                .and_then(|r| h.object_slot(r, 0).as_number())
                .unwrap_or(-1.0);
            if self.pend_of(self.fl.combos[c].cap).is_none() {
                for &(_, p, j) in &self.fl.combos[c].elements {
                    self.fl.pends[p].reactions[j].consumer = Consumer::Dead;
                }
                self.fl.combos[c].state = State::Done;
            } else if rem != self.fl.combos[c].elements.len() as f64 {
                let p = self.combo_path(c);
                self.refuse(
                    p,
                    "a promise combinator whose pending elements are not all reachable",
                );
            }
        }
        for k in 0..self.fl.clos.len() {
            let Clo { kind, data, .. } = self.fl.clos[k];
            if kind != RESOLVE && kind != REJECT {
                continue;
            }
            let target = match data.as_object() {
                Some(d) if h.object_class(d) == ObjectClass::Record => h.object_slot(d, 0),
                _ => data,
            };
            if let Some(p) = self.pend_of(target)
                && self.fl.pends[p].creator == Creator::Unknown
            {
                self.fl.pends[p].creator = Creator::Resolvers;
            }
        }
    }

    fn combo_path(&self, c: usize) -> String {
        match self.node_of(self.fl.combos[c].cap) {
            Some(n) => self.nodes[n].path.clone(),
            None => "<a promise combinator>".into(),
        }
    }

    /// Records element `data` (an element Record) of a combinator; its index.
    fn combo_element(&mut self, kind: usize, data: Value, p: usize, j: usize) -> Option<usize> {
        let h = self.h;
        let r = data.as_object()?;
        if !h.object_slot(r, 5).is_undefined() {
            return None;
        }
        let index = h.object_slot(r, 1).as_number()? as u32;
        let cap = h.object_slot(r, 3);
        let c = match self.fl.combos.iter().position(|c| c.cap == cap) {
            Some(c) => c,
            None => {
                self.fl.combos.push_out(Combo {
                    kind: if kind == SETTLED_REJECTED {
                        SETTLED_FULFILLED
                    } else {
                        kind
                    },
                    cap,
                    values: h.object_slot(r, 2),
                    remaining: h.object_slot(r, 4),
                    elements: Vec::new(),
                    state: State::New,
                });
                self.fl.combos.len() - 1
            }
        };
        self.fl.combos[c].elements.push_out((index, p, j));
        Some(c)
    }

    // ---------------------------------------------------------------- emission

    /// Writes the open calls, first and in their order.
    pub(super) fn flight_start(&mut self) {
        self.analyze_flight();
        for c in 0..self.fl.calls.len() {
            if let Some(p) = self.pend_of(self.fl.calls[c].promise) {
                self.make(p);
            }
        }
    }

    /// The rest of the reactions of every promise the dump made, then the handles.
    pub(super) fn flight_finish(&mut self) {
        loop {
            let mut progress = false;
            for p in 0..self.fl.pends.len() {
                let len = self.fl.pends[p].reactions.len();
                if len > 0 && self.fl.pends[p].cursor < len {
                    self.react_upto(p, len);
                    progress = true;
                }
            }
            for a in 0..self.fl.acts.len() {
                if self.fl.acts[a].state == State::New && self.fl.acts[a].s.kind != suspend::ASYNC {
                    self.emit_act(a);
                    progress = true;
                }
            }
            if !progress || !self.refusals.is_empty() {
                break;
            }
        }
        let handles = core::mem::take(&mut self.fl.handles);
        for (name, v) in handles {
            let before = self.fns_read.get();
            let e = self.expr(v, None, None);
            self.line(cat!("$dump.handle(", quote(&name), ", ", e, ");"), before);
        }
    }

    /// Pushes a line that writes no binding.
    fn line(&mut self, text: String, fns_before: u32) {
        self.ns = true;
        self.out.push_out(text);
        self.skeleton
            .push_out((self.fns_read.get() == fns_before).then(String::new));
    }

    /// The expression of an in-flight node.
    pub(super) fn flight_expr(&mut self, n: usize) -> String {
        let h = self.h;
        let o = self.nodes[n].v.as_object().expect("an object");
        let i = self.nodes[n].fl;
        match h.object_class(o) {
            ObjectClass::Promise => self.make(i),
            ObjectClass::Generator => {
                if self.fl.acts[i].s.kind == suspend::ASYNC {
                    let p = self.nodes[n].path.clone();
                    self.refuse(p, "internal: an async function's state as a value");
                    return "undefined".into();
                }
                self.emit_act(i)
            }
            ObjectClass::BuiltinClosure => self.closure_expr(i),
            ObjectClass::ArrayIterator => self.iterator_expr(n, o),
            ObjectClass::CollectionIterator => self.collection_iterator_expr(n, o),
            ObjectClass::Record if self.is_from_sync(o) => self.from_sync_expr(n, o),
            _ => {
                let p = self.nodes[n].path.clone();
                self.refuse(p, "internal: a promise builtin's record as a value");
                "undefined".into()
            }
        }
    }

    /// The iterator a `for await` over a sync iterable makes (CreateAsyncFromSyncIterator):
    /// a Record `[iterator, next]` with %AsyncFromSyncIteratorPrototype%.
    pub(super) fn is_from_sync(&self, o: Obj) -> bool {
        let h = self.h;
        h.object_class(o) == ObjectClass::Record
            && h.get_prototype_of(o).map(Value::object)
                == Some(intrinsic(h, Intrinsic::AsyncFromSyncIteratorProto))
    }

    /// An async-from-sync iterator: the dump's own, `$dump.fromSync(it, next)`, which steps
    /// the same way (the object is never visible to user code).
    fn from_sync_expr(&mut self, n: usize, o: Obj) -> String {
        if self.nodes[n].state == State::Done {
            return self.nodes[n].loc.clone();
        }
        self.nodes[n].state = State::Busy;
        let h = self.h;
        let before = self.fns_read.get();
        let it = self.expr(h.object_slot(o, 0), None, None);
        let next = self.expr(h.object_slot(o, 1), None, None);
        if it.is_empty() || next.is_empty() {
            let p = self.nodes[n].path.clone();
            self.refuse(p, "a for-await over an iterator on a reference cycle");
        }
        self.helpers |= H_FROM_SYNC;
        let name = self.fresh("i");
        self.line(
            cat!(name, " = $dump.fromSync(", it, ", ", next, ");"),
            before,
        );
        self.nodes[n].loc = name.clone();
        self.nodes[n].state = State::Done;
        name
    }

    fn closure_expr(&mut self, k: usize) -> String {
        let h = self.h;
        let Clo { node, kind, data } = self.fl.clos[k];
        if kind == FROM_SYNC_UNWRAP {
            // A for-await over a sync iterator: the result of a step (D2612).
            return cat!("(v) => ({ value: v, done: ", tf(data == Value::TRUE), " })");
        }
        if kind == FROM_SYNC_CLOSE {
            let it = self.expr(data, None, None);
            self.helpers |= H_FROM_SYNC;
            return cat!("$dump.closeThrow(", it, ")");
        }
        if kind != RESOLVE && kind != REJECT {
            let p = self.nodes[node].path.clone();
            self.refuse(p, "internal: a promise builtin's function as a value");
            return "undefined".into();
        }
        let target = match data.as_object() {
            Some(d) if h.object_class(d) == ObjectClass::Record => h.object_slot(d, 0),
            _ => data,
        };
        match self.pend_of(target) {
            Some(p) if self.fl.pends[p].creator == Creator::Resolvers => {
                self.make(p);
                let r = self.fl.pends[p].resolvers.clone();
                if r.is_empty() {
                    return r;
                }
                cat!(r, "[", kind, "]")
            }
            Some(_) => {
                let p = self.nodes[node].path.clone();
                self.refuse(p, "a resolving function of a promise made otherwise");
                "undefined".into()
            }
            // The promise settled: calling them does nothing.
            None => {
                self.helpers |= H_SPENT;
                self.ns = true;
                cat!("$dump.spent()[", kind, "]")
            }
        }
    }

    fn iterator_expr(&mut self, n: usize, o: Obj) -> String {
        if self.nodes[n].state == State::Done {
            return self.nodes[n].loc.clone();
        }
        if self.nodes[n].state == State::Busy {
            return String::new();
        }
        self.nodes[n].state = State::Busy;
        let h = self.h;
        let before = self.fns_read.get();
        let target = h.object_slot(o, 0);
        let index = h.object_slot(o, 1).as_int().unwrap_or(0);
        let kind = h.object_slot(o, 2).as_int();
        let method = match kind {
            Some(0) => "Array.prototype.keys",
            Some(2) => "Array.prototype.entries",
            // A String Iterator (D1346): it steps by code points.
            Some(3) => "String.prototype[Symbol.iterator]",
            _ => "Array.prototype.values",
        };
        let (t, steps) = if target.is_undefined() {
            (String::from(if kind == Some(3) { "\"\"" } else { "[]" }), 1)
        } else if kind == Some(3) {
            let units: Vec<u16> = target
                .as_string()
                .map(|s| h.str_units(s).take(index.max(0) as usize).collect())
                .unwrap_or_default();
            let low = |u: u16| (0xDC00..0xE000).contains(&u);
            let high = |u: u16| (0xD800..0xDC00).contains(&u);
            let cps = (0..units.len())
                .filter(|&i| !(low(units[i]) && i > 0 && high(units[i - 1])))
                .count();
            (self.expr(target, None, None), cps as i32)
        } else {
            (self.expr(target, None, None), index)
        };
        if t.is_empty() {
            let p = self.nodes[n].path.clone();
            self.refuse(p, "an iterator over an array on a reference cycle");
        }
        let mut e = cat!(method, ".call(", t, ")");
        if steps > 0 {
            self.helpers |= H_ADVANCE;
            e = cat!("$dump.advance(", e, ", ", steps, ")");
        }
        let name = self.fresh("i");
        self.line(cat!(name, " = ", e, ";"), before);
        self.nodes[n].loc = name.clone();
        self.nodes[n].state = State::Done;
        name
    }

    /// A Map or Set iterator: made on the collection and advanced past the live entries
    /// it has passed (advancing runs no user code).
    fn collection_iterator_expr(&mut self, n: usize, o: Obj) -> String {
        if self.nodes[n].state == State::Done {
            return self.nodes[n].loc.clone();
        }
        self.nodes[n].state = State::Busy;
        let h = self.h;
        let before = self.fns_read.get();
        let set = h.object_slot(o, 3).as_int() == Some(1);
        let class = if set { "Set" } else { "Map" };
        let method = match (set, h.object_slot(o, 2).as_int()) {
            (false, Some(0)) => "keys",
            (_, Some(2)) => "entries",
            _ => "values",
        };
        let e = match h.collection_iterator_position(o) {
            None => {
                self.helpers |= H_ADVANCE;
                cat!(
                    "$dump.advance(",
                    class,
                    ".prototype.",
                    method,
                    ".call(new ",
                    class,
                    "()), 1)"
                )
            }
            Some((entries, live)) => {
                // The collection: a node of the dump whose entries these are.
                let owner = self.nodes.iter().position(|d| {
                    d.v.as_object().is_some_and(|c| {
                        matches!(h.object_class(c), ObjectClass::Map | ObjectClass::Set)
                            && h.collection_entries_ref(c) == Some(entries)
                    })
                });
                let t = match owner {
                    Some(c) => self.expr(self.nodes[c].v, None, None),
                    // Only the iterator holds the collection: a new one with its entries.
                    None => {
                        let mut items = Vec::new();
                        for (k, v) in h.collection_entries_of(entries) {
                            let k = self.expr(k, None, None);
                            if set {
                                items.push_out(k);
                            } else {
                                let v = self.expr(v, None, None);
                                items.push_out(cat!("[", k, ", ", v, "]"));
                            }
                        }
                        cat!("new ", class, "([", join(&items, ", "), "])")
                    }
                };
                if t.is_empty() {
                    let p = self.nodes[n].path.clone();
                    self.refuse(p, "an iterator over a collection on a reference cycle");
                }
                let mut e = cat!(class, ".prototype.", method, ".call(", t, ")");
                if live > 0 {
                    self.helpers |= H_ADVANCE;
                    e = cat!("$dump.advance(", e, ", ", live, ")");
                }
                e
            }
        };
        let name = self.fresh("i");
        self.line(cat!(name, " = ", e, ";"), before);
        self.nodes[n].loc = name.clone();
        self.nodes[n].state = State::Done;
        name
    }

    /// Makes pending promise `p` (once); its location.
    fn make(&mut self, p: usize) -> String {
        let n = self.fl.pends[p].node;
        if self.fl.pends[p].made {
            return self.nodes[n].loc.clone();
        }
        if self.fl.pends[p].busy {
            return String::new();
        }
        if self.data_depth > 0
            && matches!(
                self.fl.pends[p].creator,
                Creator::Act(_) | Creator::Then(..) | Creator::Combo(_)
            )
        {
            // Data holds it: the activation or reaction that makes it may need that data
            // first. The data gets the promise by a patch at the end; the promise is made
            // with what makes it (at the latest when its awaited promise's reactions are
            // written).
            self.reserve(n, "p");
            return String::new();
        }
        self.fl.pends[p].busy = true;
        let before = self.fns_read.get();
        match self.fl.pends[p].creator {
            Creator::Call(c) => {
                let args = self.fl.calls[c].args.clone();
                let a: Vec<String> = args.iter().map(|&v| self.expr(v, None, None)).collect();
                if a.iter().any(String::is_empty) {
                    let path = self.nodes[n].path.clone();
                    self.refuse(
                        path,
                        "an open call whose arguments are on a reference cycle",
                    );
                }
                let name = self.reserve(n, "p");
                let Call { id, name: f, .. } = &self.fl.calls[c];
                let l = cat!(
                    name,
                    " = $dump.pending(",
                    *id,
                    ", ",
                    quote(f),
                    ", [",
                    join(&a, ", "),
                    "]);"
                );
                self.line(l, before);
                self.made(p, name);
            }
            Creator::Act(a) => {
                self.emit_act(a);
            }
            Creator::Then(q, j) => {
                self.react_upto(q, j + 1);
            }
            Creator::Combo(c) => {
                self.emit_combo(c);
            }
            Creator::Resolvers => {
                let name = self.reserve(n, "p");
                let r = self.fresh("r");
                self.line(
                    cat!(
                        name,
                        " = new Promise((res, rej) => { ",
                        r,
                        " = [res, rej]; });"
                    ),
                    before,
                );
                self.fl.pends[p].resolvers = r;
                self.made(p, name);
            }
            Creator::Unknown => {
                let name = self.reserve(n, "p");
                self.line(cat!(name, " = new Promise(() => {});"), before);
                self.made(p, name);
            }
        }
        self.fl.pends[p].busy = false;
        if !self.fl.pends[p].made {
            // Its maker is being written further up (a reference cycle): the caller
            // patches the reference in later, or refuses.
            return String::new();
        }
        self.nodes[n].loc.clone()
    }

    /// The location of node `n`, chosen now if it has none yet (so that patches can name
    /// a value that is made later).
    fn reserve(&mut self, n: usize, prefix: &str) -> String {
        if self.nodes[n].loc.is_empty() {
            self.nodes[n].loc = self.fresh(prefix);
        }
        self.nodes[n].loc.clone()
    }

    fn made(&mut self, p: usize, loc: String) {
        let n = self.fl.pends[p].node;
        self.nodes[n].loc = loc;
        self.nodes[n].state = State::Done;
        self.fl.pends[p].made = true;
        self.fl.made.push_out(p);
    }

    /// Adds the reactions of `q` before `k`, in their order.
    fn react_upto(&mut self, q: usize, k: usize) {
        if self.make(q).is_empty() {
            let path = self.nodes[self.fl.pends[q].node].path.clone();
            self.refuse(path, "a promise reacted to before the dump can make it");
            return;
        }
        while self.fl.pends[q].cursor < k && self.refusals.is_empty() {
            let j = self.fl.pends[q].cursor;
            if self.fl.pends[q].reactions[j].busy {
                // Being written further up: it adds itself.
                return;
            }
            self.fl.pends[q].reactions[j].busy = true;
            match self.fl.pends[q].reactions[j].consumer {
                Consumer::Await(a) => {
                    self.emit_act(a);
                }
                Consumer::Then | Consumer::Finally | Consumer::Dead => self.emit_then(q, j),
                Consumer::Element(c) => self.emit_combo(c),
            }
            self.fl.pends[q].reactions[j].busy = false;
            if self.fl.pends[q].cursor == j {
                let path = self.nodes[self.fl.pends[q].node].path.clone();
                self.refuse(path, "reactions the dump cannot add in their order");
                return;
            }
        }
    }

    /// The reaction `j` of `q` is added now.
    fn subscribed(&mut self, q: usize, j: usize) {
        if self.fl.pends[q].cursor != j {
            let path = self.nodes[self.fl.pends[q].node].path.clone();
            self.refuse(path, "reactions the dump cannot add in their order");
        }
        self.fl.pends[q].cursor = j + 1;
    }

    /// `p.then(f, r)` (or `p.finally(f)`) again: its derived promise.
    fn emit_then(&mut self, q: usize, j: usize) {
        let h = self.h;
        let before = self.fns_read.get();
        let Reaction {
            cap,
            f,
            r,
            consumer,
            ..
        } = self.fl.pends[q].reactions[j];
        let ql = self.make(q);
        let call = if consumer == Consumer::Dead {
            cat!("Promise.prototype.then.call(", ql, ", () => {}, () => {})")
        } else if consumer == Consumer::Finally {
            let on = self
                .clo_of(f)
                .and_then(|c| c.data.as_object())
                .map_or(Value::UNDEFINED, |d| h.object_slot(d, 0));
            let e = self.expr(on, None, None);
            cat!("Promise.prototype.finally.call(", ql, ", ", e, ")")
        } else {
            let fe = self.expr(f, None, None);
            let re = self.expr(r, None, None);
            if fe.is_empty() || re.is_empty() {
                let path = self.nodes[self.fl.pends[q].node].path.clone();
                self.refuse(path, "a reaction whose handler is on a reference cycle");
            }
            cat!("Promise.prototype.then.call(", ql, ", ", fe, ", ", re, ")")
        };
        self.react_upto(q, j);
        match self.pend_of(cap) {
            Some(d) if !self.fl.pends[d].made => {
                let name = self.reserve(self.fl.pends[d].node, "p");
                self.line(cat!(name, " = ", call, ";"), before);
                self.made(d, name);
            }
            _ => self.line(cat!("void ", call, ";"), before),
        }
        self.subscribed(q, j);
    }

    /// A combinator called again: settled elements by their values, pending ones by
    /// their promises.
    fn emit_combo(&mut self, c: usize) {
        if self.fl.combos[c].state != State::New {
            return;
        }
        self.fl.combos[c].state = State::Busy;
        let h = self.h;
        let before = self.fns_read.get();
        let mut elements = self.fl.combos[c].elements.clone();
        crate::sort::sort_by(&mut elements, |a, b| a.0 < b.0);
        let values = self.fl.combos[c].values.as_object();
        let len = values.map_or(0, |v| h.array_length(v));
        let kind = self.fl.combos[c].kind;
        let mut items: Vec<String> = Vec::new();
        for i in 0..len {
            let e = match elements.iter().find(|e| e.0 == i) {
                Some(&(_, p, _)) => self.make(p),
                None => {
                    let v = values
                        .and_then(|v| h.get_element_fast(v, i))
                        .unwrap_or(Value::UNDEFINED);
                    self.settled_element(kind, v)
                }
            };
            items.push_out(e);
        }
        for &(_, p, j) in &elements {
            self.react_upto(p, j);
        }
        let f = match kind {
            ALL_ELEMENT => "all",
            SETTLED_FULFILLED => "allSettled",
            _ => "any",
        };
        let call = cat!("Promise.", f, "([", join(&items, ", "), "])");
        if items.iter().any(String::is_empty) {
            let p = self.combo_path(c);
            self.refuse(p, "a promise combinator on a reference cycle");
        }
        match self.pend_of(self.fl.combos[c].cap) {
            Some(d) if !self.fl.pends[d].made => {
                let name = self.reserve(self.fl.pends[d].node, "p");
                self.line(cat!(name, " = ", call, ";"), before);
                self.made(d, name);
            }
            _ => self.line(cat!("void ", call, ";"), before),
        }
        for &(_, p, j) in &elements {
            self.subscribed(p, j);
        }
        self.fl.combos[c].state = State::Done;
    }

    /// An element that settled, written so the combinator records the same: its value,
    /// or for a rejection a rejected promise.
    fn settled_element(&mut self, kind: usize, v: Value) -> String {
        let h = self.h;
        let get = |o: Obj, k: &str| {
            h.own_keys(o)
                .into_iter()
                .find(|a| h.atom_name(*a) == k)
                .and_then(|a| h.get_own_property(o, a))
                .and_then(|p| p.value())
        };
        let rejected = |d: &mut Self, e: Value| {
            d.helpers |= H_REJECTED;
            d.ns = true;
            let r = d.expr(e, None, None);
            cat!("$dump.rejected(", r, ")")
        };
        match kind {
            SETTLED_FULFILLED => {
                let Some(o) = v.as_object() else {
                    return "undefined".into();
                };
                let status = get(o, "status").and_then(|s| s.as_string());
                let fulfilled = status.is_some_and(|s| h.str_to_string(s) == "fulfilled");
                if fulfilled {
                    let x = get(o, "value").unwrap_or(Value::UNDEFINED);
                    self.expr(x, None, None)
                } else {
                    let x = get(o, "reason").unwrap_or(Value::UNDEFINED);
                    rejected(self, x)
                }
            }
            ANY_REJECT => rejected(self, v),
            _ => self.expr(v, None, None),
        }
    }

    /// `$dump.resume(fn, {...})` for activation `a`: its location.
    fn emit_act(&mut self, a: usize) -> String {
        let n = self.fl.acts[a].node;
        match self.fl.acts[a].state {
            State::Done => return self.nodes[n].loc.clone(),
            State::Busy => return String::new(),
            State::New => {}
        }
        let Some(site) = self.fl.acts[a].site.clone() else {
            return self.emit_unstarted(a);
        };
        self.fl.acts[a].state = State::Busy;
        let h = self.h;
        let before = self.fns_read.get();
        let path = self.nodes[n].path.clone();
        let s = &self.fl.acts[a].s;
        let frame = s.frame.clone();
        let queue = s.queue.clone();
        let kind = s.kind;
        let hp = s.hp as usize;
        let lp = hp + crate::interp::frame::HEADER as usize;
        let argc = frame
            .get(hp + crate::interp::frame::ARGC as usize)
            .and_then(|v| v.as_int())
            .unwrap_or(0) as usize;
        let fv = |i: usize| frame.get(i).copied().unwrap_or(Value::UNDEFINED);
        let mut fields: Vec<String> = alloc::vec![cat!("at: ", quote(&site.name))];
        let awaiting = self.fl.acts[a].awaiting;
        let mut parts: Vec<String> = Vec::new();
        let fe = self.expr(fv(0), None, None);
        parts.push_out(fe.clone());
        if let Some((q, _)) = awaiting {
            let ql = self.make(q);
            parts.push_out(ql.clone());
            fields.push_out(cat!("awaiting: ", ql));
        } else if s_state_awaiting(&self.fl.acts[a].s) {
            fields.push_out("awaiting: new Promise(() => {})".into());
        }
        let this = self.expr(fv(1), None, None);
        parts.push_out(this.clone());
        if this != "undefined" {
            fields.push_out(cat!("this: ", this));
        }
        let args: Vec<String> = (0..argc)
            .map(|i| self.expr(fv(2 + i), None, None))
            .collect();
        if !args.is_empty() {
            fields.push_out(cat!("args: [", join(&args, ", "), "]"));
        }
        parts.extend(args);
        // Bindings: by name, or `name@scope` where two of a name are in scope.
        let names: Vec<&str> = site
            .args
            .iter()
            .chain(&site.locals)
            .filter_map(|s| match s {
                Slot::Binding { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        let mut bindings: Vec<String> = Vec::new();
        let slots: Vec<(usize, &Slot, bool)> = site
            .args
            .iter()
            .enumerate()
            .map(|(i, s)| (2 + i, s, false))
            .chain(
                site.locals
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (lp + i, s, site.boxed.get(i).copied().unwrap_or(false))),
            )
            .collect();
        for (at, slot, boxed) in slots {
            let key = match slot {
                Slot::Binding { name, scope } => {
                    if names.iter().filter(|x| **x == name.as_str()).count() > 1 {
                        quote(&cat!(name, "@", *scope))
                    } else if is_ident(name) {
                        name.clone()
                    } else {
                        quote(name)
                    }
                }
                Slot::Hidden(name) if !computed_hidden(name) => quote(name),
                _ => continue,
            };
            let v = fv(at);
            let e = self.slot_expr(v, boxed);
            parts.push_out(e.clone());
            bindings.push_out(cat!(key, ": ", e));
        }
        if !bindings.is_empty() {
            fields.push_out(cat!("bindings: { ", join(&bindings, ", "), " }"));
        }
        let mut ops: Vec<String> = Vec::new();
        let mut loops: Vec<String> = Vec::new();
        let mut it = String::new();
        for (k, role) in site.operands.iter().enumerate() {
            let from_sync = matches!(role, Role::LoopNext(Loop::Await))
                && k > 0
                && fv(lp + site.locals.len() + k - 1)
                    .as_object()
                    .is_some_and(|o| self.is_from_sync(o));
            let e = if from_sync {
                cat!(it, ".next")
            } else {
                self.expr(fv(lp + site.locals.len() + k), None, None)
            };
            parts.push_out(e.clone());
            match role {
                Role::LoopIterator(_) => it = e,
                Role::LoopNext(l) => {
                    if *l == Loop::In {
                        self.refuse(path.clone(), "an activation in a for-in loop");
                    }
                    loops.push_out(cat!(
                        "{ kind: \"",
                        l.name(),
                        "\", iterator: ",
                        it,
                        ", next: ",
                        e,
                        " }"
                    ));
                }
                r => ops.push_out(cat!(
                    "[",
                    quote(&super::super::dump_resume::role_name(*r)),
                    ", ",
                    e,
                    "]"
                )),
            }
        }
        if !ops.is_empty() {
            fields.push_out(cat!("operands: [", join(&ops, ", "), "]"));
        }
        if !loops.is_empty() {
            fields.push_out(cat!("loops: [", join(&loops, ", "), "]"));
        }
        let mut reqs: Vec<String> = Vec::new();
        for r in queue {
            let Some(r) = r.as_object() else { continue };
            let t = match h.object_slot(r, 0).as_number().map(|t| t as i32) {
                Some(1) => "throw",
                Some(2) => "return",
                _ => "next",
            };
            let v = self.expr(h.object_slot(r, 1), None, None);
            let p = match self.pend_of(h.object_slot(r, 2)) {
                Some(p) => self.make(p),
                None => String::new(),
            };
            parts.push_out(v.clone());
            parts.push_out(p.clone());
            reqs.push_out(cat!(
                "{ type: \"",
                t,
                "\", value: ",
                v,
                ", promise: ",
                p,
                " }"
            ));
        }
        if !reqs.is_empty() {
            fields.push_out(cat!("queue: [", join(&reqs, ", "), "]"));
        }
        if parts.iter().any(String::is_empty) {
            self.refuse(path.clone(), "an activation on a reference cycle");
        }
        if let Some((q, j)) = awaiting {
            self.react_upto(q, j);
        }
        let name = match self.pend_of(self.fl.acts[a].s.promise) {
            Some(p) if kind == suspend::ASYNC => self.reserve(self.fl.pends[p].node, "a"),
            _ => self.reserve(n, "a"),
        };
        let comment = fv(0)
            .as_object()
            .map(|f| self.site_comment(f, &site))
            .unwrap_or_default();
        self.line(
            cat!(
                comment,
                name,
                " = $dump.resume(",
                fe,
                ", { ",
                join(&fields, ", "),
                " });"
            ),
            before,
        );
        if let Some((q, j)) = awaiting {
            self.subscribed(q, j);
        }
        self.nodes[n].loc = name.clone();
        self.nodes[n].state = State::Done;
        self.fl.acts[a].state = State::Done;
        if kind == suspend::ASYNC
            && let Some(p) = self.pend_of(self.fl.acts[a].s.promise)
            && !self.fl.pends[p].made
        {
            self.made(p, name.clone());
        }
        name
    }

    /// A generator that has not started (called again) or has finished (a new one,
    /// finished at once, with its prototype).
    fn emit_unstarted(&mut self, a: usize) -> String {
        let n = self.fl.acts[a].node;
        self.fl.acts[a].state = State::Busy;
        let h = self.h;
        let before = self.fns_read.get();
        let s = &self.fl.acts[a].s;
        let (state, kind) = (s.state, s.kind);
        let frame = s.frame.clone();
        let hp = s.hp as usize;
        let e = if state == suspend::START {
            let argc = frame
                .get(hp + crate::interp::frame::ARGC as usize)
                .and_then(|v| v.as_int())
                .unwrap_or(0) as usize;
            let fv = |i: usize| frame.get(i).copied().unwrap_or(Value::UNDEFINED);
            let f = self.expr(fv(0), None, None);
            let this = self.expr(fv(1), None, None);
            let args: Vec<String> = (0..argc)
                .map(|i| self.expr(fv(2 + i), None, None))
                .collect();
            cat!(
                "Reflect.apply(",
                f,
                ", ",
                this,
                ", [",
                join(&args, ", "),
                "])"
            )
        } else {
            let g = self.nodes[n].v.as_object().expect("a generator");
            let proto = h.get_prototype_of(g).map_or(Value::NULL, Value::object);
            // A generator function's own `prototype` is written with the function.
            let owner = self.nodes.iter().position(|d| {
                d.v.as_object().is_some_and(|f| {
                    h.object_class(f) == ObjectClass::Function
                        && h.get_own_property(f, crate::heap::atoms::PROTOTYPE)
                            .and_then(|p| p.value())
                            == Some(proto)
                })
            });
            let p = match owner {
                Some(f) => {
                    let e = self.expr(self.nodes[f].v, None, None);
                    cat!(e, ".prototype")
                }
                None => self.expr(proto, None, None),
            };
            let make = if kind == suspend::ASYNC_GENERATOR {
                "(async function* () {})()"
            } else {
                "(function* () {})()"
            };
            cat!(
                "Object.setPrototypeOf(((g) => (g.return(), g))(",
                make,
                "), ",
                p,
                ")"
            )
        };
        let name = self.reserve(n, "a");
        let comment = if state == suspend::START {
            "// a generator that has not started\n"
        } else {
            "// a finished generator\n"
        };
        self.line(cat!(comment, name, " = ", e, ";"), before);
        self.nodes[n].loc = name.clone();
        self.nodes[n].state = State::Done;
        self.fl.acts[a].state = State::Done;
        name
    }

    /// Where an activation stopped, for the reader: its function, the site, and the
    /// source line with a caret under the `await`/`yield`. Comments only.
    fn site_comment(&self, f: Obj, site: &Site) -> String {
        let h = self.h;
        let t = h.closure_template(f);
        let info = template::debug_info(h, t);
        let a = h.arena();
        let text = |r: Ref| r.get().map(|s| h.str_to_string(crate::value::Str(s)));
        let script = info.script.get();
        let src = script.and_then(|s| text(a.get::<Ref>(s, Script::SOURCE)));
        let file = script
            .and_then(|s| text(a.get::<Ref>(s, Script::NAME)))
            .unwrap_or_default();
        let atom = Atom::from_id(a.get::<u32>(t, FuncTemplate::NAME));
        let fname = if atom.is_null() {
            String::new()
        } else {
            h.atom_name(atom)
        };
        let fname = if fname.is_empty() {
            String::from("<anonymous>")
        } else {
            fname
        };
        let mut out = cat!("// ", fname, ", at ", site.name);
        let Some(src) = src else {
            out.push_str(":\n");
            return out;
        };
        let at = (info.src_start + site.pos) as usize;
        let start = src
            .get(..at)
            .and_then(|b| b.rfind('\n'))
            .map_or(0, |i| i + 1);
        let end = src
            .get(at..)
            .and_then(|r| r.find('\n'))
            .map_or(src.len(), |i| at + i);
        let line_no = src.get(..at).map_or(0, |b| b.matches('\n').count()) + 1;
        let before = src.get(start..at).unwrap_or_default();
        let col = before.chars().count() + 1;
        cat_to!(&mut out, " (", file, ":", line_no, ":", col, "):\n");
        let line: String = src
            .get(start..end)
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect();
        let caret: String = before
            .chars()
            .map(|c| if c == '\t' { '\t' } else { ' ' })
            .collect();
        if col <= 120 {
            cat_to!(&mut out, "//   ", line.trim_end(), "\n//   ", caret, "^\n");
        }
        out
    }

    /// A frame slot's value: `$dump.TDZ`, a Box's cell when closures share it, else the
    /// value.
    fn slot_expr(&mut self, v: Value, boxed: bool) -> String {
        let h = self.h;
        let inner = match (boxed, v.as_internal()) {
            (true, Some(b)) => {
                if let Some(bn) = self.node_of(v)
                    && let g = self.nodes[bn].group
                    && g != usize::MAX
                    && !self.groups[g].closures.is_empty()
                {
                    self.emit_group(g);
                    return self.nodes[bn].loc.clone();
                }
                h.arena().get(b, crate::heap::cells::BoxCell::VALUE)
            }
            _ => v,
        };
        if inner.is_uninitialized() {
            return "$dump.TDZ".into();
        }
        self.expr(inner, None, None)
    }
}

fn s_state_awaiting(s: &suspend::Suspended) -> bool {
    s.state == suspend::AWAITING
}

/// Whether a function's parameter list (in its source text) runs no code when called:
/// no defaults, no destructuring.
fn simple_params(src: &str) -> bool {
    let Some(open) = src.find('(') else {
        return false;
    };
    let mut depth = 0;
    for c in src.get(open..).unwrap_or_default().chars() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return true;
                }
            }
            '=' | '{' | '[' | '`' | '"' | '\'' | '/' => return false,
            _ => {}
        }
    }
    false
}

/// Hidden locals the loader makes itself.
fn computed_hidden(name: &str) -> bool {
    matches!(
        name,
        "<state>" | "this" | "new.target" | "<active>" | "<home>"
    )
}
