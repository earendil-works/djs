//! Single-use values written where they are used (0216's rules 3 and 4, D3001).
//!
//! A value the dump names once (`$dump.p1 = $djs.resume(…)`, read only by a
//! continuation's `await $dump.p1`) is written in place of that reference, its line left
//! out, where moving its evaluation there keeps the dump's order:
//!
//! - a *pure* value (a data literal, a function, a builtin iterator, a host value) may move
//!   to any later point of the load: making it again later gives the same value, nothing
//!   else holds it, and it reads no name a later statement could change;
//! - an *effectful* one (a host call reopened, an activation, a reaction, anything else)
//!   only to the first thing its use's line evaluates, with nothing effectful in between:
//!   a continuation's site operand where its prefix does nothing before the site, the
//!   first argument of a builtin call (`Promise.prototype.then.call($dump.a3, …)`), or the
//!   Script's completion value right after it.
//!
//! The uses are counted over everything the writer gets (lines, factories, activations,
//! handles) before it writes: a value counted more often than it is written stays named,
//! and a value read again after it was written in place is a refusal (a bug).

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::{IK, W, ident_part, ident_start, scan_spans};
use crate::buf::PushOut;

/// A value whose line is written and may still go where its one use is.
pub(super) struct Pend {
    name: String,
    rhs: String,
    /// The line list it was written in (its nesting) and its index there.
    depth: usize,
    idx: usize,
    effect: bool,
    /// The names its text binds itself (a continuation's): its reads of them are its own.
    bound: Vec<String>,
}

/// Where a reference stands, for what may be written there.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum At {
    /// Somewhere evaluated later than the value's line: pure values only.
    Later,
    /// The first thing the line evaluates: effectful values too.
    First,
}

#[derive(Default)]
pub(super) struct Inline {
    /// `$dump.X` references over the writer's input, by name (a value's own `$dump.X = `
    /// among them).
    pub uses: BTreeMap<String, u32>,
    pub pend: Vec<Pend>,
    /// The lines of each list being written (a callback's inside the top level's).
    pub stack: Vec<Vec<String>>,
    /// Per list, the index of its last effectful line.
    pub effects: Vec<Option<usize>>,
    /// The values written in place.
    pub done: Vec<String>,
}

/// Builtins a pure value may name (frozen builtins: nothing replaces them).
const PURE_NAMES: &[&str] = &[
    "undefined",
    "true",
    "false",
    "null",
    "NaN",
    "Infinity",
    "Map",
    "Set",
    "Date",
    "Symbol",
    "Object",
    "Array",
    "RegExp",
    "BigInt",
    "Error",
    "TypeError",
    "RangeError",
    "$djs",
];

/// Whether evaluating value text `rhs` later during the load gives the same value and does
/// nothing else.
pub(super) fn pure(rhs: &str) -> bool {
    // (A literal's other reads: values the dump wrote, builtins.)
    shape(rhs)
        && super::refs(rhs.trim())
            .names
            .iter()
            .all(|n| PURE_NAMES.contains(&n.as_str()))
}

/// Whether evaluating value text `rhs` only makes a value (reading names, calling nothing).
pub(super) fn shape(rhs: &str) -> bool {
    let t = rhs.trim();
    let starts = |p: &str| t.starts_with(p);
    starts("[")
        || starts("{")
        || starts("\"")
        || starts("'")
        || starts("`")
        || t.as_bytes().first().is_some_and(u8::is_ascii_digit)
        || starts("-") && t[1..].starts_with(|c: char| c.is_ascii_digit() || c == 'I')
        || starts("function")
        || starts("async function")
        || starts("class ")
        || starts("$djs.host(")
        || starts("$djs.source(")
        || starts("$djs.bytes(")
        || starts("$djs.advance(")
        || starts("new Map(")
        || starts("new Set(")
        || starts("new Date(")
        || matches!(
            t,
            "true" | "false" | "null" | "undefined" | "NaN" | "Infinity"
        )
        // (A name read: no getter, unlike a property; a value the dump made, or its
        // factory's member.)
        || super::is_ident(t)
        || t.strip_prefix("$dump.")
            .is_some_and(|r| r.split('.').all(|p| !p.is_empty() && super::is_ident(p)))
}

/// Whether value text `rhs` is a function or class without a name of its own (a name
/// would be inferred where it is written as an initializer or a property's value).
fn anonymous(rhs: &str) -> bool {
    let t = rhs.trim();
    let t = t.strip_prefix("async").map_or(t, str::trim_start);
    if let Some(r) = t.strip_prefix("function") {
        let r = r.trim_start().trim_start_matches('*').trim_start();
        return r.starts_with('(');
    }
    if let Some(r) = t.strip_prefix("class") {
        let r = r.trim_start();
        return r.starts_with('{') || r.starts_with("extends");
    }
    super::is_fn_text(t)
}

/// `rhs` as an operand: in parentheses unless it is one already.
pub(super) fn operand(rhs: &str) -> String {
    let t = rhs.trim();
    let b = t.as_bytes();
    let call_or_literal = b
        .first()
        .is_some_and(|&c| ident_start(c) || matches!(c, b'[' | b'{' | b'"' | b'\'' | b'`'))
        || b.first().is_some_and(u8::is_ascii_digit);
    let keyword = [
        "function", "async", "class", "new", "typeof", "void", "await", "yield",
    ]
    .iter()
    .any(|k| t.starts_with(k) && !t.as_bytes().get(k.len()).is_some_and(|&c| ident_part(c)));
    if call_or_literal && !keyword && !super::is_fn_text(t) {
        String::from(t)
    } else {
        cat!("(", t, ")")
    }
}

impl W<'_> {
    /// Counts the `$dump.X` references of everything the writer writes from.
    pub(super) fn count_uses(&mut self) {
        let ir = self.ir;
        let mut texts: Vec<&str> = Vec::new();
        // (An activation's or a factory's line is the format 2 one: the writer writes them
        // from the activation and the factory, counted below.)
        texts.extend(
            self.items
                .iter()
                .filter(|it| !matches!(it.kind, IK::Act(_) | IK::Factory(_)))
                .map(|it| it.text.as_str()),
        );
        for a in &ir.acts {
            texts.push_out(a.fexpr.as_str());
            texts.push_out(a.this.as_str());
            texts.extend(a.awaiting.as_deref());
            texts.extend(a.proto.as_deref());
            texts.extend(a.super_proto.as_deref());
            // (A temporary K evaluates again is not written, D2836.)
            texts.extend(
                a.slots
                    .iter()
                    .filter(|s| !a.rereads.contains(&s.name))
                    .map(|s| s.value.as_str()),
            );
            for (_, v, p) in &a.queue {
                texts.push_out(v.as_str());
                texts.push_out(p.as_str());
            }
            for u in &a.usings {
                texts.extend(u.triples.iter().flat_map(|t| [t.0.as_str(), t.1.as_str()]));
                texts.extend(u.state.iter().map(|s| s.1.as_str()));
            }
        }
        for f in &ir.factories {
            for sc in &f.scopes {
                texts.extend(sc.other.iter().map(String::as_str));
                for v in &sc.vars {
                    texts.push_out(v.line.as_str());
                    texts.extend(v.init.as_deref());
                }
                texts.extend(sc.props.iter().map(|p| p.text.as_str()));
            }
        }
        texts.extend(ir.handles.iter().map(|h| h.1.as_str()));
        let mut uses: BTreeMap<String, u32> = BTreeMap::new();
        for t in texts {
            for (s, e) in scan_spans(t) {
                *uses.entry(String::from(&t[s + 6..e])).or_insert(0) += 1;
            }
        }
        self.inl.uses = uses;
    }

    /// Whether `$dump.name` is read once besides its own line (`own`: whether its line is
    /// among the counted texts).
    pub(super) fn single(&self, name: &str, own: bool) -> bool {
        self.inl.uses.get(name).copied() == Some(1 + u32::from(own))
    }

    /// Starts a list of lines (a callback's, or the top level).
    pub(super) fn open_list(&mut self) {
        self.inl.stack.push(Vec::new());
        self.inl.effects.push(None);
    }

    /// Ends the current list: its lines (those written in place left out).
    pub(super) fn close_list(&mut self) -> Vec<String> {
        let depth = self.inl.stack.len() - 1;
        self.inl.pend.retain(|p| p.depth != depth);
        self.inl.effects.pop();
        let lines = self.inl.stack.pop().unwrap_or_default();
        lines.into_iter().filter(|l| !l.is_empty()).collect()
    }

    /// Writes line `text` of the current list; `effect`: whether it does more than make a
    /// value; `def`: the value it names and its text, a candidate for its one use.
    pub(super) fn put_line(
        &mut self,
        text: String,
        effect: bool,
        def: Option<(String, String, bool)>,
    ) {
        self.put_line_binding(text, effect, def, Vec::new());
    }

    /// [`W::put_line`] for a value whose text binds names `bound` itself.
    pub(super) fn put_line_binding(
        &mut self,
        text: String,
        effect: bool,
        def: Option<(String, String, bool)>,
        bound: Vec<String>,
    ) {
        let depth = self.inl.stack.len() - 1;
        let idx = self.inl.stack[depth].len();
        self.inl.stack[depth].push_out(text);
        if effect {
            self.inl.effects[depth] = Some(idx);
        }
        if let Some((name, rhs, eff)) = def {
            self.inl.pend.push_out(Pend {
                name,
                rhs,
                depth,
                idx,
                effect: eff,
                bound,
            });
        }
    }

    /// `text` with the references to values waiting for their one use written in place
    /// where that keeps the order: `first` is where in `text` the line's first evaluation
    /// is (the one place an effectful value may go), if anywhere.
    ///
    /// `in_k`: `text` is a value of a continuation's text, whose bindings could hide the
    /// names a value reads: only values that read none but builtins go there.
    pub(super) fn inline_refs(&mut self, text: &str, first: Option<usize>, in_k: bool) -> String {
        self.inline_refs_reading(text, first, if in_k { PURE_NAMES } else { &[] }, !in_k)
    }

    /// A continuation's site operand `text`, its first evaluation at `first` (if so), where
    /// K may bind the names `scope`: a value may go there that reads, besides builtins and
    /// what it binds itself, no name of `scope` (and not `this`, `arguments`, `super`).
    pub(super) fn inline_site(
        &mut self,
        text: &str,
        first: Option<usize>,
        scope: &[String],
    ) -> String {
        if self.inl.pend.is_empty() || !text.contains("$dump.") {
            return String::from(text);
        }
        let spans = scan_spans(text);
        let mut out = String::new();
        let mut at = 0;
        for (s, e) in spans {
            let name = &text[s + 6..e];
            let Some(p) = self.inl.pend.iter().position(|p| p.name == name) else {
                continue;
            };
            let where_ = if first == Some(s) {
                At::First
            } else {
                At::Later
            };
            let pd = &self.inl.pend[p];
            let reads_ok = super::refs(&pd.rhs).names.iter().all(|n| {
                !matches!(n.as_str(), "this" | "arguments" | "super" | "new")
                    && (PURE_NAMES.contains(&n.as_str())
                        || pd.bound.contains(n)
                        || !scope.contains(n))
            });
            if !reads_ok || !self.may_inline(p, where_, &text[..s]) {
                continue;
            }
            let p = self.inl.pend.remove(p);
            self.inl.stack[p.depth][p.idx] = String::new();
            out.push_str(&text[at..s]);
            out.push_str(&operand(&p.rhs));
            at = e;
            self.inl.done.push_out(p.name);
        }
        out.push_str(&text[at..]);
        out
    }

    /// [`W::inline_refs`] where the values written in place may read the names `names`
    /// only, or any (`any`).
    fn inline_refs_reading(
        &mut self,
        text: &str,
        first: Option<usize>,
        names: &[&str],
        any: bool,
    ) -> String {
        if self.inl.pend.is_empty() || !text.contains("$dump.") {
            return String::from(text);
        }
        let spans = scan_spans(text);
        let mut out = String::new();
        let mut at = 0;
        for (s, e) in spans {
            let name = &text[s + 6..e];
            // (`$dump.X.key`: a factory's member, never a candidate.)
            let Some(p) = self.inl.pend.iter().position(|p| p.name == name) else {
                continue;
            };
            let where_ = if first == Some(s) {
                At::First
            } else {
                At::Later
            };
            if !self.may_inline(p, where_, &text[..s])
                || !any
                    && !super::refs(&self.inl.pend[p].rhs)
                        .names
                        .iter()
                        .all(|n| names.contains(&n.as_str()))
            {
                continue;
            }
            let p = self.inl.pend.remove(p);
            self.inl.stack[p.depth][p.idx] = String::new();
            out.push_str(&text[at..s]);
            out.push_str(&operand(&p.rhs));
            at = e;
            self.inl.done.push_out(p.name);
        }
        out.push_str(&text[at..]);
        out
    }

    fn may_inline(&self, p: usize, at: At, before: &str) -> bool {
        let p = &self.inl.pend[p];
        let depth = self.inl.stack.len() - 1;
        if p.effect {
            // Nothing effectful since its line, and this the first thing evaluated.
            return at == At::First && p.depth == depth && self.inl.effects[depth] == Some(p.idx);
        }
        // A function without a name where one would be inferred keeps its line.
        let prev = before.trim_end();
        let infers = (prev.ends_with('=') && !prev.ends_with("==") && !prev.ends_with("=>"))
            || prev.ends_with(':');
        !(infers && anonymous(&p.rhs))
    }

    /// Whether item `kind`'s line does more than make a value.
    pub(super) fn effectful(kind: &IK, text: &str) -> bool {
        match kind {
            IK::Def(_) => !shape(def_rhs(text).unwrap_or("")),
            IK::Decl(_) => {
                // (`const x = value;`.)
                let rhs = text
                    .split_once(" = ")
                    .map_or("", |x| x.1)
                    .trim_end_matches(';');
                !shape(rhs)
            }
            IK::Factory(_) | IK::FnDecl => false,
            IK::Stmt | IK::Act(_) => true,
        }
    }

    /// Slot `name`'s value `v` of activation `a`, with the values that wait for their use
    /// written in place (once per slot: K's generator may ask again).
    pub(super) fn inlined_slot(&mut self, a: usize, name: &str, v: &str) -> String {
        let key = (a, String::from(name));
        if let Some(s) = self.vcache.get(&key) {
            return s.clone();
        }
        // (A value that reads a name would be read before the continuation, `pre`.)
        let s = self.inline_refs_reading(v, None, &["undefined", "true", "false", "null"], false);
        self.vcache.insert(key, s.clone());
        s
    }

    /// Fails where a value written in place is still read by name.
    pub(super) fn check_inlined(&self, text: &str) -> Result<(), String> {
        if self.inl.done.is_empty() {
            return Ok(());
        }
        for (s, e) in scan_spans(text) {
            let name = &text[s + 6..e];
            if self.inl.done.iter().any(|d| d == name) {
                return Err(cat!(
                    "plain dumps: $dump.",
                    name,
                    " is written in place and read again (a bug)"
                ));
            }
        }
        Ok(())
    }
}

/// The value of a `$dump.X = value;` line.
pub(super) fn def_rhs(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("$dump.")?;
    let (_, v) = rest.split_once(" = ")?;
    Some(v.strip_suffix(';').unwrap_or(v))
}

/// Where in a line `$dump.X = <callee>(…)` its first evaluation is a reference: the first
/// argument of a builtin's call (`Promise.prototype.then.call($dump.a3, …)`), or the
/// value itself.
pub(super) fn first_eval(text: &str) -> Option<usize> {
    let rest = text.strip_prefix("$dump.")?;
    let (_, v) = rest.split_once(" = ")?;
    let at = text.len() - v.len();
    if v.starts_with("$dump.") {
        return Some(at);
    }
    let open = v.find('(')?;
    let callee = &v[..open];
    let builtin = !callee.is_empty()
        && callee
            .split('.')
            .all(|p| !p.is_empty() && p.bytes().all(ident_part))
        && !callee.starts_with("$dump")
        && !callee.starts_with("$djs");
    (builtin && v[open + 1..].starts_with("$dump.")).then_some(at + open + 1)
}
