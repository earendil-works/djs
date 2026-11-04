//! Operands a plain continuation evaluates again instead of keeping (D2836, 0207).
//!
//! When a statement suspends inside a call's arguments, the callee, `this` and the
//! arguments before the site were already evaluated: the frame holds them as
//! temporaries, which a continuation keeps as constants (`$djs_R($djs_t0, $djs_t1, [...])`).
//! Plain dumps are written only with frozen builtins, so an operand that is pure to
//! evaluate again and gives the saved value on the dumped state can be written as the
//! original text (`results.push(await p)`):
//!
//! - an identifier whose binding (a frame binding of that name, unique at the site; else a
//!   variable the function captured; else a global's data property) holds the saved
//!   value now;
//! - a callee `b.name` whose base `b` is such an identifier holding the saved `this`, and
//!   whose property `name` is found on a frozen object (a builtin): the receiver has no
//!   own property of that name (or is frozen itself), and every object on the way is
//!   frozen, so nothing the dump restores later can change the lookup.
//!
//! Everything else keeps its temporary: user functions read through user objects, own
//! properties of user objects, getters, proxies.

use alloc::string::String;
use alloc::vec::Vec;

use super::*;
use crate::compiler::{Holds, NamedSlot, NodeKind, Role, Syntax};

/// An operand's text that may be read again.
enum Read<'t> {
    Ident(&'t str),
    /// `base.name`.
    Member(&'t str, &'t str),
}

fn parse(text: &str) -> Option<Read<'_>> {
    let t = text.trim();
    if is_ident(t) {
        return Some(Read::Ident(t));
    }
    let (b, k) = t.split_once('.')?;
    (is_ident(b) && is_ident(k)).then_some(Read::Member(b, k))
}

/// Strict equality of a saved operand and a value read now.
fn same(h: &Heap, a: Value, b: Value) -> bool {
    if a == b {
        return true;
    }
    match (a.as_string(), b.as_string()) {
        (Some(x), Some(y)) => text_of(h, x) == text_of(h, y),
        _ => matches!((a.as_number(), b.as_number()), (Some(x), Some(y)) if x == y),
    }
}

impl Dumper<'_> {
    /// The temporaries of activation frame `named` (values by `val`) that its
    /// continuation may evaluate again from `ftext` (D2836): see the module comment.
    pub(super) fn rereads(
        &self,
        f: Option<Obj>,
        syntax: &Syntax,
        ftext: &str,
        named: &[NamedSlot],
        val: &dyn Fn(&NamedSlot) -> Value,
    ) -> Vec<String> {
        let slot = |name: &str| named.iter().find(|x| x.name == name).map(val);
        let text = |a: u32, b: u32| ftext.get(a as usize..b as usize).unwrap_or("");
        let mut out = Vec::new();
        for n in &syntax.nodes {
            let call = matches!(n.kind, NodeKind::Call { optional: false });
            if !call && n.kind != NodeKind::New {
                continue;
            }
            // The callee and `this` of a call.
            let callee = cat!("call.callee@", n.start);
            let this = cat!("call.this@", n.start);
            if call
                && let (Some(cv), Some(tv)) = (slot(&callee), slot(&this))
                && let Some(c) = n.parts.iter().find(|p| p.role == Role::Callee)
            {
                let ok = match parse(text(c.start, c.end)) {
                    Some(Read::Ident(name)) => {
                        tv.is_undefined()
                            && self
                                .read_name(f, named, val, name)
                                .is_some_and(|v| same(self.h, v, cv))
                    }
                    Some(Read::Member(b, k)) => {
                        self.read_name(f, named, val, b)
                            .is_some_and(|v| same(self.h, v, tv))
                            && self.frozen_lookup(tv, k, cv)
                    }
                    None => false,
                };
                if ok {
                    out.push(callee);
                    out.push(this);
                }
            }
            // Arguments evaluated before the site.
            let construct = if call { "call.arg@" } else { "new.arg@" };
            for p in n.parts.iter().filter(|p| p.role == Role::Arg) {
                let name = cat!(construct, p.start);
                let Some(v) = slot(&name) else { continue };
                if let Some(Read::Ident(id)) = parse(text(p.start, p.end))
                    && self
                        .read_name(f, named, val, id)
                        .is_some_and(|now| same(self.h, now, v))
                {
                    out.push(name);
                }
            }
        }
        out
    }

    /// The value identifier `name` reads in the continuation, if it reads one without
    /// side effects: the frame's binding, a variable `f` captured, a global.
    fn read_name(
        &self,
        f: Option<Obj>,
        named: &[NamedSlot],
        val: &dyn Fn(&NamedSlot) -> Value,
        name: &str,
    ) -> Option<Value> {
        let h = self.h;
        if name == "arguments" || name == "undefined" {
            return (name == "undefined").then_some(Value::UNDEFINED);
        }
        let bindings: Vec<&NamedSlot> = named
            .iter()
            .filter(|x| {
                matches!(x.holds, Holds::Binding)
                    && (x.name == name || x.name.rsplit_once('@').is_some_and(|(b, _)| b == name))
            })
            .collect();
        match bindings.as_slice() {
            [x] if x.name == name => {
                let v = val(x);
                return Some(match v.as_internal() {
                    Some(b) if h.cell_kind(b) == crate::heap::cells::BoxCell::KIND => {
                        h.arena().get(b, crate::heap::cells::BoxCell::VALUE)
                    }
                    _ => v,
                });
            }
            [] => {}
            // Several of the name in scope: which one the text reads is the compiler's.
            _ => return None,
        }
        if let Some(v) = f.and_then(|f| captured_value(h, f, name)) {
            return Some(v);
        }
        // A global: the global lexical record's binding, else the global object's data
        // property.
        let lex = self.vm.global_lexical();
        if let Some(v) = super::origin::own(h, lex, name) {
            return Some(v);
        }
        super::origin::own(h, self.vm.global(), name)
    }

    /// Whether `recv.key` finds `want` on a frozen object through frozen objects only
    /// (after `recv` itself, which may be the session's if it has no own `key`).
    fn frozen_lookup(&self, recv: Value, key: &str, want: Value) -> bool {
        let h = self.h;
        let Some(mut o) = recv.as_object() else {
            return false;
        };
        let mut first = true;
        loop {
            if h.object_class(o) == ObjectClass::Proxy || (!first && !h.frozen(o)) {
                return false;
            }
            let k = h
                .own_keys(o)
                .into_iter()
                .find(|&k| h.atom_symbol(k).is_none() && atom_text(h, k) == key);
            if let Some(k) = k {
                return h.frozen(o)
                    && matches!(h.get_own_property(o, k),
                        Some(Property::Data { value, .. }) if same(h, value, want));
            }
            first = false;
            match h.get_prototype_of(o) {
                Some(p) => o = p,
                None => return false,
            }
        }
    }
}
