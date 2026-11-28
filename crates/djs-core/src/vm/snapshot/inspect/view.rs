//! The run in JavaScript terms (D3760): what each open call waits in, as sentences and
//! JavaScript literals, each part with a plain note on what it is and why the snapshot
//! has it; the storage details (record ids, kinds, pcs, frame layouts) after them. Built
//! once: `describe` renders it as text, the JSON carries it for the docs site.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::*;

/// A part of the run, in JavaScript terms.
#[derive(Clone, Default)]
pub(super) struct Node {
    /// `call`, `frame`, `element`, `reaction`, `result`, `end`, `seen`.
    pub kind: &'static str,
    /// What it is, as a sentence: "fetchPage(n = 1) is stopped at `debugger;` (line 3)".
    pub title: String,
    /// One plain sentence: what this part is and why the snapshot holds it.
    pub note: &'static str,
    /// Where it waits: script, line, column, the line's text.
    pub source: Option<(u32, u32, u32, String)>,
    /// Its variables: parameters and locals, then the captured ones.
    pub bindings: Vec<Binding>,
    pub captured: Vec<Binding>,
    /// Declared, not reached yet (in the temporal dead zone).
    pub uninit: Vec<String>,
    /// What the expression or loop it waits in is doing: "halfway through pages.push(…)".
    pub doing: Option<String>,
    /// Its record, for the storage layer.
    pub record: Option<u32>,
    /// How djs stores it: records, kinds, pcs.
    pub detail: Vec<String>,
    /// What waits on it.
    pub children: Vec<Node>,
}

/// A variable and its value (a box's value for a captured one: `boxed` is the box).
#[derive(Clone)]
pub(super) struct Binding {
    pub name: String,
    pub value: V,
    pub boxed: Option<u32>,
    /// A parameter (the text shows it in the call already).
    pub param: bool,
}

const NOTE_CALL: &str = "An async call to the host that has not been answered: the run waits for it. It is stored with its id, so the host can answer it after loading the snapshot.";
const NOTE_PAUSE: &str = "A debugger; pause: the statement awaits a host call, so the run is idle there and can be written out. Continue answers the call.";
const NOTE_FRAME: &str = "A suspended async function: its variables and where it continues are stored; its code is not, a loading djs compiles the script again.";
const NOTE_GENERATOR: &str = "A suspended generator: its variables and where it continues are stored; its code is not, a loading djs compiles the script again.";
const NOTE_ELEMENT: &str = "Promise.all and its kin keep a slot per promise they were given: the snapshot keeps which are still open and the results so far.";
const NOTE_REACTION: &str = "A reaction registered on a promise (a .then callback or a resolving function): it runs when the promise settles.";
const NOTE_RESULT: &str =
    "The promise the run's body returned: the host reads the run's result from it once it settles.";
const NOTE_END: &str = "A pending promise nothing waits on yet.";
const NOTE_SEEN: &str = "This chain joins one shown above.";

/// The notes of value kinds (the JSON's `notes`): what a value of the kind is in a
/// snapshot.
pub(super) const VALUE_NOTES: &[(&str, &str)] = &[
    (
        "builtin",
        "Named, not stored: every djs has it, and the loading one finds it by this name.",
    ),
    (
        "host",
        "Named, not stored: the host gives it again when it loads the snapshot.",
    ),
    (
        "function",
        "Code is not stored: the function is named by its place in the script, which a loading djs compiles again.",
    ),
    (
        "script",
        "The script's text: compiled again when the snapshot loads.",
    ),
    (
        "record",
        "Data of the run, stored as a record: its kind and its fields by name.",
    ),
    ("string", "A string, stored as its text."),
    (
        "box",
        "A variable shared with a closure: the function and its closures refer to one box.",
    ),
    (
        "uninitialized",
        "Declared but not reached yet: let and const before their declaration.",
    ),
];

impl Inspection<'_> {
    pub(super) fn build_view(&mut self) {
        let mut roles = ById::default();
        let calls: Vec<Node> = (0..self.calls.len())
            .map(|k| self.call_node(k, &mut roles))
            .collect();
        let others = self.others.clone();
        let others = others
            .iter()
            .map(|l| self.link_node(l, None, &mut roles))
            .collect();
        self.view = calls;
        self.others_view = others;
        self.roles = roles;
    }

    fn call_node(&self, k: usize, roles: &mut ById<String>) -> Node {
        let c = &self.calls[k];
        let args: Vec<String> = c.args.iter().map(|&a| self.short(a)).collect();
        let call = format!("{}({})", c.function, args.join(", "));
        let pause = c.function == "debugger";
        let title = if pause {
            format!(
                "a pause at debugger; (open call {}): the run waits until it is continued",
                c.id
            )
        } else {
            format!(
                "{call} (open call {}): the run waits for the host's answer",
                c.id
            )
        };
        roles.insert(
            c.record,
            format!("open call {}: {call}, waiting for the host's answer", c.id),
        );
        if let Some(p) = c.promise {
            roles.insert(
                p,
                format!("the promise of {call}: the host's answer settles it"),
            );
        }
        let subject = if pause { None } else { Some(call.clone()) };
        let children = c
            .waiters
            .iter()
            .map(|l| self.link_node(l, subject.clone(), roles))
            .collect();
        Node {
            kind: "call",
            title,
            note: if pause { NOTE_PAUSE } else { NOTE_CALL },
            record: Some(c.record),
            detail: alloc::vec![format!(
                "open call {} is record #{} (HostCall), its promise #{}",
                c.id,
                c.record,
                c.promise
                    .map_or_else(|| String::from("?"), |p| p.to_string())
            )],
            children,
            ..Node::default()
        }
    }

    /// The node of link `l`; `waits_for`: what it awaits, in words (`None`: the pause
    /// itself; `Some("")`: nothing named).
    fn link_node(&self, l: &Link, waits_for: Option<String>, roles: &mut ById<String>) -> Node {
        match l {
            Link::Activation(f) => self.frame_node(f, waits_for, roles),
            Link::Element {
                record,
                combinator,
                index,
                values,
                remaining,
                promise,
                waiters,
            } => {
                let n = values
                    .and_then(Self::rec)
                    .map(|v| match self.field(v, "elements") {
                        Some(F::Vs(e)) => e.len(),
                        _ => 0,
                    });
                let k = index.map_or_else(|| String::from("?"), |i| ordinal(i + 1));
                let of = n.map_or_else(String::new, |n| format!(" of {n}"));
                let done = match (n, remaining) {
                    (Some(n), Some(r)) => format!(
                        ", which has {} of {n} results so far",
                        (n as i64 - r).max(0)
                    ),
                    _ => String::new(),
                };
                let title = format!("its promise is the {k}{of} given to {combinator}{done}");
                if let Some(p) = promise {
                    roles.insert(*p, format!("the promise {combinator} returned"));
                }
                Node {
                    kind: "element",
                    title,
                    note: NOTE_ELEMENT,
                    record: Some(*record),
                    detail: alloc::vec![format!(
                        "element function #{record} (BuiltinClosure); results so far in #{}; the combined promise #{}",
                        values
                            .and_then(Self::rec)
                            .map_or_else(|| String::from("?"), |r| r.to_string()),
                        promise.map_or_else(|| String::from("?"), |p| p.to_string())
                    )],
                    children: waiters
                        .iter()
                        .map(|w| self.link_node(w, Some(combinator.clone()), roles))
                        .collect(),
                    ..Node::default()
                }
            }
            Link::Reaction {
                handler,
                promise,
                waiters,
            } => {
                let (title, next) = match *handler {
                    V::Rec(r, _) if self.type_name(r) == FUNCTION => {
                        let name = self.short(*handler);
                        (
                            format!("then the callback {name} runs (a .then reaction)"),
                            String::from("the .then callback"),
                        )
                    }
                    V::Imm(v) if v.is_undefined() => (
                        String::from("then a promise waiting on it settles"),
                        String::new(),
                    ),
                    V::Rec(r, _)
                        if self.type_name(r) == "Object.BuiltinClosure"
                            && matches!(self.field(r, "builtin"), Some(F::Intr(k)) if self.intrinsic(*k) == "PromiseResolveFunction") =>
                    {
                        (
                            String::from(
                                "then another promise takes on its result (it was resolved with this one)",
                            ),
                            String::new(),
                        )
                    }
                    h => (
                        format!("then {} runs, settling another promise", self.short(h)),
                        String::new(),
                    ),
                };
                Node {
                    kind: "reaction",
                    title,
                    note: NOTE_REACTION,
                    record: Self::rec(*handler),
                    detail: alloc::vec![format!(
                        "a reaction calling {}; it settles promise #{}",
                        self.raw(*handler),
                        promise.map_or_else(|| String::from("?"), |p| p.to_string())
                    )],
                    children: waiters
                        .iter()
                        .map(|w| self.link_node(w, Some(next.clone()), roles))
                        .collect(),
                    ..Node::default()
                }
            }
            Link::End {
                promise,
                result,
                held,
            } => {
                let state = self.constant(*promise, "state");
                let still = if state == "pending" {
                    String::from("still pending")
                } else {
                    state.clone()
                };
                let (kind, title, note) = if *result {
                    roles.insert(*promise, String::from("the run's result"));
                    (
                        "result",
                        format!("the run's result is a promise that is {still}"),
                        NOTE_RESULT,
                    )
                } else if let Some(h) = held {
                    (
                        "end",
                        format!("the host holds it as {h:?}, a promise that is {still}"),
                        NOTE_END,
                    )
                } else {
                    (
                        "end",
                        format!("nothing waits for it yet (a promise that is {still})"),
                        NOTE_END,
                    )
                };
                Node {
                    kind,
                    title,
                    note,
                    record: Some(*promise),
                    detail: alloc::vec![format!("promise #{promise} (Object.Promise), {state}")],
                    ..Node::default()
                }
            }
            Link::Seen { promise } => Node {
                kind: "seen",
                title: String::from("from there on, as above"),
                note: NOTE_SEEN,
                record: Some(*promise),
                detail: alloc::vec![format!("promise #{promise}, shown above")],
                ..Node::default()
            },
        }
    }

    fn frame_node(&self, f: &Frame, waits_for: Option<String>, roles: &mut ById<String>) -> Node {
        let generator = matches!(
            self.constant(f.record, "kind").as_str(),
            "generator" | "asyncGenerator"
        );
        let name = self.frame_name(f);
        let top = f.function.is_some_and(|k| self.functions[k].top);
        let args: Vec<String> = f
            .args
            .iter()
            .map(|(n, v)| match n {
                Some(n) => format!("{n} = {}", self.short(*v)),
                None => self.short(*v),
            })
            .collect();
        let call = if top {
            name.clone()
        } else {
            format!("{name}({})", args.join(", "))
        };
        let src = self.frame_source(f);
        let snippet = src.as_ref().map(|s| snippet(&s.3, s.2));
        let at = match (&snippet, f.pos) {
            (Some(s), Some((l, _))) if !s.is_empty() => format!("at `{s}` (line {l})"),
            (_, Some((l, c))) => format!("at line {l}, column {c}"),
            _ => format!("at pc {}", f.pc),
        };
        let mut b = Vec::new();
        let mut uninit = Vec::new();
        let mut temps = Vec::new();
        for (n, v) in &f.locals {
            match (n.as_str(), v) {
                ("<state>", _) => {}
                (_, V::Imm(x)) if x.is_uninitialized() => uninit.push(n.clone()),
                ("<temp>", V::Imm(x)) if x.is_undefined() => {}
                ("<temp>", _) => temps.push(*v),
                (n, V::Imm(x)) if n.starts_with('<') && x.is_undefined() => {}
                (n, _) => b.push(self.binding(n, *v)),
            }
        }
        // A parameter captured by a closure is a local too (its box): shown once.
        let named: Vec<String> = b.iter().map(|x| x.name.clone()).collect();
        let mut bindings: Vec<Binding> = f
            .args
            .iter()
            .filter_map(|(n, v)| {
                n.as_ref()
                    .filter(|n| !named.contains(*n))
                    .map(|n| self.binding(n, *v))
            })
            .collect();
        let params: Vec<&str> = f.args.iter().filter_map(|(n, _)| n.as_deref()).collect();
        for x in &mut b {
            x.param = params.contains(&x.name.as_str());
        }
        // A body's parameters are not in its title: shown with its variables.
        for x in &mut bindings {
            x.param = !top;
        }
        bindings.extend(b);
        let captured: Vec<Binding> = f
            .captures
            .iter()
            .map(|(n, v)| self.binding(n, *v))
            .collect();
        let mut held = temps.clone();
        held.extend(f.stack.iter().copied());
        let doing = self.doing(&held, &bindings);
        let state = self.constant(f.record, "state");
        let title = match waits_for {
            _ if state == "start" => format!("{call} has not started yet"),
            None => format!("{call} is stopped {at}"),
            Some(w) if generator || w.is_empty() => format!("{call} is waiting {at}"),
            Some(w) => format!("{call} is waiting for {w}, {at}"),
        };
        let title = match &doing {
            Some(d) if title.len() + d.len() < 120 => format!("{title}, {d}"),
            _ => title,
        };
        let doing = doing.filter(|d| !title.ends_with(d.as_str()));
        roles.insert(
            f.record,
            format!("the activation of {call}: where it continues and its frame"),
        );
        if let Some(fr) = self.value(f.record, "frame").and_then(Self::rec) {
            roles.insert(fr, format!("the frame of {call}: callee, this, arguments, a header, its variables, its operand stack"));
        }
        if let Some(p) = f.promise {
            roles.insert(p, format!("the promise of {call}"));
        }
        for x in bindings.iter().chain(&captured) {
            if let Some(bx) = x.boxed {
                roles.insert_new(bx, || {
                    format!("the box of {}, shared with closures", x.name)
                });
            }
        }
        let mut detail = alloc::vec![format!(
            "activation #{} (Object.Generator, {}) continues at pc {}; its frame #{}",
            f.record,
            self.constant(f.record, "state"),
            f.pc,
            self.value(f.record, "frame")
                .and_then(Self::rec)
                .map_or_else(|| String::from("?"), |r| r.to_string()),
        )];
        if !f.slots.is_empty() {
            let s: Vec<String> = f.slots.iter().map(|&v| self.raw(v)).collect();
            detail.push(format!(
                "no source to name it by: this and its slots {}",
                s.join(", ")
            ));
        } else {
            let mut parts = alloc::vec![format!(
                "{} argument{}",
                f.args.len(),
                if f.args.len() == 1 { "" } else { "s" }
            )];
            parts.push(format!(
                "{} local{}",
                f.locals.len(),
                if f.locals.len() == 1 { "" } else { "s" }
            ));
            let stack: Vec<String> = held.iter().map(|&v| self.raw(v)).collect();
            if stack.is_empty() {
                parts.push(String::from("nothing on the operand stack"));
            } else {
                parts.push(format!("operand stack {}", stack.join(", ")));
            }
            detail.push(format!(
                "its frame holds callee, this, {}",
                parts.join(", ")
            ));
        }
        let boxes: Vec<String> = bindings
            .iter()
            .chain(&captured)
            .filter_map(|x| x.boxed.map(|b| format!("{} in #{b}", x.name)))
            .collect();
        if !boxes.is_empty() {
            detail.push(format!("boxes: {}", boxes.join(", ")));
        }
        if let Some(p) = f.promise {
            detail.push(format!("its promise #{p}"));
        }
        let next = if top { String::from("the body") } else { name };
        Node {
            kind: "frame",
            title,
            note: if generator {
                NOTE_GENERATOR
            } else {
                NOTE_FRAME
            },
            source: src,
            bindings,
            captured,
            uninit,
            doing,
            record: Some(f.record),
            detail,
            children: f
                .waiters
                .iter()
                .map(|w| self.link_node(w, Some(next.clone()), roles))
                .collect(),
        }
    }

    fn frame_name(&self, f: &Frame) -> String {
        match (f.function, f.callee) {
            (Some(k), _) => self.fn_name(k),
            (None, V::Base(b, _)) => self.builtin(b),
            (None, v) => self.short(v),
        }
    }

    fn frame_source(&self, f: &Frame) -> Option<(u32, u32, u32, String)> {
        let (l, c) = f.pos?;
        let script = self.snap.functions[f.function?].0;
        let text = core::str::from_utf8(self.source(script)?).ok()?;
        let line = text.split('\n').nth(l as usize - 1)?.trim_end_matches('\r');
        Some((script, l, c, line.to_string()))
    }

    fn binding(&self, n: &str, v: V) -> Binding {
        match v {
            V::Rec(r, _) if self.type_name(r) == BOX => Binding {
                name: n.to_string(),
                value: self.value(r, "VALUE").unwrap_or(V::Imm(Value::UNDEFINED)),
                boxed: Some(r),
                param: false,
            },
            _ => Binding {
                name: n.to_string(),
                value: v,
                boxed: None,
                param: false,
            },
        }
    }

    /// What the operand stack and the construct temporaries say the activation is in the
    /// middle of: a `for…of` loop, a call, an expression.
    fn doing(&self, held: &[V], bindings: &[Binding]) -> Option<String> {
        if held.is_empty() {
            return None;
        }
        let named = |v: V| -> Option<String> {
            let r = Self::rec(v)?;
            bindings
                .iter()
                .find(|b| Self::rec(b.value) == Some(r))
                .map(|b| b.name.clone())
        };
        // A loop's iterator (and its `next` after it), then what is above them.
        let mut lp = None;
        let mut from = 0;
        for (at, &v) in held.iter().enumerate() {
            let Some(r) = Self::rec(v) else { continue };
            from = at + 2;
            if self.type_name(r) == "Object.ArrayIterator" {
                let target = self.value(r, "target").unwrap_or(V::Imm(Value::UNDEFINED));
                let over = named(target).unwrap_or_else(|| self.short(target));
                let n = Self::rec(target).and_then(|t| match self.field(t, "elements") {
                    Some(F::Vs(e)) => Some(e.len()),
                    _ => None,
                });
                let k = self.int(r, "index").unwrap_or(0);
                let of = n.map_or_else(String::new, |n| format!(" of {n}"));
                lp = Some(format!("inside a for…of loop over {over}, at item {k}{of}"));
                break;
            }
            if self.type_name(r).contains("Iterator") || self.type_name(r) == GENERATOR {
                lp = Some(format!("inside a loop over {}", named(v).unwrap_or_else(|| self.short(v))));
                break;
            }
        }
        if lp.is_none() {
            from = 0;
        }
        let rest = held.get(from..).unwrap_or(&[]);
        let call = self.call_in(rest, &named);
        match (lp, call) {
            (Some(l), Some(c)) => Some(format!("{l}, {c}")),
            (Some(l), None) => Some(l),
            (None, Some(c)) => Some(c),
            (None, None) if rest.is_empty() => None,
            (None, None) => {
                let vs: Vec<String> = rest.iter().map(|&v| self.short(v)).collect();
                Some(format!("halfway through an expression, holding {}", vs.join(", ")))
            }
        }
    }

    /// The innermost call the operand stack `held` is in the middle of: the last callee
    /// on it, and the `this` after it.
    fn call_in(&self, held: &[V], named: &dyn Fn(V) -> Option<String>) -> Option<String> {
        for (k, &v) in held.iter().enumerate().rev() {
            let method = match v {
                V::Base(b, _) => Some(self.builtin(b)),
                V::Rec(r, _)
                    if matches!(
                        self.type_name(r),
                        FUNCTION | "Object.HostFunction" | "Object.BoundFunction"
                    ) =>
                {
                    Some(
                        self.short(v)
                            .trim_start_matches("function ")
                            .trim_end_matches(" (the host's)")
                            .to_string(),
                    )
                }
                V::Host(h) => Some(self.host_name(h)),
                _ => None,
            };
            let Some(m) = method else { continue };
            let short = m.rsplit('.').next().unwrap_or(&m).to_string();
            let this = held.get(k + 1).copied();
            let recv = match this {
                Some(V::Imm(x)) if x.is_undefined() => None,
                Some(V::Base(b, _)) => Some(self.builtin(b)),
                Some(t) => named(t),
                None => None,
            };
            return Some(match recv {
                Some(r) => format!("halfway through {r}.{short}(…)"),
                None => format!("halfway through a call of {short}(…)"),
            });
        }
        None
    }

    /// `v` as a short JavaScript literal (no ids, two levels deep).
    pub(super) fn short(&self, v: V) -> String {
        Js::new(self).js(v, 2, "")
    }

    /// `v` as the storage layer has it: records by id.
    pub(super) fn raw(&self, v: V) -> String {
        match v {
            V::Imm(x) => imm(x),
            V::Str(s) => quote(&self.text_of(s), STRING),
            V::Rec(r, _) => format!("#{r}"),
            V::Base(b, _) => self.builtin(b),
            V::Fn(f) => format!(
                "code of {}",
                if (f as usize) < self.functions.len() {
                    self.fn_name(f as usize)
                } else {
                    String::from("?")
                }
            ),
            V::Script(s) => format!("script {}", self.script_name(s)),
            V::Host(h) => self.host_name(h),
        }
    }

    /// A record in a few words: "an array (2 items)", "function fetchPage".
    pub(super) fn label(&self, r: u32) -> String {
        let ty = self.type_name(r);
        match ty {
            "Patch.Global" => String::from("the global object"),
            "Patch.GlobalLex" => String::from("the global scope's let and const"),
            "Patch.SymbolRegistry" => String::from("the Symbol.for registry"),
            PROMISE => format!("a promise ({})", self.constant(r, "state")),
            GENERATOR => format!(
                "a suspended {}",
                match self.constant(r, "kind").as_str() {
                    "generator" => "generator",
                    "asyncGenerator" => "async generator",
                    _ => "async function",
                }
            ),
            "ValueArray" => {
                let n = self.list(V::Rec(r, true)).len();
                format!("a list of {n} value{}", if n == 1 { "" } else { "s" })
            }
            "HostCall" => format!("open call {}", self.int(r, "ID").unwrap_or(0)),
            FUNCTION => self.short(V::Rec(r, false)),
            BOX => String::from("a box (a variable shared with closures)"),
            "Object.Array" => {
                let n = match self.field(r, "elements") {
                    Some(F::Vs(e)) => e.len(),
                    _ => 0,
                };
                format!("an array ({n} item{})", if n == 1 { "" } else { "s" })
            }
            "Object.Ordinary" => String::from("an object"),
            "Object.Map" | "Object.Set" => format!("a {}", &ty[7..]),
            "Object.BuiltinClosure" => match self.field(r, "builtin") {
                Some(F::Intr(k)) => format!("a builtin's function ({})", self.intrinsic(*k)),
                _ => String::from("a builtin's function"),
            },
            "Symbol" => String::from("a symbol"),
            t if t.starts_with("Object.Record.") => format!("{} (a builtin's record)", &t[14..]),
            t if t.starts_with("Object.") => format!("a {} object", &t[7..]),
            t => format!("a {t}"),
        }
    }

    /// Record `r` by what holds it, for the storage layer: "reactions of #12", "the
    /// global object".
    pub(super) fn held_as(&self, r: u32) -> String {
        match self.type_name(r) {
            t if t.starts_with("Patch.") => return self.label(r),
            FUNCTION => return self.short(V::Rec(r, false)),
            _ => {}
        }
        let Some(&h) = self.held_by.get(r as usize).and_then(|h| h.first()) else {
            return String::new();
        };
        let (t, fs) = if h == u32::MAX {
            (self.snap.root.0, &self.snap.root.1)
        } else {
            (
                self.snap.records[h as usize].0,
                &self.snap.records[h as usize].1,
            )
        };
        let ty = &self.snap.types[t as usize];
        for ((n, _), f) in ty.fields.iter().zip(fs) {
            let mut hit = false;
            super::walk_refs(core::slice::from_ref(f), &mut |x| hit |= x == r);
            if hit {
                let n = if n == "VALUES" {
                    "an element"
                } else {
                    n.as_str()
                };
                return if h == u32::MAX {
                    format!("{n} of the VM")
                } else {
                    format!("{n} of #{h}")
                };
            }
        }
        String::new()
    }

    /// A record's note: its role in the run when the tree gave it one, else its kind's.
    pub(super) fn note(&self, r: u32) -> String {
        let kind = match self.type_name(r) {
            "Patch.Global" | "Patch.GlobalLex" | "Patch.SymbolRegistry" => {
                "Only what the run changed is stored; a loading djs starts from its own."
            }
            PROMISE => "A promise: its state, its result, and what waits on it.",
            GENERATOR => "A suspended function: where it continues (pc) and its frame.",
            "ValueArray" => {
                "An internal list: a frame, a promise's reactions, the arguments of a call."
            }
            "HostCall" => {
                "An async host call waiting for its answer, by the id the host knows it by."
            }
            FUNCTION => {
                "A function object: its code is not stored, only its place in the script and its captured variables."
            }
            BOX => {
                "A variable shared with a closure: the function and its closures refer to this one box."
            }
            "Object.Array" => "An array: its elements and properties.",
            "Object.Ordinary" => "An object: its prototype and its properties in order.",
            "Object.Map" | "Object.Set" => {
                "Its entries in order; the hash index is not stored but rebuilt on load."
            }
            "Object.BuiltinClosure" => {
                "A function a builtin made, named by its kind, with its data."
            }
            t if t.starts_with("Object.Record.") => {
                "Internal state of a builtin, by its kind and its fields' names."
            }
            _ => "Data of the run: its kind and its fields by name.",
        };
        match self.roles.get(r) {
            Some(role) => format!("{}{}. {kind}", role[..1].to_uppercase(), &role[1..]),
            None => kind.to_string(),
        }
    }
}

/// The code at column `col` of `line` up to the end of its expression: "await
/// fetchPage(n)", "debugger;".
fn snippet(line: &str, col: u32) -> String {
    let rest: String = line.chars().skip(col as usize - 1).collect();
    let mut out = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut prev = ' ';
    for c in rest.chars() {
        if let Some(q) = quote {
            out.push(c);
            if c == q && prev != '\\' {
                quote = None;
            }
            prev = c;
            continue;
        }
        match c {
            '"' | '\'' | '`' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth == 0 => break,
            ')' | ']' | '}' => depth -= 1,
            ';' | ',' if depth == 0 => {
                if out == "debugger" && c == ';' {
                    out.push(';');
                }
                break;
            }
            '/' if prev == '/' && depth == 0 => {
                out.pop();
                break;
            }
            _ => {}
        }
        out.push(c);
        prev = c;
    }
    let open = depth > 0;
    let out = String::from(out.trim()) + if open { "…" } else { "" };
    let out = out.as_str();
    if out.chars().count() <= 40 {
        return out.to_string();
    }
    // Too long: what is inside the inner brackets left out.
    let mut short = String::new();
    let mut depth = 0;
    for c in out.chars() {
        match c {
            '(' | '[' | '{' => {
                depth += 1;
                if depth <= 2 {
                    short.push(c);
                }
                if depth == 2 {
                    short.push('…');
                }
            }
            ')' | ']' | '}' => {
                if depth <= 2 {
                    short.push(c);
                }
                depth -= 1;
            }
            c if depth < 2 => short.push(c),
            _ => {}
        }
    }
    clip(&short, 48)
}

// -------------------------------------------------------------------- JavaScript literals

/// Values as JavaScript literals. With `paths`, an object shown before is named by where
/// it was first seen (`same as page.items`).
pub(super) struct Js<'i, 'a> {
    i: &'i Inspection<'a>,
    pub paths: Option<ById<String>>,
    /// Records on the path being rendered (a cycle).
    open: Vec<u32>,
}

impl<'i, 'a> Js<'i, 'a> {
    pub fn new(i: &'i Inspection<'a>) -> Self {
        Js {
            i,
            paths: None,
            open: Vec::new(),
        }
    }

    pub fn tracking(i: &'i Inspection<'a>) -> Self {
        Js {
            i,
            paths: Some(ById::default()),
            open: Vec::new(),
        }
    }

    /// `v`, records `depth` levels deep; `path`: where it is (`page.items`), empty for
    /// none.
    pub fn js(&mut self, v: V, depth: usize, path: &str) -> String {
        let i = self.i;
        match v {
            V::Imm(x) => imm(x),
            V::Str(s) => quote(&i.text_of(s), STRING),
            V::Base(b, _) => i.builtin(b),
            V::Fn(f) => {
                if (f as usize) < i.functions.len() {
                    format!("the code of {}", i.fn_name(f as usize))
                } else {
                    String::from("code")
                }
            }
            V::Script(s) => format!("the script {}", i.script_name(s)),
            V::Host(h) => match i.snap.host.get(h as usize) {
                Some(HostEntry::Function(_)) => format!("{}()", i.host_name(h)),
                _ => i.host_name(h),
            },
            V::Rec(r, _) => self.record(r, depth, path),
        }
    }

    fn record(&mut self, r: u32, depth: usize, path: &str) -> String {
        let i = self.i;
        let ty = i.type_name(r).to_string();
        if ty == BOX {
            let v = i.value(r, "VALUE").unwrap_or(V::Imm(Value::UNDEFINED));
            return self.js(v, depth, path);
        }
        // Small things that need no expansion.
        match ty.as_str() {
            "Patch.Global" => return String::from("globalThis"),
            "Patch.GlobalLex" | "Patch.SymbolRegistry" | "HostCall" | "ValueArray" => {
                return format!("‹{}›", i.label(r));
            }
            "Object.BuiltinClosure" => {
                let k = match i.field(r, "builtin") {
                    Some(F::Intr(k)) => i.intrinsic(*k),
                    _ => String::new(),
                };
                return format!(
                    "‹{}›",
                    match k.as_str() {
                        "PromiseResolveFunction" => String::from("a promise's resolve function"),
                        "PromiseRejectFunction" => String::from("a promise's reject function"),
                        k if k.ends_with("Element") =>
                            format!("an element function of {}", combinator(k)),
                        _ => i.label(r),
                    }
                );
            }
            "Object.HostFunction" => {
                let name = match i.field(r, "props") {
                    Some(F::Props(ps)) => ps
                        .iter()
                        .find(|p| matches!(p.key, V::Str(s) if i.text_of(s) == "name"))
                        .map(|p| p.a),
                    _ => None,
                };
                return match name {
                    Some(V::Str(s)) => format!("function {} (the host's)", i.text_of(s)),
                    _ => String::from("a host function"),
                };
            }
            FUNCTION => {
                let f = match i.field(r, "template") {
                    Some(F::Fn(f)) if (*f as usize) < i.functions.len() => *f as usize,
                    _ => return String::from("function"),
                };
                let d = &i.functions[f];
                if d.top {
                    return i.fn_name(f);
                }
                if !d.name.is_empty() {
                    return format!("function {}", i.fn_name(f));
                }
                // An anonymous function: its source's start.
                let (script, start, end, ..) = i.snap.functions[f];
                return match i
                    .source(script)
                    .and_then(|s| s.get(start as usize..end as usize))
                    .and_then(|s| core::str::from_utf8(s).ok())
                {
                    Some(s) => clip(&s.split_whitespace().collect::<Vec<_>>().join(" "), 32),
                    None => i.fn_name(f),
                };
            }
            GENERATOR => {
                return match i.constant(r, "state").as_str() {
                    "completed" => String::from("‹a finished generator›"),
                    "start" => format!("‹a generator of {}, not started›", self.generator_name(r)),
                    _ => format!("‹{} of {}›", i.label(r), self.generator_name(r)),
                };
            }
            // A builtin's bookkeeping, not a value a script sees.
            t if t.starts_with("Object.Record.") => return format!("‹{}›", &t[14..]),
            t if t.starts_with("Object.IteratorHelper") => {
                return String::from("‹an iterator helper›");
            }
            "Object.Error" => {
                let prop = |n: &str| match i.field(r, "props") {
                    Some(F::Props(ps)) => ps
                        .iter()
                        .find(|p| matches!(p.key, V::Str(s) if i.text_of(s) == n))
                        .map(|p| p.a),
                    _ => None,
                };
                let msg = match prop("message") {
                    Some(V::Str(s)) => quote(&i.text_of(s), STRING),
                    _ => String::new(),
                };
                // Its class by its prototype (TypeError.prototype).
                let class = match i.value(r, "proto") {
                    Some(V::Base(b, _)) => i.builtin(b).trim_end_matches(".prototype").to_string(),
                    _ => String::from("Error"),
                };
                return format!("{class}({msg})");
            }
            _ => {}
        }
        if self.open.contains(&r) {
            return String::from("[Circular]");
        }
        if let Some(paths) = &mut self.paths {
            match paths.get(r) {
                Some(p) if !p.is_empty() => return format!("(same as {p})"),
                Some(_) => {}
                None => {
                    paths.insert(r, path.to_string());
                }
            }
        }
        if depth == 0 {
            return String::from(match ty.as_str() {
                "Object.Array" => "[…]",
                PROMISE => "Promise {…}",
                _ => "{…}",
            });
        }
        self.open.push(r);
        let d = depth - 1;
        let sub = |p: &str, k: &str| {
            if p.is_empty() {
                String::new()
            } else if is_ident(k) {
                format!("{p}.{k}")
            } else {
                format!("{p}[{k}]")
            }
        };
        let out = match ty.as_str() {
            PROMISE => {
                let state = i.constant(r, "state");
                match state.as_str() {
                    "pending" => String::from("Promise { <pending> }"),
                    s => {
                        let v = i.value(r, "result").unwrap_or(V::Imm(Value::UNDEFINED));
                        let x = self.js(v, d, "");
                        if s == "fulfilled" {
                            format!("Promise {{ {x} }}")
                        } else {
                            format!("Promise {{ <{s}> {x} }}")
                        }
                    }
                }
            }
            "Object.Array" => {
                let el = match i.field(r, "elements") {
                    Some(F::Vs(v)) => v.clone(),
                    _ => Vec::new(),
                };
                let mut parts: Vec<String> = Vec::new();
                for (k, v) in el.iter().enumerate() {
                    if k >= BREADTH || long(&parts) {
                        break;
                    }
                    parts.push(match v {
                        V::Imm(x) if x.is_hole() => String::from("<empty>"),
                        v => self.js(*v, d, &sub(path, &k.to_string())),
                    });
                }
                let shown = parts.len();
                if el.len() > shown {
                    parts.push(format!("… {} more", el.len() - shown));
                }
                parts.extend(self.props(r, d, BREADTH.saturating_sub(shown), path));
                format!("[{}]", parts.join(", "))
            }
            "Object.Map" | "Object.Set" => {
                let map = ty == "Object.Map";
                let entries = i.value(r, "entries").map(|e| i.list(e)).unwrap_or_default();
                let live: Vec<&[V]> = entries
                    .get(2..)
                    .unwrap_or(&[])
                    .chunks(2)
                    .filter(|kv| !matches!(kv[0], V::Imm(x) if x.is_hole()))
                    .collect();
                let mut parts = Vec::new();
                for kv in live.iter() {
                    if parts.len() >= BREADTH || long(&parts) {
                        break;
                    }
                    let k = self.js(kv[0], d, "");
                    if map {
                        let v = self.js(
                            kv.get(1).copied().unwrap_or(V::Imm(Value::UNDEFINED)),
                            d,
                            "",
                        );
                        parts.push(format!("{k} => {v}"));
                    } else {
                        parts.push(k);
                    }
                }
                if live.len() > parts.len() {
                    parts.push(format!("… {} more", live.len() - parts.len()));
                }
                let n = if map { "Map" } else { "Set" };
                if parts.is_empty() {
                    format!("{n}(0) {{}}")
                } else {
                    format!("{n}({}) {{ {} }}", live.len(), parts.join(", "))
                }
            }
            _ => {
                let class = ty
                    .strip_prefix("Object.")
                    .map(|t| t.strip_prefix("Record.").unwrap_or(t))
                    .unwrap_or(&ty);
                let class = if class == "Ordinary" {
                    String::new()
                } else {
                    format!("{class} ")
                };
                let proto_null = matches!(i.value(r, "proto"), Some(V::Imm(x)) if x.is_null())
                    && ty == "Object.Ordinary";
                let mut parts = self.props(r, d, BREADTH, path);
                parts.extend(self.slots(r, d));
                let pre = if proto_null {
                    "[Object: null prototype] "
                } else {
                    ""
                };
                if parts.is_empty() {
                    format!("{pre}{class}{{}}")
                } else {
                    format!("{pre}{class}{{ {} }}", parts.join(", "))
                }
            }
        };
        self.open.pop();
        out
    }

    fn generator_name(&self, r: u32) -> String {
        let i = self.i;
        let f = i
            .value(r, "frame")
            .map(|f| i.list(f))
            .and_then(|s| s.first().copied());
        match f
            .and_then(Inspection::rec)
            .and_then(|c| i.field(c, "template"))
        {
            Some(F::Fn(k)) if (*k as usize) < i.functions.len() => i.fn_name(*k as usize),
            _ => String::from("?"),
        }
    }

    pub fn key(&self, k: V) -> String {
        let i = self.i;
        match k {
            V::Str(s) => {
                let t = i.text_of(s);
                if is_ident(&t) { t } else { quote(&t, STRING) }
            }
            V::Rec(r, _) => {
                let d = match i.value(r, "DESCRIPTION") {
                    Some(V::Str(s)) => i.text_of(s),
                    _ => String::new(),
                };
                format!("[Symbol({d})]")
            }
            V::Base(b, _) => format!("[{}]", i.builtin(b)),
            V::Imm(v) => v
                .as_int()
                .map_or_else(|| String::from("?"), |n| n.to_string()),
            _ => String::from("?"),
        }
    }

    pub fn prop(&mut self, p: &P, depth: usize, path: &str) -> String {
        if p.bits & PROP_ACCESSOR != 0 {
            let g = !matches!(p.a, V::Imm(v) if v.is_undefined());
            let s = !matches!(p.b, V::Imm(v) if v.is_undefined());
            String::from(match (g, s) {
                (true, true) => "[Getter/Setter]",
                (true, false) => "[Getter]",
                _ => "[Setter]",
            })
        } else {
            self.js(p.a, depth, path)
        }
    }

    /// An object's own elements and properties, at most `max`.
    fn props(&mut self, r: u32, d: usize, max: usize, path: &str) -> Vec<String> {
        let i = self.i;
        let mut parts = Vec::new();
        let mut n = 0;
        let sub = |k: &str| {
            if path.is_empty() {
                String::new()
            } else if is_ident(k) {
                format!("{path}.{k}")
            } else {
                format!("{path}[{k}]")
            }
        };
        if i.type_name(r) != "Object.Array"
            && let Some(F::Vs(el)) = i.field(r, "elements")
        {
            for (k, v) in el.clone().iter().enumerate() {
                if matches!(v, V::Imm(x) if x.is_hole()) {
                    continue;
                }
                n += 1;
                if parts.len() < max && !long(&parts) {
                    let s = self.js(*v, d, &sub(&k.to_string()));
                    parts.push(format!("{k}: {s}"));
                }
            }
        }
        if let Some(F::Props(ps)) = i.field(r, "props") {
            for p in ps.clone() {
                n += 1;
                if parts.len() < max && !long(&parts) {
                    let k = self.key(p.key);
                    let s = self.prop(&p, d, &sub(&k));
                    parts.push(format!("{k}: {s}"));
                }
            }
        }
        if n > parts.len() {
            parts.push(format!("… {} more", n - parts.len()));
        }
        parts
    }

    /// An object's internal slots by name (a Date's time, a RegExp's source).
    fn slots(&mut self, r: u32, d: usize) -> Vec<String> {
        let i = self.i;
        let Some((t, fs)) = i.snap.records.get(r as usize) else {
            return Vec::new();
        };
        let ty = &i.snap.types[*t as usize];
        let mut parts = Vec::new();
        for ((n, _), f) in ty.fields.iter().zip(fs) {
            if matches!(n.as_str(), "proto" | "header" | "props" | "elements") {
                continue;
            }
            let s = match f {
                F::V(v) => self.js(*v, d, ""),
                F::U(u) | F::Buckets(u) | F::Flags(u) => u.to_string(),
                F::I(x) => x.to_string(),
                F::F64(x) => crate::num::to_string(*x).as_str().to_string(),
                F::Const(_) => i.constant(r, n),
                F::Intr(k) => i.intrinsic(*k),
                F::HostFn(k) => i.host_name(*k),
                _ => continue,
            };
            if long(&parts) {
                parts.push(String::from("…"));
                break;
            }
            parts.push(format!("[[{n}]]: {s}"));
        }
        parts
    }
}

// -------------------------------------------------------------------- the text

/// A byte count in plain words.
pub(super) fn bytes(n: usize) -> String {
    if n < 1000 {
        format!("{n} bytes")
    } else {
        let tenths = (n * 10 + 512) / 1024;
        format!("{}.{} KB", tenths / 10, tenths % 10)
    }
}

impl Inspection<'_> {
    /// The parts of the snapshot's size, in plain words: (part, bytes, what it is).
    pub(super) fn sizes(&self) -> Vec<(&'static str, usize, String)> {
        let sec = |n: &str| {
            let m = &self.snap.marks;
            m.iter()
                .position(|x| x.0 == n)
                .map_or(0, |k| m.get(k + 1).map_or(0, |e| e.1) - m[k].1)
        };
        let kinds = sec("types");
        let script = sec("scripts");
        let data = sec("strings") + sec("records") + sec("vm");
        let names = self.size - kinds - script - data;
        alloc::vec![
            (
                "kinds",
                kinds,
                format!(
                    "name the {} kinds of data and their fields, so a later djs can read it",
                    self.snap.types.len()
                )
            ),
            (
                "script",
                script,
                String::from(if self.snap.scripts.iter().all(|s| s.3.is_ok()) {
                    "is the script's text, compiled again when the snapshot loads"
                } else {
                    "name the scripts, some by a hash of their text, which the loading host gives"
                }),
            ),
            (
                "data",
                data,
                format!(
                    "is the run's own data: {} records and {} strings",
                    self.snap.records.len(),
                    self.snap.strings.len()
                )
            ),
            (
                "names",
                names,
                String::from(
                    "name what it refers to without storing it: builtins, host values, functions"
                )
            ),
        ]
    }

    pub(super) fn size_text(&self) -> String {
        let parts: Vec<String> = self
            .sizes()
            .iter()
            .filter(|p| p.1 > 0)
            .map(|(_, n, s)| format!("{} {s}", bytes(*n)))
            .collect();
        format!("{}: {}.", bytes(self.size), parts.join("; "))
    }

    /// The first line: the scripts and what the run is doing.
    pub(super) fn summary(&self) -> String {
        let scripts: Vec<String> = (0..self.snap.scripts.len() as u32)
            .map(|k| self.script_name(k))
            .collect();
        let what = if scripts.is_empty() {
            String::from("A snapshot")
        } else {
            scripts.join(", ")
        };
        let n = self.calls.len();
        let pauses = self
            .calls
            .iter()
            .filter(|c| c.function == "debugger")
            .count();
        let state = match self.run_state() {
            "waiting" if n == 0 => String::from("waiting, but not for the host"),
            "waiting" if pauses == n && n == 1 => String::from("paused at debugger;"),
            "waiting" => format!(
                "waiting for the host to answer {n} call{}",
                if n == 1 { "" } else { "s" }
            ),
            "threw" => String::from("the run has thrown"),
            _ => String::from("the run has ended"),
        };
        format!(
            "{what}, {state} ({}, {} records)",
            bytes(self.size),
            self.snap.records.len()
        )
    }

    /// The snapshot for people: the run in JavaScript terms first, each part's storage
    /// details on indented `·` lines after it; then what a loading djs must have, the size
    /// in words, and the records as djs stores them.
    pub fn text(&self) -> String {
        self.text_and_paths().0
    }

    /// The text, and where in it each record was first shown (`page.items`).
    pub(super) fn text_and_paths(&self) -> (String, ById<String>) {
        let mut t = Text {
            i: self,
            js: Js::tracking(self),
            out: String::new(),
        };
        t.all();
        (t.out, t.js.paths.take().unwrap_or_default())
    }
}

struct Text<'i, 'a> {
    i: &'i Inspection<'a>,
    js: Js<'i, 'a>,
    out: String,
}

/// The records the text lists in full.
const RECORDS: usize = 40;

impl Text<'_, '_> {
    fn line(&mut self, indent: usize, s: &str) {
        for _ in 0..indent {
            self.out.push(' ');
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn all(&mut self) {
        let i = self.i;
        self.line(0, &i.summary());
        for (k, n) in i.view.iter().enumerate() {
            self.out.push('\n');
            if k >= CALLS {
                self.line(0, &i.view[k].title.clone());
                continue;
            }
            self.node(0, n);
        }
        if i.view.len() > CALLS {
            self.line(0, &format!("(the first {CALLS} open calls in full)"));
        }
        if !i.others_view.is_empty() {
            self.out.push('\n');
            self.line(0, "also suspended, not waiting for the host:");
            for n in &i.others_view {
                self.node(2, n);
            }
        }
        self.out.push('\n');
        self.globals();
        self.needs();
        self.out.push('\n');
        let s = i.size_text();
        self.wrap(0, &s);
        self.records();
    }

    fn node(&mut self, indent: usize, n: &Node) {
        self.wrap(indent, &n.title);
        let b = indent + 2;
        if let Some((_, l, c, text)) = &n.source {
            let num = l.to_string();
            let w = num.len().max(4);
            self.line(b, &format!("{num:>w$} | {text}"));
            let pad: String = text
                .chars()
                .take(*c as usize - 1)
                .map(|ch| if ch == '\t' { '\t' } else { ' ' })
                .collect();
            self.line(b, &format!("{:>w$} | {pad}^", ""));
        }
        let mut parts = Vec::new();
        for x in n.bindings.iter().filter(|x| !x.param) {
            let v = self.js.js(x.value, DEPTH, &x.name);
            parts.push(format!("{} = {v}", x.name));
        }
        self.pack(b, &parts, "");
        let caps: Vec<String> = n
            .captured
            .iter()
            .map(|x| format!("{} = {}", x.name, self.js.js(x.value, DEPTH, &x.name)))
            .collect();
        self.pack(b, &caps, "from outside: ");
        if !n.uninit.is_empty() {
            self.line(b, &format!("not reached yet: {}", n.uninit.join(", ")));
        }
        if let Some(d) = &n.doing {
            self.wrap(b, d);
        }
        for d in &n.detail {
            self.wrap_prefixed(b, "· ", d);
        }
        if n.children.len() > 1 {
            self.line(indent, &format!("{} things wait for it:", n.children.len()));
            for c in &n.children {
                self.node(indent + 2, c);
            }
        } else if let Some(c) = n.children.first() {
            // An open call's chain under it; a chain's links one after another.
            self.node(if n.kind == "call" { indent + 2 } else { indent }, c);
        }
    }

    fn globals(&mut self) {
        let i = self.i;
        let Some(F::V(V::Rec(g, _))) = i.root_field("global") else {
            return;
        };
        let mut parts = Vec::new();
        if let Some(F::Props(ps)) = i.field(*g, "props") {
            for p in ps.clone() {
                let k = self.js.key(p.key);
                let v = self.js.prop(&p, 1, &k);
                // A host value under its own name: the name says it.
                parts.push(if v == k || v == format!("{k}()") {
                    v
                } else {
                    format!("{k} = {v}")
                });
            }
        }
        if let Some(F::Vs(del)) = i.field(*g, "deleted") {
            for k in del.clone() {
                parts.push(format!("{} deleted", self.js.key(k)));
            }
        }
        if !parts.is_empty() {
            self.pack(0, &parts, "on the global object: ");
        }
        if let Some(F::V(V::Str(s))) = i.root_field("hostData") {
            let d = i.text_of(*s);
            if !d.is_empty() {
                self.line(
                    0,
                    &format!("the host's own data of the run: {}", clip(&d, 64)),
                );
            }
        }
    }

    fn needs(&mut self) {
        let i = self.i;
        self.line(0, "named, not stored (a loading djs must have them):");
        let mut code = Vec::new();
        let mut iface = Vec::new();
        for (f, e) in i.snap.functions.iter().enumerate() {
            if e.3 {
                code.push(i.fn_name(f))
            } else {
                iface.push(i.fn_name(f))
            }
        }
        if !code.is_empty() {
            let s = format!(
                "the code of {}: the same bytecode, since {} stopped halfway (checked by hash)",
                and(&code),
                if code.len() == 1 { "it is" } else { "they are" }
            );
            self.wrap(2, &s);
        }
        if !iface.is_empty() {
            let s = format!(
                "for {}, held only as {}: the same parameters and captured variables (the code may differ)",
                and(&iface),
                if iface.len() == 1 {
                    "a value"
                } else {
                    "values"
                }
            );
            self.wrap(2, &s);
        }
        let b: Vec<String> = (0..i.snap.bases.len() as u32)
            .map(|k| i.builtin(k))
            .collect();
        if !b.is_empty() {
            self.pack(2, &b, "builtins every djs has: ");
        }
        let b: Vec<String> = (0..i.snap.intrinsics.len() as u32)
            .map(|k| i.intrinsic(k))
            .collect();
        if !b.is_empty() {
            self.pack(2, &b, "builtin functions by kind: ");
        }
        let h: Vec<String> = i
            .snap
            .host
            .iter()
            .enumerate()
            .map(|(k, e)| match e {
                HostEntry::Function(_) => format!("{}()", i.host_name(k as u32)),
                _ => i.host_name(k as u32),
            })
            .collect();
        if !h.is_empty() {
            self.pack(2, &h, "from the host: ");
        }
        for (f, &(script, start, end, pinned, hash, _)) in i.snap.functions.iter().enumerate() {
            let what = if pinned { "bytecode" } else { "interface" };
            self.line(
                2,
                &format!(
                    "· {} is {} {start}..{end}, {what} hash {hash:016x}",
                    i.fn_name(f),
                    i.script_name(script)
                ),
            );
        }
    }

    /// The storage layer: every record by id, kind, size and fields.
    fn records(&mut self) {
        let i = self.i;
        self.out.push('\n');
        self.line(
            0,
            &format!(
                "how djs stores it: {} records, each a kind with named fields",
                i.snap.records.len()
            ),
        );
        for r in 0..i.snap.records.len().min(RECORDS) {
            let (from, to) = i.spans[r];
            let fields = self.fields(r as u32);
            let fields = if fields.is_empty() {
                String::from("(empty)")
            } else {
                fields
            };
            // What the text above called it.
            let known = match self.js.paths.as_ref().and_then(|p| p.get(r as u32)) {
                Some(p) if !p.is_empty() => p.clone(),
                _ => i.roles.get(r as u32).map_or_else(
                    || i.held_as(r as u32),
                    |s| s.split(':').next().unwrap_or("").to_string(),
                ),
            };
            let known = if known.is_empty() {
                String::new()
            } else {
                format!(" {known}")
            };
            let s = format!(
                "#{r} {} ({} B){known}: {fields}",
                i.type_name(r as u32),
                to - from
            );
            self.line(2, &clip(&s, WIDTH + 12));
        }
        if i.snap.records.len() > RECORDS {
            self.line(
                2,
                &format!(
                    "… {} more (djs describe --json has every record)",
                    i.snap.records.len() - RECORDS
                ),
            );
        }
        let parts: Vec<String> = i
            .snap
            .marks
            .windows(2)
            .map(|w| format!("{} {}", w[0].0, w[1].1 - w[0].1))
            .collect();
        self.pack(2, &parts, "bytes per section: ");
    }

    /// A record's fields as stored, the empty ones left out.
    fn fields(&self, r: u32) -> String {
        let i = self.i;
        let Some((t, fs)) = i.snap.records.get(r as usize) else {
            return String::new();
        };
        let ty = &i.snap.types[*t as usize];
        let mut parts = Vec::new();
        for ((n, fty), f) in ty.fields.iter().zip(fs) {
            let s = match f {
                F::V(V::Imm(x)) if x.is_undefined() => continue,
                // The usual prototypes and header bits say little here.
                F::V(V::Base(..) | V::Imm(_)) if n == "proto" => continue,
                F::Flags(bits) if n == "header" => match fty {
                    Ty::Flags(names)
                        if names.first().is_some_and(|f| f == "extensible") && bits & 1 == 0 =>
                    {
                        String::from("not extensible")
                    }
                    _ => continue,
                },
                F::V(v) => i.raw(*v),
                F::U(0) | F::I(0) | F::Flags(0) | F::None | F::Buckets(_) => continue,
                F::U(u) => u.to_string(),
                F::I(x) => x.to_string(),
                F::F64(x) => crate::num::to_string(*x).as_str().to_string(),
                F::Vs(v) if v.is_empty() => continue,
                F::Vs(v) => format!(
                    "[{}]",
                    v.iter().map(|&x| i.raw(x)).collect::<Vec<_>>().join(", ")
                ),
                F::Tail(rows) if rows.is_empty() => continue,
                F::Tail(rows) => {
                    let cells: Vec<String> = rows
                        .iter()
                        .map(|row| {
                            let c: Vec<String> = row
                                .iter()
                                .map(|f| match f {
                                    F::V(v) => i.raw(*v),
                                    F::U(u) => u.to_string(),
                                    _ => String::from("…"),
                                })
                                .collect();
                            if c.len() == 1 {
                                c[0].clone()
                            } else {
                                format!("({})", c.join(" "))
                            }
                        })
                        .collect();
                    format!("[{}]", cells.join(", "))
                }
                F::Props(ps) if ps.is_empty() => continue,
                F::Props(ps) => {
                    let c: Vec<String> = ps
                        .iter()
                        .map(|p| format!("{} = {}", self.js.key(p.key), i.raw(p.a)))
                        .collect();
                    format!("{{{}}}", c.join(", "))
                }
                F::Fn(k) => format!(
                    "code of {}",
                    if (*k as usize) < i.functions.len() {
                        i.fn_name(*k as usize)
                    } else {
                        String::from("?")
                    }
                ),
                F::Intr(k) => i.intrinsic(*k),
                F::HostFn(k) => i.host_name(*k),
                F::Flags(bits) => match fty {
                    Ty::Flags(names) => names
                        .iter()
                        .enumerate()
                        .filter(|(k, _)| bits & (1 << k) != 0)
                        .map(|(_, n)| n.as_str())
                        .collect::<Vec<_>>()
                        .join("|"),
                    _ => bits.to_string(),
                },
                F::Const(_) => i.constant(r, n),
            };
            if s.is_empty() {
                continue;
            }
            parts.push(format!("{n} {s}"));
        }
        parts.join(", ")
    }

    /// `parts` after `head`, as many on a line as fit.
    fn pack(&mut self, indent: usize, parts: &[String], head: &str) {
        let mut cur = String::from(head);
        let mut fresh = true;
        let mut cont = false;
        for (k, p) in parts.iter().enumerate() {
            let piece = if k + 1 < parts.len() {
                format!("{p},")
            } else {
                p.clone()
            };
            let w = if cont { indent + 2 } else { indent };
            if !fresh && w + cur.chars().count() + 1 + piece.chars().count() > WIDTH {
                self.line(w, &cur);
                cur.clear();
                cont = true;
                fresh = true;
            }
            if !fresh {
                cur.push(' ');
            }
            cur.push_str(&piece);
            fresh = false;
        }
        if !fresh {
            self.line(if cont { indent + 2 } else { indent }, &cur);
        }
    }

    /// A sentence, wrapped at word boundaries.
    fn wrap(&mut self, indent: usize, s: &str) {
        let words: Vec<String> = s.split(' ').map(String::from).collect();
        let mut cur = String::new();
        let mut cont = false;
        for w in words {
            let at = if cont { indent + 2 } else { indent };
            if !cur.is_empty() && at + cur.chars().count() + 1 + w.chars().count() > WIDTH {
                self.line(at, &cur);
                cur.clear();
                cont = true;
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(&w);
        }
        if !cur.is_empty() {
            self.line(if cont { indent + 2 } else { indent }, &cur);
        }
    }

    fn wrap_prefixed(&mut self, indent: usize, prefix: &str, s: &str) {
        let before = self.out.len();
        self.wrap(indent, &format!("{prefix}{s}"));
        // Continuation lines line up after the prefix.
        let added = self.out[before..].to_string();
        let mut fixed = String::new();
        for (k, l) in added.lines().enumerate() {
            if k > 0 {
                fixed.push_str(&" ".repeat(prefix.chars().count()));
            }
            fixed.push_str(l);
            fixed.push('\n');
        }
        self.out.truncate(before);
        self.out.push_str(&fixed);
    }
}

/// Whether the parts of a literal are as long as one should get: the rest is left out.
fn long(parts: &[String]) -> bool {
    parts.iter().map(|p| p.chars().count() + 2).sum::<usize>() > 72
}

fn ordinal(n: i64) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn and(xs: &[String]) -> String {
    match xs {
        [] => String::new(),
        [a] => a.clone(),
        [a @ .., b] => format!("{} and {b}", a.join(", ")),
    }
}

pub(super) fn imm(x: Value) -> String {
    if x.is_undefined() {
        String::from("undefined")
    } else if x.is_null() {
        String::from("null")
    } else if x.is_hole() {
        String::from("<empty>")
    } else if x.is_uninitialized() {
        String::from("<not initialized>")
    } else if let Some(b) = x.as_bool() {
        String::from(if b { "true" } else { "false" })
    } else if let Some(n) = x.as_number() {
        crate::num::to_string(n).as_str().to_string()
    } else {
        String::from("?")
    }
}

pub(super) fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    c.next()
        .is_some_and(|f| f.is_alphabetic() || f == '_' || f == '$')
        && c.all(|x| x.is_alphanumeric() || x == '_' || x == '$')
}

/// `s` as a JSON string, clipped to `max` characters.
pub(super) fn quote(s: &str, max: usize) -> String {
    let mut out = String::from("\"");
    let n = s.chars().count();
    for c in s.chars().take(max) {
        esc(&mut out, c);
    }
    if n > max {
        out += &format!("…\" ({n} chars)");
        return out;
    }
    out.push('"');
    out
}

pub(super) fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

pub(super) fn esc(out: &mut String, c: char) {
    match c {
        '"' => out.push_str("\\\""),
        '\\' => out.push_str("\\\\"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
        c => out.push(c),
    }
}
