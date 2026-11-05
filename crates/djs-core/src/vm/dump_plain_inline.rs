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

/// The names a value written in place may read.
#[derive(Clone, Copy)]
enum Reads<'a> {
    Any,
    Only(&'a [&'a str]),
    /// None that a continuation binds where the reference is (`scope`).
    Scope(&'a [String]),
}

#[derive(Default)]
pub(super) struct Inline {
    /// `$dump.X` references over the writer's input, by name (a value's own `$dump.X = `
    /// among them).
    pub uses: BTreeMap<String, u32>,
    pub pend: Vec<Pend>,
    /// The lines of each list being written (a callback's inside the top level's).
    pub stack: Vec<Vec<String>>,
    /// Per list, the indexes of its effectful lines (those written in place left out).
    pub effects: Vec<Vec<usize>>,
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
        self.inl.effects.push(Vec::new());
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
            self.inl.effects[depth].push_out(idx);
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
    /// is (where effectful values may go, [`first_run`]), if anywhere.
    ///
    /// `in_k`: `text` is a value of a continuation's text, whose bindings could hide the
    /// names a value reads: only values that read none but builtins go there.
    pub(super) fn inline_refs(&mut self, text: &str, first: Option<usize>, in_k: bool) -> String {
        if in_k {
            self.inline_with(text, first, Reads::Only(PURE_NAMES))
        } else {
            self.inline_with(text, first, Reads::Any)
        }
    }

    /// A continuation's site operand `text`, its first evaluation at `first` (if so), where
    /// K may bind the names `scope`: a value may go there that reads, besides builtins and
    /// what it binds itself, no name of `scope` (and not `this`, `arguments`, `super`, or
    /// K's own `$djs_` helpers).
    pub(super) fn inline_site(
        &mut self,
        text: &str,
        first: Option<usize>,
        scope: &[String],
    ) -> String {
        self.inline_with(text, first, Reads::Scope(scope))
    }

    fn inline_with(&mut self, text: &str, first: Option<usize>, reads: Reads) -> String {
        if self.inl.pend.is_empty() || !text.contains("$dump.") {
            return String::from(text);
        }
        let spans = scan_spans(text);
        let run = first.map_or_else(Vec::new, |f| first_run(text, f, &spans));
        let depth = self.inl.stack.len() - 1;
        // Per reference: the pending value written there, if one may be.
        let mut take: Vec<Option<usize>> = Vec::new();
        for &(s, e) in &spans {
            // (`$dump.X.key`: a factory's member, never a candidate.)
            let name = &text[s + 6..e];
            let Some(p) = self.inl.pend.iter().position(|p| p.name == name) else {
                take.push_out(None);
                continue;
            };
            let pd = &self.inl.pend[p];
            let names = super::refs(&pd.rhs).names;
            let reads_ok = match reads {
                Reads::Any => true,
                Reads::Only(only) => names.iter().all(|n| only.contains(&n.as_str())),
                Reads::Scope(scope) => names.iter().all(|n| {
                    !matches!(n.as_str(), "this" | "arguments" | "super" | "new")
                        && !n.starts_with("$djs_")
                        && (PURE_NAMES.contains(&n.as_str())
                            || pd.bound.contains(n)
                            || !scope.contains(n)
                            || only_keys(&pd.rhs, n))
                }),
            };
            let ok = reads_ok
                && if pd.effect {
                    // (In the run of first evaluations, of the list being written; the
                    // run's order is checked below.)
                    run.contains(&s) && pd.depth == depth
                } else {
                    // A function without a name where one would be inferred keeps its
                    // line.
                    let prev = text[..s].trim_end();
                    let infers =
                        (prev.ends_with('=') && !prev.ends_with("==") && !prev.ends_with("=>"))
                            || prev.ends_with(':');
                    !(infers && anonymous(&pd.rhs))
                };
            take.push_out(ok.then_some(p));
        }
        // The effectful values of the run go there all or none: each must be the next of
        // the list's effects, the last one its last (nothing effectful comes between
        // their lines and this one).
        let mut eff: Vec<usize> = Vec::new();
        let mut all = true;
        for (k, &(s, e)) in spans.iter().enumerate() {
            if !run.contains(&s) {
                continue;
            }
            let name = &text[s + 6..e];
            if let Some(pd) = self.inl.pend.iter().find(|p| p.name == name && p.effect) {
                all &= take[k].is_some();
                eff.push_out(pd.idx);
            }
        }
        let tail: Vec<usize> = match eff.first() {
            Some(&f) => self.inl.effects[depth]
                .iter()
                .copied()
                .filter(|&i| i >= f)
                .collect(),
            None => Vec::new(),
        };
        if !all || tail != eff {
            for t in &mut take {
                if t.is_some_and(|p| self.inl.pend[p].effect) {
                    *t = None;
                }
            }
        }
        let names: Vec<Option<String>> = take
            .iter()
            .map(|t| t.map(|p| self.inl.pend[p].name.clone()))
            .collect();
        let mut out = String::new();
        let mut at = 0;
        for (k, &(s, e)) in spans.iter().enumerate() {
            let Some(name) = &names[k] else {
                continue;
            };
            let Some(p) = self.inl.pend.iter().position(|p| p.name == *name) else {
                continue;
            };
            let p = self.inl.pend.remove(p);
            self.inl.stack[p.depth][p.idx] = String::new();
            self.inl.effects[p.depth].retain(|&i| i != p.idx);
            out.push_str(&text[at..s]);
            out.push_str(&operand(&p.rhs));
            at = e;
            self.inl.done.push_out(p.name);
        }
        out.push_str(&text[at..]);
        out
    }

    /// Activation `a`'s site operand where it is a value the last part of its callback
    /// (`b`, the list being written) makes for it alone (D3002): K runs that part right
    /// before its site, and where nothing effectful comes between (no code of K's blocks
    /// below `b`), the value is made at the site instead, as its first evaluation.
    pub(super) fn site_from_callback(&mut self, a: usize, b: usize) {
        let ir = self.ir;
        let act = &ir.acts[a];
        let Some(aw) = act.awaiting.as_deref() else {
            return;
        };
        let Some(name) = aw.strip_prefix("$dump.").filter(|n| super::is_ident(n)) else {
            return;
        };
        // (As `awaiting` asks for it: K called on its own line, its site its first
        // evaluation after its prefix.)
        if act.kind != crate::interp::run::suspend::ASYNC
            || self.acts[a].home.is_some()
            || self
                .placed
                .iter()
                .any(|x| matches!(x.0, super::Pl::Block(a2, i) if a2 == a && i > b))
        {
            return;
        }
        if !self.inl.pend.iter().any(|p| p.name == name) {
            return;
        }
        let scope = self.k_scope(a);
        let v = self.inline_site(aw, Some(0), &scope);
        if v != aw {
            self.acts[a].site_val = Some(v);
        }
    }

    /// The names activation `a`'s continuation may bind anywhere: any declaration of its
    /// function, any slot's name (and its own `$djs_` helpers).
    pub(super) fn k_scope(&self, a: usize) -> Vec<String> {
        let act = &self.ir.acts[a];
        let mut scope: Vec<String> = Vec::new();
        for nd in &act.syntax.nodes {
            scope.extend(nd.decls.iter().map(|d| d.name.clone()));
        }
        for sl in &act.slots {
            scope.push_out(String::from(sl.name.split('@').next().unwrap_or(&sl.name)));
        }
        scope
    }

    /// Whether the lines of a part of activation `a`'s callback may be written in its
    /// continuation where it calls them (D3002): they read no name K may bind, and nothing
    /// that is K's own there (`this`, `arguments`, `super`, `new.target`).
    pub(super) fn callback_in_k(&self, a: usize, lines: &[String]) -> bool {
        let scope = self.k_scope(a);
        lines.iter().all(|l| {
            !l.contains("new.target")
                && super::refs(l).names.iter().all(|n| {
                    !matches!(n.as_str(), "this" | "arguments" | "super")
                        && !n.starts_with("$djs_")
                        && (!scope.contains(n) || only_keys(l, n))
                })
        })
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
        // (A value that reads a name K binds would be read before the continuation,
        // `pre`.)
        let scope = self.k_scope(a);
        let s = self.inline_with(v, None, Reads::Scope(&scope));
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

/// Whether `text` names `name` only as an object literal's key (`{ path: "b.md" }`): the
/// references `refs` finds err on the side of keys.
pub(super) fn only_keys(text: &str, name: &str) -> bool {
    let b = text.as_bytes();
    let mut any = false;
    for (at, _) in text.match_indices(name) {
        let end = at + name.len();
        if at > 0 && ident_part(b[at - 1]) || b.get(end).is_some_and(|&c| ident_part(c)) {
            continue;
        }
        any = true;
        let before = text[..at].trim_end();
        let after = text[end..].trim_start();
        let key = (before.ends_with('{') || before.ends_with(','))
            && after.starts_with(':')
            && !after.starts_with("::");
        if !key {
            return false;
        }
    }
    any
}

/// The value of a `$dump.X = value;` line.
pub(super) fn def_rhs(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("$dump.")?;
    let (_, v) = rest.split_once(" = ")?;
    Some(v.strip_suffix(';').unwrap_or(v))
}

/// Where in a line `$dump.X = value;` or `void value;` its value starts (its first
/// evaluation, [`first_run`]).
pub(super) fn first_eval(text: &str) -> Option<usize> {
    let v = match text.strip_prefix("void ") {
        Some(v) => v,
        None => text.strip_prefix("$dump.")?.split_once(" = ")?.1,
    };
    Some(text.len() - v.len())
}

/// The references of `text` its evaluation from `first` evaluates first, one after another
/// with nothing effectful between (`spans`: its references): the value itself
/// (`$dump.a3.then(…)`: its receiver), or a builtin call's or an array's leading operands
/// (`Promise.all([$dump.p4, $dump.p5])`) with literals among them. Effectful values may
/// go there, in this order.
pub(super) fn first_run(text: &str, first: usize, spans: &[(usize, usize)]) -> Vec<usize> {
    let b = text.as_bytes();
    let mut i = first;
    let ws = |i: &mut usize| {
        while b.get(*i).is_some_and(u8::is_ascii_whitespace) {
            *i += 1;
        }
    };
    // A builtin's call (not the dump's own): its callee read, its arguments in order.
    let n = b[i..]
        .iter()
        .take_while(|&&c| ident_part(c) || c == b'.')
        .count();
    if n > 0
        && b.get(i + n) == Some(&b'(')
        && ident_start(b[i])
        && !text[i..].starts_with("$dump")
        && !text[i..].starts_with("$djs")
        && text[i..i + n].split('.').all(|p| !p.is_empty())
    {
        i += n + 1;
    }
    let mut out = Vec::new();
    loop {
        // Literals and punctuation evaluate nothing.
        loop {
            ws(&mut i);
            match b.get(i) {
                Some(b'[' | b',') => i += 1,
                Some(b'"') => {
                    let mut j = i + 1;
                    while j < b.len() && b[j] != b'"' {
                        j += if b[j] == b'\\' { 2 } else { 1 };
                    }
                    i = j + 1;
                }
                Some(c)
                    if c.is_ascii_digit()
                        || *c == b'-' && b.get(i + 1).is_some_and(u8::is_ascii_digit) =>
                {
                    i += 1;
                    while b
                        .get(i)
                        .is_some_and(|&c| c.is_ascii_alphanumeric() || c == b'.' || c == b'_')
                    {
                        i += 1;
                    }
                }
                _ => {
                    let word = ["true", "false", "null", "undefined"].iter().find(|w| {
                        text[i..].starts_with(**w)
                            && !b.get(i + w.len()).is_some_and(|&c| ident_part(c))
                    });
                    match word {
                        Some(w) => i += w.len(),
                        None => break,
                    }
                }
            }
        }
        let Some(&(s, e)) = spans.iter().find(|x| x.0 == i) else {
            break;
        };
        out.push_out(s);
        i = e;
        // (Only a whole operand lets the next one follow.)
        ws(&mut i);
        if b.get(i) != Some(&b',') {
            break;
        }
    }
    out
}
