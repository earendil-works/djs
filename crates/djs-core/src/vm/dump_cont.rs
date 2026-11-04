//! The continuation of a suspended activation (D2830, docs/plain-js-dumps.md §2): a
//! function of the original's kind whose body is the rest of the original from its
//! site, written from the original's text by the syntax the compiler recorded around
//! the site (D2850, compiler/smap_syntax.rs).
//!
//! The continuation's prefix declares the bindings live at the site with their values
//! (the dump's code for the frame's shared scopes goes into the block that declares
//! them, `Ctx::block`), then the site's statement rewritten: values evaluated before
//! the site come from the frame's temporaries, the site itself is `(await p)` (or a
//! placeholder `(yield)`), the rest is the original's text. Loops around the site are
//! scaffolds whose first iteration is the rest of the current one; `try`, `switch` and
//! labels are kept; then the rest of the function, verbatim.

use alloc::string::String;
use alloc::vec::Vec;

use super::plain::{Act, BDecl, Ctx, KText};
use super::{is_ident, join, quote};
use crate::buf::PushOut;
use crate::compiler::{DeclKind, Node, NodeKind, Role, Special};
use crate::interp::run::suspend;

type R<T> = Result<T, String>;

fn no<T>(why: &str) -> R<T> {
    Err(cat!("plain dumps: ", why))
}

/// Generated text: strings, verbatim spans of the original, marks.
enum P {
    S(String),
    V(u32, u32),
    /// A scaffold record (`scaf[i]`): its continuation offset is set here.
    Mark(usize),
    /// Where K's captured helpers are declared.
    Helpers,
    L(Vec<P>),
}

fn s(t: &str) -> P {
    P::S(String::from(t))
}

/// Writes activation `act`'s continuation.
pub(super) fn generate(act: &Act, ctx: &mut dyn Ctx) -> R<KText> {
    if act.site.gap.is_some() {
        return no(
            "a site inside a construct the compiler does not record (a pattern, a class body)",
        );
    }
    if act.site.path.is_empty() {
        return no("no syntax recorded at the site");
    }
    if !matches!(
        act.site.kind,
        crate::compiler::SiteKind::Await
            | crate::compiler::SiteKind::Yield
            | crate::compiler::SiteKind::AsyncYield
    ) {
        return Err(cat!(
            "plain dumps: a site of kind ",
            act.site.name,
            " (0203)"
        ));
    }
    for &(k, _) in &act.syntax.special {
        if k != Special::Arguments {
            return no("new.target or super in a continuation's function (0203)");
        }
    }
    let mut g = G {
        act,
        ctx,
        used: Vec::new(),
        scaf: Vec::new(),
        cmap: Vec::new(),
        temps: Vec::new(),
        ntemps: 0,
        go_done: false,
        block_index: 0,
        uses_arguments: false,
        arrow: act.fkind == crate::bytecode::template::func_flags::KIND_ARROW,
    };
    let body = g.body()?;
    let head = match (g.arrow, act.kind) {
        (true, suspend::ASYNC) => "async () => {\n",
        (true, _) => "() => {\n",
        (false, suspend::ASYNC) => "async function () {\n",
        (false, suspend::GENERATOR) => "function* () {\n",
        (false, _) => "async function* () {\n",
    };
    let parts = P::L(alloc::vec![s(head), P::Helpers, body, s("\n}")]);
    Ok(g.flatten(parts))
}

/// Whether `text` holds the identifier `name` (not as part of a longer one).
fn has_ident(text: &str, name: &str) -> bool {
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'$';
    let b = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find(name) {
        let (s, e) = (from + at, from + at + name.len());
        if (s == 0 || !ident(b[s - 1])) && (e == b.len() || !ident(b[e])) {
            return true;
        }
        from = s + 1;
    }
    false
}

struct G<'a, 'c> {
    act: &'a Act,
    ctx: &'c mut dyn Ctx,
    used: Vec<String>,
    /// Scaffold constructs: kind, offset in the original, offset in K.
    /// Scaffold constructs: kind, offset in the original, offset in K, and what else
    /// origin tracking needs of it (a finally's completion: `, kind, "binding"`).
    scaf: Vec<(&'static str, u32, u32, String)>,
    /// The original's temporaries of the rewritten statement: slot, and the binding of K
    /// that holds the value or the literal written in its place.
    cmap: Vec<(String, String, bool)>,
    /// Temporaries of the site's statement not yet declared.
    temps: Vec<String>,
    ntemps: usize,
    go_done: bool,
    block_index: usize,
    uses_arguments: bool,
    arrow: bool,
}

impl G<'_, '_> {
    fn ftext(&self) -> &str {
        &self.act.ftext
    }

    /// `base`, or `base_k` if the function's text names it already (a continuation
    /// of a continuation has the first one's scaffold names).
    fn fresh(&self, base: String) -> String {
        if !has_ident(self.ftext(), &base) {
            return base;
        }
        let mut k = 1;
        loop {
            let n = cat!(base, "_", k);
            if !has_ident(self.ftext(), &n) {
                return n;
            }
            k += 1;
        }
    }

    fn node(&self, i: usize) -> &Node {
        let step = self.act.site.path[i];
        &self.act.syntax.nodes[step.node as usize]
    }

    /// The part of path node `i` that holds node `i + 1`.
    fn next_part(&self, i: usize) -> Option<usize> {
        self.act.site.path[i].part.map(usize::from)
    }

    fn is_site(&self, i: usize) -> bool {
        i + 1 == self.act.site.path.len()
    }

    fn text(&self, a: u32, b: u32) -> &str {
        self.ftext().get(a as usize..b as usize).unwrap_or("")
    }

    fn mark(&mut self, kind: &'static str, f: u32) -> P {
        self.scaf.push_out((kind, f, 0, String::new()));
        P::Mark(self.scaf.len() - 1)
    }

    fn has(&mut self, name: &str) -> bool {
        self.act.slots.iter().any(|s| s.name == name)
    }

    fn take(&mut self, name: &str) -> R<String> {
        match self.ctx.slot(name) {
            Some(v) => {
                if !self.used.iter().any(|u| u == name) {
                    self.used.push_out(String::from(name));
                }
                Ok(v)
            }
            None => Err(cat!("plain dumps: missing slot ", name)),
        }
    }

    /// A temporary of the site's statement: a literal in place, else a const declared
    /// right before the statement (the dump's values are reachable only while it loads).
    fn tmp(&mut self, construct: &str, off: u32) -> R<P> {
        let slot = cat!(construct, "@", off);
        let v = self.take(&slot)?;
        if literal(&v) {
            self.cmap.push_out((slot, v.clone(), true));
            return Ok(P::S(v));
        }
        let mut name = cat!("$djs_t", self.ntemps);
        self.ntemps += 1;
        while has_ident(self.ftext(), &name)
            || self.temps.iter().any(|t| t.starts_with(&cat!(name, " ")))
        {
            name = cat!("$djs_t", self.ntemps);
            self.ntemps += 1;
        }
        self.temps.push_out(cat!(name, " = ", v));
        self.cmap.push_out((slot, name.clone(), false));
        Ok(P::S(name))
    }

    fn flush_temps(&mut self) -> P {
        if self.temps.is_empty() {
            return s("");
        }
        let t = cat!("const ", join(&self.temps, ", "), ";\n");
        self.temps.clear();
        P::S(t)
    }

    /// The statement at the site, after the dump's callback and the temporaries.
    fn with_go(&mut self, parts: P) -> P {
        let t = self.flush_temps();
        if self.go_done {
            return P::L(alloc::vec![t, parts]);
        }
        self.go_done = true;
        let go = self.ctx.go();
        P::L(alloc::vec![
            P::S(if go.is_empty() { go } else { cat!(go, "\n") }),
            t,
            parts
        ])
    }

    /// The original's text `[a, b)`, with `arguments` renamed (the continuation's own
    /// would be another object).
    fn rest(&mut self, a: u32, b: u32) -> P {
        if self.arrow {
            return P::V(a, b);
        }
        let mut out = Vec::new();
        let mut at = a;
        for &(k, off) in &self.act.syntax.special {
            if k == Special::Arguments && off >= a && off < b {
                out.push_out(P::V(at, off));
                out.push_out(s("$djs_arguments"));
                at = off + "arguments".len() as u32;
                self.uses_arguments = true;
            }
        }
        if out.is_empty() {
            return P::V(a, b);
        }
        out.push_out(P::V(at, b));
        P::L(out)
    }

    fn rest_part(&mut self, n: &Node, k: usize) -> P {
        let p = n.parts[k];
        self.rest(p.start, p.end)
    }

    // ---------------------------------------------------------------- bindings

    /// The slot of binding `name` declared by scope `scope`.
    fn binding_slot(&mut self, name: &str, scope: u32) -> Option<String> {
        let shadowed = cat!(name, "@", scope);
        if self.has(&shadowed) {
            return Some(shadowed);
        }
        self.has(name).then(|| String::from(name))
    }

    /// The declarations of a block of K: the live bindings with their values, the dump's
    /// code for this block, bindings whose values exist only after the dump's callback.
    fn declare(&mut self, decls: &[(String, DeclKind, u32, bool)]) -> R<P> {
        let mut early = Vec::new();
        let mut late = Vec::new();
        let mut after = Vec::new();
        let mut info = Vec::new();
        for (name, kind, scope, live) in decls {
            let sname = self.binding_slot(name, *scope);
            info.push_out(BDecl {
                sname: sname.clone(),
            });
            let (true, Some(sname)) = (*live, sname) else {
                continue;
            };
            let value = match self.ctx.cell(&sname) {
                Some(None) => {
                    self.used.push_out(sname);
                    continue;
                }
                Some(Some(init)) => init,
                None => match self.ctx.slot(&sname) {
                    Some(v) if v == "$dump.TDZ" => {
                        self.used.push_out(sname);
                        continue;
                    }
                    Some(v) => v,
                    None => continue,
                },
            };
            self.used.push_out(sname.clone());
            if matches!(kind, DeclKind::Using | DeclKind::AwaitUsing) {
                return no("a using declaration in scope (0203)");
            }
            let kw = match kind {
                DeclKind::Const | DeclKind::FuncName => "const",
                DeclKind::Var | DeclKind::Param | DeclKind::Function => "var",
                _ => "let",
            };
            let target = if name == "arguments" {
                self.uses_arguments = true;
                "$djs_arguments"
            } else {
                name.as_str()
            };
            let line = cat!(kw, " ", target, " = ", value, ";");
            if self.ctx.after_go(&sname) {
                after.push_out(line);
            } else if has_factory_ref(&value) {
                late.push_out(line);
            } else {
                early.push_out(line);
            }
        }
        let content = self.ctx.block(self.block_index, &info)?;
        self.block_index += 1;
        let mut lines = early;
        if !content.is_empty() {
            lines.push_out(content);
        }
        lines.extend(late);
        if !after.is_empty() && !self.go_done {
            self.go_done = true;
            let go = self.ctx.go();
            if !go.is_empty() {
                lines.push_out(go);
            }
        }
        lines.extend(after);
        Ok(P::S(join(&lines, "\n")))
    }

    /// The bindings of the function body, statement `j` holding the site.
    fn body_decls(&mut self, n: &Node, j: Option<usize>) -> Vec<(String, DeclKind, u32, bool)> {
        let mut out: Vec<(String, DeclKind, u32, bool)> = Vec::new();
        for d in &n.decls {
            let live = match d.kind {
                DeclKind::Param | DeclKind::Var | DeclKind::Function | DeclKind::FuncName => true,
                _ => d
                    .part
                    .is_some_and(|p| j.is_some_and(|j| usize::from(p) < j)),
            };
            if !out.iter().any(|o| o.0 == d.name) {
                out.push_out((d.name.clone(), d.kind, d.scope, live));
            }
        }
        if !self.arrow && self.has("arguments") && !out.iter().any(|o| o.0 == "arguments") {
            out.push_out((String::from("arguments"), DeclKind::Var, 0, true));
        }
        out
    }

    fn block_decls(n: &Node, j: usize) -> Vec<(String, DeclKind, u32, bool)> {
        n.decls
            .iter()
            .map(|d| {
                let live =
                    d.kind == DeclKind::Function || d.part.is_some_and(|p| usize::from(p) < j);
                (d.name.clone(), d.kind, d.scope, live)
            })
            .collect()
    }

    // ---------------------------------------------------------------- statements

    fn body(&mut self) -> R<P> {
        let n = self.node(0).clone();
        let Some(k) = self.next_part(0) else {
            return no("a site that is its function");
        };
        if n.parts[k].role == Role::Operand {
            // An arrow's concise body.
            let decls = self.body_decls(&n, None);
            let d = self.declare(&decls)?;
            let e = self.expr(1)?;
            let st = self.with_go(P::L(alloc::vec![s("return "), e, s(";")]));
            return Ok(P::L(alloc::vec![d, s("\n"), st]));
        }
        let decls = self.body_decls(&n, Some(k));
        self.list(&n, k, 1, &decls)
    }

    /// A statement list from part `k` (holding path node `i`) on.
    fn list(
        &mut self,
        n: &Node,
        k: usize,
        i: usize,
        decls: &[(String, DeclKind, u32, bool)],
    ) -> R<P> {
        let d = self.declare(decls)?;
        let st = self.stmt(i, None)?;
        let mut out = alloc::vec![d, s("\n"), st];
        for m in k + 1..n.parts.len() {
            match n.parts[m].role {
                Role::Stmt => {
                    out.push_out(s("\n"));
                    out.push_out(self.rest_part(n, m));
                }
                // Hoisted: a binding of the prefix.
                Role::FuncDecl => {}
                // The next case clause of a switch: its statements follow (fall-through).
                Role::Case | Role::Default => {}
                _ => {}
            }
        }
        Ok(P::L(out))
    }

    fn stmt(&mut self, i: usize, label: Option<P>) -> R<P> {
        let n = self.node(i).clone();
        let k = self.next_part(i);
        let lbl = |l: Option<P>| l.unwrap_or_else(|| s(""));
        match n.kind {
            NodeKind::Block => {
                let k = k.ok_or("plain dumps: a block at the site")?;
                let decls = Self::block_decls(&n, k);
                let l = self.list(&n, k, i + 1, &decls)?;
                Ok(P::L(alloc::vec![lbl(label), s("{\n"), l, s("\n}")]))
            }
            NodeKind::Expr => {
                let e = self.expr(i + 1)?;
                Ok(self.with_go(P::L(alloc::vec![e, s(";")])))
            }
            NodeKind::Return | NodeKind::Throw => {
                let e = self.expr(i + 1)?;
                let kw = if n.kind == NodeKind::Return {
                    "return "
                } else {
                    "throw "
                };
                Ok(self.with_go(P::L(alloc::vec![s(kw), e, s(";")])))
            }
            NodeKind::Decl(kind) => self.decl_stmt(i, &n, kind),
            NodeKind::If => {
                let k = k.unwrap_or(0);
                if n.parts[k].role != Role::Test {
                    return self.stmt(i + 1, label);
                }
                let e = self.expr(i + 1)?;
                let mut out = alloc::vec![s("if ("), e, s(") ")];
                for m in 0..n.parts.len() {
                    match n.parts[m].role {
                        Role::Then => out.push_out(self.rest_part(&n, m)),
                        Role::Else => {
                            out.push_out(s(" else "));
                            out.push_out(self.rest_part(&n, m));
                        }
                        _ => {}
                    }
                }
                Ok(self.with_go(P::L(out)))
            }
            NodeKind::Label => {
                let lp = n
                    .parts
                    .iter()
                    .find(|p| p.role == Role::Label)
                    .copied()
                    .ok_or("plain dumps: a label without its name")?;
                let l = P::L(alloc::vec![P::V(lp.start, lp.end), s(": ")]);
                let inner = self.node(i + 1).kind;
                if matches!(
                    inner,
                    NodeKind::While
                        | NodeKind::DoWhile
                        | NodeKind::For
                        | NodeKind::ForOf
                        | NodeKind::ForIn
                ) {
                    // On the scaffold's loop, so `continue label` still works.
                    self.stmt(i + 1, Some(l))
                } else {
                    let st = self.stmt(i + 1, None)?;
                    Ok(P::L(alloc::vec![l, st]))
                }
            }
            NodeKind::While | NodeKind::DoWhile => {
                let k = k.unwrap_or(0);
                if n.parts[k].role != Role::Body {
                    return no("a site in a loop's test (0203)");
                }
                let f = self.fresh(cat!("$djs_f", i));
                let test = n.parts.iter().position(|p| p.role == Role::Test);
                let test = match test {
                    Some(t) => self.rest_part(&n, t),
                    None => s("true"),
                };
                let body = self.rest_part(&n, k);
                let m = self.mark(
                    if n.kind == NodeKind::While {
                        "while"
                    } else {
                        "do"
                    },
                    n.start,
                );
                let first = self.stmt(i + 1, None)?;
                Ok(if n.kind == NodeKind::While {
                    P::L(alloc::vec![
                        P::S(cat!("{ let ", f, " = true; ")),
                        m,
                        lbl(label),
                        P::S(cat!("while (", f, " || (")),
                        test,
                        P::S(cat!(")) { if (", f, ") { ", f, " = false; ")),
                        first,
                        s(" } else "),
                        body,
                        s(" } }"),
                    ])
                } else {
                    P::L(alloc::vec![
                        P::S(cat!("{ let ", f, " = true; ")),
                        m,
                        lbl(label),
                        P::S(cat!("do { if (", f, ") { ", f, " = false; ")),
                        first,
                        s(" } else "),
                        body,
                        s(" } while ("),
                        test,
                        s("); }"),
                    ])
                })
            }
            NodeKind::For => self.for_stmt(i, &n, label),
            NodeKind::ForOf | NodeKind::ForIn => self.for_of(i, &n, label),
            NodeKind::ForAwait => no("a for await around the site (0203)"),
            NodeKind::Try => self.try_stmt(i, &n),
            NodeKind::Switch => self.switch_stmt(i, &n),
            _ => Err(cat!(
                "plain dumps: a statement around the site: ",
                kind_name(n.kind)
            )),
        }
    }

    fn decl_stmt(&mut self, i: usize, n: &Node, kind: DeclKind) -> R<P> {
        let kw = match kind {
            DeclKind::Let => "let ",
            DeclKind::Const => "const ",
            DeclKind::Var => return no("a site in a var declaration (0203)"),
            _ => return no("a site in a using declaration (0203)"),
        };
        let Some(k) = self.next_part(i) else {
            return no("a declaration at the site");
        };
        if n.parts[k].role != Role::Init {
            return no("a site in a pattern (0203)");
        }
        let mut items: Vec<P> = Vec::new();
        let mut m = 0;
        while m < n.parts.len() {
            let t = n.parts[m];
            if t.role != Role::Target {
                m += 1;
                continue;
            }
            let init =
                (m + 1 < n.parts.len() && n.parts[m + 1].role == Role::Init).then_some(m + 1);
            let end = init.map_or(t.end, |x| n.parts[x].end);
            if init == Some(k) {
                let e = self.expr(i + 1)?;
                items.push_out(P::L(alloc::vec![P::V(t.start, t.end), s(" = "), e]));
            } else if t.start > n.parts[k].start {
                items.push_out(self.rest(t.start, end));
            } else {
                // Declared before the site: its value is in the frame.
                let name = String::from(self.text(t.start, t.end).trim());
                if !is_ident(&name) {
                    return no("a pattern declared before the site in its statement (0203)");
                }
                let scope = self.enclosing_scope_of(&name);
                let v = match self.binding_slot(&name, scope) {
                    Some(sn) => {
                        let v = match self.ctx.cell(&sn) {
                            Some(Some(init)) => init,
                            Some(None) => return no("a shared binding in its TDZ"),
                            None => self.take(&sn)?,
                        };
                        if !self.used.contains(&sn) {
                            self.used.push_out(sn);
                        }
                        v
                    }
                    None => String::from("undefined"),
                };
                items.push_out(P::S(cat!(name, " = ", v)));
            }
            m = init.map_or(m + 1, |x| x + 1);
        }
        let mut out = alloc::vec![s(kw)];
        for (x, it) in items.into_iter().enumerate() {
            if x > 0 {
                out.push_out(s(", "));
            }
            out.push_out(it);
        }
        out.push_out(s(";"));
        Ok(self.with_go(P::L(out)))
    }

    /// The scope offset of a binding declared by the statement list around path node
    /// `i`: the innermost enclosing node that declares it.
    fn enclosing_scope_of(&self, name: &str) -> u32 {
        for i in (0..self.act.site.path.len()).rev() {
            if let Some(d) = self.node(i).decls.iter().find(|d| d.name == name) {
                return d.scope;
            }
        }
        0
    }

    fn for_stmt(&mut self, i: usize, n: &Node, label: Option<P>) -> R<P> {
        let k = self.next_part(i).unwrap_or(0);
        if n.parts[k].role != Role::Body {
            return no("a site in a for loop's head (0203)");
        }
        let f = self.fresh(cat!("$djs_f", i));
        let mut init: Vec<P> = Vec::new();
        if !n.decls.is_empty() {
            let kw = match n.decls[0].kind {
                DeclKind::Const => "const ",
                DeclKind::Let => "let ",
                _ => return no("a for loop's var or using declaration (0203)"),
            };
            init.push_out(s(kw));
            for (x, d) in n.decls.clone().iter().enumerate() {
                let Some(sn) = self.binding_slot(&d.name, d.scope) else {
                    return Err(cat!("plain dumps: no slot for ", d.name));
                };
                let v = match self.ctx.cell(&sn) {
                    Some(Some(v)) => v,
                    Some(None) => return no("a loop binding in its TDZ"),
                    None => self.take(&sn)?,
                };
                if !self.used.contains(&sn) {
                    self.used.push_out(sn);
                }
                if x > 0 {
                    init.push_out(s(", "));
                }
                init.push_out(P::S(cat!(d.name, " = ", v)));
            }
        } else if n.parts.iter().any(|p| p.role == Role::Init)
            && self.text(n.start, n.parts[0].start).contains("var")
        {
            return no("a for loop's var declaration (0203)");
        }
        let test = match n.parts.iter().position(|p| p.role == Role::Test) {
            Some(t) => self.rest_part(n, t),
            None => s("true"),
        };
        let update = match n.parts.iter().position(|p| p.role == Role::Update) {
            Some(t) => self.rest_part(n, t),
            None => s(""),
        };
        let body = self.rest_part(n, k);
        let m = self.mark("for", n.start);
        // The per-iteration scope: its bindings are declared by the head; the code the
        // dump puts there runs in the first iteration.
        let own: Vec<_> = n
            .decls
            .iter()
            .map(|d| (d.name.clone(), d.kind, d.scope, false))
            .collect();
        let own = self.declare(&own)?;
        let first = self.stmt(i + 1, None)?;
        Ok(P::L(alloc::vec![
            P::S(cat!("{ let ", f, " = true; ")),
            m,
            label.unwrap_or_else(|| s("")),
            s("for ("),
            P::L(init),
            P::S(cat!("; ", f, " || (")),
            test,
            s("); "),
            update,
            P::S(cat!(") { if (", f, ") { ", f, " = false; ")),
            own,
            s("\n"),
            first,
            s(" } else "),
            body,
            s(" } }"),
        ]))
    }

    /// A for-of or for-in around the site: the rest of this iteration runs first, then
    /// the loop goes on over the saved iterator record as a real loop (`break`,
    /// `return` and exceptions close the iterator as the original's do).
    fn for_of(&mut self, i: usize, n: &Node, label: Option<P>) -> R<P> {
        let k = self.next_part(i).unwrap_or(0);
        if n.parts[k].role != Role::Body {
            return no("a site in a loop's head (0203)");
        }
        let c = if n.kind == NodeKind::ForIn {
            "for-in"
        } else {
            "for-of"
        };
        let it = self.take(&cat!(c, ".iterator@", n.start))?;
        let next = if n.kind == NodeKind::ForIn {
            String::from("null")
        } else {
            self.take(&cat!(c, ".next@", n.start))?
        };
        let head = n
            .parts
            .iter()
            .find(|p| p.role == Role::Head)
            .copied()
            .ok_or("plain dumps: a loop without its head")?;
        let f = self.fresh(cat!("$djs_f", i));
        let v = self.fresh(cat!("$djs_v", i));
        let head_text = String::from(self.text(head.start, head.end));
        let bind = if n.decls.is_empty() {
            if head_text.trim_start().starts_with("var") {
                return no("a for (var ... of) around the site (0203)");
            }
            P::L(alloc::vec![
                s("("),
                P::V(head.start, head.end),
                P::S(cat!(" = ", v, ");"))
            ])
        } else {
            P::L(alloc::vec![
                P::V(head.start, head.end),
                P::S(cat!(" = ", v, ";"))
            ])
        };
        let decls: Vec<_> = n
            .decls
            .iter()
            .map(|d| (d.name.clone(), d.kind, d.scope, true))
            .collect();
        let body = self.rest_part(n, k);
        let m = self.mark(c, n.start);
        let d = self.declare(&decls)?;
        let first = self.stmt(i + 1, None)?;
        Ok(P::L(alloc::vec![
            P::S(cat!("{ let ", f, " = true; ")),
            label.unwrap_or_else(|| s("")),
            // (At the loop's own start: its temporaries are named by it.)
            m,
            P::S(cat!(
                "for (const ",
                v,
                " of $djs_L(",
                it,
                ", ",
                next,
                ")) { if (",
                f,
                ") { ",
                f,
                " = false; {\n"
            )),
            d,
            s("\n"),
            first,
            s("\n} } else { "),
            bind,
            s(" "),
            body,
            s(" } } }"),
        ]))
    }

    fn try_stmt(&mut self, i: usize, n: &Node) -> R<P> {
        let k = self.next_part(i).unwrap_or(0);
        let part = |r: Role| n.parts.iter().position(|p| p.role == r);
        match n.parts[k].role {
            Role::Block => {
                let m = self.mark("try", n.start);
                let first = self.stmt(i + 1, None)?;
                let mut out = alloc::vec![m, s("try "), first];
                if let Some(cb) = part(Role::CatchBody) {
                    out.push_out(s(" catch"));
                    if let Some(cp) = part(Role::CatchParam) {
                        out.push_out(s(" ("));
                        out.push_out(self.rest_part(n, cp));
                        out.push_out(s(")"));
                    }
                    out.push_out(s(" "));
                    out.push_out(self.rest_part(n, cb));
                }
                if let Some(fi) = part(Role::Finally) {
                    out.push_out(s(" finally "));
                    out.push_out(self.rest_part(n, fi));
                }
                Ok(P::L(out))
            }
            Role::CatchBody => {
                let decls: Vec<_> = n
                    .decls
                    .iter()
                    .map(|d| (d.name.clone(), d.kind, d.scope, true))
                    .collect();
                let d = self.declare(&decls)?;
                let first = self.stmt(i + 1, None)?;
                let h = P::L(alloc::vec![s("{\n"), d, s("\n"), first, s("\n}")]);
                Ok(match part(Role::Finally) {
                    Some(fi) => {
                        let m = self.mark("try", n.start);
                        let fin = self.rest_part(n, fi);
                        P::L(alloc::vec![m, s("try "), h, s(" finally "), fin])
                    }
                    None => {
                        let m = self.mark("catch", n.start);
                        P::L(alloc::vec![m, h])
                    }
                })
            }
            Role::Finally => {
                // The rest of the finally block, then the completion that entered it.
                let kind = self.take(&cat!("completion.kind@", n.start))?;
                let v = self.take(&cat!("completion.value@", n.start))?;
                let cv = self.fresh(cat!("$djs_c", i));
                let done = match kind.as_str() {
                    "0" => String::new(),
                    "1" => cat!("throw ", cv, ";"),
                    "2" => cat!("return ", cv, ";"),
                    _ => return no("a finally entered by break or continue (0203)"),
                };
                let m = self.mark("finally", n.start);
                if let P::Mark(x) = m {
                    self.scaf[x].3 = cat!(",", kind, ",", quote(&cv));
                }
                let first = self.stmt(i + 1, None)?;
                Ok(P::L(alloc::vec![
                    m,
                    P::S(cat!("{ const ", cv, " = ", v, "; ")),
                    first,
                    P::S(cat!(" ", done, " }")),
                ]))
            }
            _ => no("a site in a catch parameter (0203)"),
        }
    }

    fn switch_stmt(&mut self, i: usize, n: &Node) -> R<P> {
        let k = self.next_part(i).unwrap_or(0);
        if n.parts[k].role != Role::Stmt {
            return no("a site in a switch's discriminant or case test (0203)");
        }
        // The clause holding the site: its statements from the site's on, then the
        // statements of the later clauses (fall-through); `break` leaves `switch (0)`.
        let decls = Self::block_decls(n, k);
        let m = self.mark("switch", n.start);
        let l = self.list(n, k, i + 1, &decls)?;
        Ok(P::L(alloc::vec![m, s("switch (0) { case 0: "), l, s(" }")]))
    }

    // ---------------------------------------------------------------- expressions

    fn expr(&mut self, i: usize) -> R<P> {
        let n = self.node(i).clone();
        if self.is_site(i) {
            let m = self.mark("site", n.start);
            return Ok(match n.kind {
                NodeKind::Await => P::L(alloc::vec![
                    s("("),
                    m,
                    s("await "),
                    P::S(self.ctx.awaiting()),
                    s(")")
                ]),
                // A yield's operand was yielded before the dump.
                NodeKind::Yield => P::L(alloc::vec![s("("), m, s("yield)")]),
                _ => return Err(cat!("plain dumps: a site at ", kind_name(n.kind))),
            });
        }
        let k = self
            .next_part(i)
            .ok_or("plain dumps: a path that ends early")?;
        let role = n.parts[k].role;
        let pt = |r: Role| n.parts.iter().position(|p| p.role == r);
        match n.kind {
            NodeKind::Await => {
                let e = self.expr(i + 1)?;
                Ok(P::L(alloc::vec![s("(await "), e, s(")")]))
            }
            NodeKind::Yield => {
                let e = self.expr(i + 1)?;
                Ok(P::L(alloc::vec![s("(yield "), e, s(")")]))
            }
            NodeKind::Call { .. } | NodeKind::New => self.call(i, &n, k),
            NodeKind::Binary | NodeKind::Logical => {
                let op = pt(Role::Op).ok_or("plain dumps: an operator without its token")?;
                let opt = String::from(self.text(n.parts[op].start, n.parts[op].end));
                if role == Role::Left {
                    let e = self.expr(i + 1)?;
                    let r = pt(Role::Right)
                        .map(|r| self.rest_part(&n, r))
                        .unwrap_or(s(""));
                    return Ok(P::L(alloc::vec![
                        s("("),
                        e,
                        P::S(cat!(" ", opt, " ")),
                        r,
                        s(")")
                    ]));
                }
                if n.kind == NodeKind::Logical {
                    // The left side decided to evaluate the right.
                    return self.expr(i + 1);
                }
                let l = self.tmp("binary.left", n.start)?;
                let e = self.expr(i + 1)?;
                Ok(P::L(alloc::vec![
                    s("("),
                    l,
                    P::S(cat!(" ", opt, " ")),
                    e,
                    s(")")
                ]))
            }
            NodeKind::Conditional => {
                if role == Role::Test {
                    let e = self.expr(i + 1)?;
                    let t = pt(Role::Then)
                        .map(|x| self.rest_part(&n, x))
                        .unwrap_or(s(""));
                    let f = pt(Role::Else)
                        .map(|x| self.rest_part(&n, x))
                        .unwrap_or(s(""));
                    return Ok(P::L(alloc::vec![
                        s("("),
                        e,
                        s(" ? "),
                        t,
                        s(" : "),
                        f,
                        s(")")
                    ]));
                }
                self.expr(i + 1)
            }
            NodeKind::Unary => {
                let op = pt(Role::Op).ok_or("plain dumps: an operator without its token")?;
                let opt = String::from(self.text(n.parts[op].start, n.parts[op].end));
                let e = self.expr(i + 1)?;
                Ok(P::L(alloc::vec![P::S(cat!("(", opt, " ")), e, s(")")]))
            }
            NodeKind::Sequence => {
                let mut out = alloc::vec![s("("), self.expr(i + 1)?];
                for m in k + 1..n.parts.len() {
                    out.push_out(s(", "));
                    out.push_out(self.rest_part(&n, m));
                }
                out.push_out(s(")"));
                Ok(P::L(out))
            }
            NodeKind::Template => {
                // Substitutions before the site are strings already; the literal parts
                // before it are kept by djs too (`template.part@` of the delimiter).
                let mut out = Vec::new();
                let mut seen = false;
                for m in 0..n.parts.len() {
                    let p = n.parts[m];
                    match p.role {
                        Role::Quasi => {
                            let name = cat!("template.part@", p.start);
                            if !seen && self.has(&name) && !self.used.contains(&name) {
                                self.used.push_out(name);
                            }
                            out.push_out(P::V(p.start, p.end));
                        }
                        Role::Subst if m == k => {
                            out.push_out(self.expr(i + 1)?);
                            seen = true;
                        }
                        Role::Subst if seen => out.push_out(self.rest_part(&n, m)),
                        Role::Subst => out.push_out(self.tmp("template.part", p.start)?),
                        _ => {}
                    }
                }
                Ok(P::L(out))
            }
            NodeKind::Array => self.array(i, &n, k),
            NodeKind::Member { computed, .. } => {
                if let Some(o) = pt(Role::Object)
                    && self.text(n.parts[o].start, n.parts[o].end) == "super"
                {
                    return no("super around the site (0203)");
                }
                if role == Role::Object {
                    let e = self.expr(i + 1)?;
                    let tail = self.rest(n.parts[k].end, n.end);
                    return Ok(P::L(alloc::vec![e, tail]));
                }
                if !computed {
                    return no("a site in a property name");
                }
                let o = self.tmp("member.object", n.start)?;
                let e = self.expr(i + 1)?;
                Ok(P::L(alloc::vec![o, s("["), e, s("]")]))
            }
            NodeKind::Assign => self.assign(i, &n, k),
            NodeKind::Object => self.object(i, &n, k),
            _ => Err(cat!(
                "plain dumps: an expression around the site: ",
                kind_name(n.kind)
            )),
        }
    }

    fn call(&mut self, i: usize, n: &Node, k: usize) -> R<P> {
        let new = n.kind == NodeKind::New;
        let callee = n.parts.iter().position(|p| p.role == Role::Callee);
        let open = n.parts.iter().position(|p| p.role == Role::Open);
        if n.parts[k].role == Role::Callee {
            if new {
                return no("a site in a new's callee");
            }
            let e = self.expr(i + 1)?;
            let tail = self.rest(n.parts[k].end, n.end);
            return Ok(P::L(alloc::vec![e, tail]));
        }
        if let Some(c) = callee {
            let t = self.text(n.parts[c].start, n.parts[c].end);
            if t == "super" || t.starts_with("super.") || t.starts_with("super[") {
                return no("a super call around the site (0203)");
            }
        }
        let (ce, this) = if new {
            (self.tmp("new.callee", n.start)?, None)
        } else {
            (
                self.tmp("call.callee", n.start)?,
                Some(self.tmp("call.this", n.start)?),
            )
        };
        let args: Vec<usize> = (0..n.parts.len())
            .filter(|&m| matches!(n.parts[m].role, Role::Arg | Role::Spread))
            .collect();
        let spread_before = args
            .iter()
            .any(|&m| m < k && n.parts[m].role == Role::Spread);
        let mut list: Vec<P> = Vec::new();
        let head = if new { "$djs_C(" } else { "$djs_R(" };
        let mut out = alloc::vec![s(head), ce, s(", ")];
        if let Some(t) = this {
            out.push_out(t);
            out.push_out(s(", "));
        }
        if spread_before {
            // The argument list so far is one array: the rest is appended to it.
            let open = open.ok_or("plain dumps: a call without its parenthesis")?;
            let acc = self.tmp(
                if new { "new.args" } else { "call.args" },
                n.parts[open].start,
            )?;
            list.push_out(acc);
            for &m in &args {
                if m == k {
                    list.push_out(self.arg(i, n, m)?);
                } else if m > k {
                    list.push_out(self.rest_part(n, m));
                }
            }
            out.push_out(s("$djs_Q("));
            push_joined(&mut out, list);
            out.push_out(s("))"));
            return Ok(P::L(out));
        }
        for &m in &args {
            let p = n.parts[m];
            if m < k {
                list.push_out(self.tmp(if new { "new.arg" } else { "call.arg" }, p.start)?);
            } else if m == k {
                list.push_out(self.arg(i, n, m)?);
            } else {
                list.push_out(self.rest_part(n, m));
            }
        }
        out.push_out(s("["));
        push_joined(&mut out, list);
        out.push_out(s("])"));
        Ok(P::L(out))
    }

    /// An argument or element holding the site (`...` kept for a spread).
    fn arg(&mut self, i: usize, n: &Node, m: usize) -> R<P> {
        let e = self.expr(i + 1)?;
        Ok(if n.parts[m].role == Role::Spread {
            P::L(alloc::vec![s("..."), e])
        } else {
            e
        })
    }

    fn array(&mut self, i: usize, n: &Node, k: usize) -> R<P> {
        let lit = cat!("array.literal@", n.start);
        if self.has(&lit) {
            if n.parts[k + 1..].iter().any(|p| p.role == Role::Hole) {
                return no("a hole after the site in an array with a spread");
            }
            let v = self.tmp("array.literal", n.start)?;
            let mut list = alloc::vec![v, self.arg(i, n, k)?];
            for m in k + 1..n.parts.len() {
                list.push_out(self.rest_part(n, m));
            }
            let mut out = alloc::vec![s("$djs_Q(")];
            push_joined(&mut out, list);
            out.push_out(s(")"));
            return Ok(P::L(out));
        }
        let mut list = Vec::new();
        for m in 0..n.parts.len() {
            let p = n.parts[m];
            if p.role == Role::Hole {
                list.push_out(s(""));
            } else if m == k {
                list.push_out(self.arg(i, n, m)?);
            } else if m > k {
                list.push_out(self.rest_part(n, m));
            } else if p.role == Role::Spread {
                return no("a spread before the site in an array");
            } else {
                list.push_out(self.tmp("array.element", p.start)?);
            }
        }
        let trailing_hole = n.parts.last().is_some_and(|p| p.role == Role::Hole);
        let mut out = alloc::vec![s("[")];
        push_joined(&mut out, list);
        out.push_out(s(if trailing_hole { ",]" } else { "]" }));
        Ok(P::L(out))
    }

    fn assign(&mut self, i: usize, n: &Node, k: usize) -> R<P> {
        if n.parts[k].role != Role::Right {
            return no("a site in an assignment's target (0203)");
        }
        let op = n
            .parts
            .iter()
            .position(|p| p.role == Role::Op)
            .ok_or("plain dumps: an assignment without its operator")?;
        let opt = String::from(self.text(n.parts[op].start, n.parts[op].end));
        let logical = matches!(opt.as_str(), "&&=" | "||=" | "??=");
        let compound = opt != "=" && !logical;
        let bin = String::from(opt.strip_suffix('=').unwrap_or(&opt));
        let target: P = if let Some(l) = n.parts.iter().position(|p| p.role == Role::Left) {
            let t = self.text(n.parts[l].start, n.parts[l].end).trim();
            if !is_ident(t) {
                return no("an assignment to a pattern around the site (0203)");
            }
            P::V(n.parts[l].start, n.parts[l].end)
        } else {
            let o = n.parts.iter().position(|p| p.role == Role::Object);
            let pr = n.parts.iter().position(|p| p.role == Role::Property);
            let (Some(o), Some(pr)) = (o, pr) else {
                return no("an assignment target the compiler did not describe");
            };
            if self.text(n.parts[o].start, n.parts[o].end) == "super" {
                return no("an assignment to super around the site (0203)");
            }
            if logical {
                return no("a logical assignment to a member around the site (0203)");
            }
            let obj = self.tmp("assign.object", n.start)?;
            let p = n.parts[pr];
            if self.text(p.start, p.end).starts_with('[') {
                let key = if self.has(&cat!("assign.key@", n.start)) {
                    self.tmp("assign.key", n.start)?
                } else {
                    self.rest(p.start + 1, p.end - 1)
                };
                P::L(alloc::vec![obj, s("["), key, s("]")])
            } else {
                P::L(alloc::vec![obj, s("."), P::V(p.start, p.end)])
            }
        };
        let e = self.expr(i + 1)?;
        Ok(if compound {
            let old = self.tmp("assign.old", n.start)?;
            P::L(alloc::vec![
                s("("),
                target,
                s(" = "),
                old,
                P::S(cat!(" ", bin, " ")),
                e,
                s(")")
            ])
        } else {
            P::L(alloc::vec![s("("), target, s(" = "), e, s(")")])
        })
    }

    fn object(&mut self, i: usize, n: &Node, k: usize) -> R<P> {
        // The object made so far; the rest of the literal is defined on it in order.
        let o = self.tmp("object.literal", n.start)?;
        let ot = match &o {
            P::S(t) => t.clone(),
            _ => String::new(),
        };
        // The first part of the property holding the site.
        let mut first = k;
        if n.parts[k].role == Role::Value && k > 0 {
            first = k - 1;
        }
        if n.parts[k].role == Role::ComputedKey {
            return no("a site in a computed key (0203)");
        }
        let mut items: Vec<P> = Vec::new();
        let mut m = first;
        while m < n.parts.len() {
            let p = n.parts[m];
            match p.role {
                Role::Spread => {
                    let v = if m == k {
                        self.expr(i + 1)?
                    } else {
                        self.rest(p.start + 3, p.end)
                    };
                    items.push_out(P::L(alloc::vec![
                        P::S(cat!("$djs_A(", ot, ", ")),
                        v,
                        s(")")
                    ]));
                    m += 1;
                }
                Role::Key | Role::ComputedKey => {
                    let vi = m + 1;
                    let key = if p.role == Role::Key {
                        let t = self.text(p.start, p.end);
                        if t == "__proto__" || t == "\"__proto__\"" || t == "'__proto__'" {
                            return no("__proto__ in an object literal around the site");
                        }
                        if is_ident(t) {
                            P::S(quote(t))
                        } else if t.bytes().all(|b| b.is_ascii_digit()) {
                            P::S(cat!("\"", t, "\""))
                        } else if t.starts_with('"') || t.starts_with('\'') {
                            P::V(p.start, p.end)
                        } else {
                            return no("a numeric key in an object literal around the site");
                        }
                    } else {
                        self.rest(p.start + 1, p.end - 1)
                    };
                    let v = if vi == k {
                        self.expr(i + 1)?
                    } else {
                        self.rest_part(n, vi)
                    };
                    items.push_out(P::L(alloc::vec![
                        P::S(cat!("$djs_D(", ot, ", ")),
                        key,
                        s(", "),
                        v,
                        s(")")
                    ]));
                    m = vi + 1;
                }
                Role::Shorthand => {
                    let t = String::from(self.text(p.start, p.end));
                    items.push_out(P::L(alloc::vec![
                        P::S(cat!("$djs_D(", ot, ", ", quote(&t), ", ")),
                        self.rest(p.start, p.end),
                        s(")"),
                    ]));
                    m += 1;
                }
                _ => return no("a method or accessor after the site in an object literal (0203)"),
            }
        }
        let mut out = alloc::vec![s("(")];
        push_joined(&mut out, items);
        out.push_out(P::S(cat!(", ", ot, ")")));
        Ok(P::L(out))
    }

    // ---------------------------------------------------------------- text

    /// The continuation's text, its segments (where it copied the original) and its
    /// scaffold records.
    fn flatten(mut self, parts: P) -> KText {
        let mut out = Flat::default();
        let act = self.act;
        out.walk(&parts, &act.ftext, &mut self.scaf);
        let mut text = out.text;
        // K's helpers, captured where the marker is.
        let names: Vec<&str> = ["R", "G", "C", "D", "A", "L", "N", "Q"]
            .into_iter()
            .filter(|h| text.contains(&cat!("$djs_", *h, "(")))
            // A continuation of a continuation declares them already (its own
            // first line, restored from its slot).
            .filter(|h| !act.slots.iter().any(|s| s.name == cat!("$djs_", *h)))
            .collect();
        let mut decl = String::new();
        if !names.is_empty() {
            let list: Vec<String> = names
                .iter()
                .map(|h| cat!("$djs_", *h, " = $dump.", *h))
                .collect();
            decl = cat!("const ", join(&list, ", "), ";\n");
        }
        if self.uses_arguments && !self.declares_arguments(&text) {
            decl.push_str("const $djs_arguments = undefined;\n");
        }
        let at = out.helpers.unwrap_or(0);
        text.insert_str(at, &decl);
        let shift = decl.len() as u32;
        let mut segs = out.segs;
        for x in segs.chunks_mut(3) {
            if x[0] as usize >= at {
                x[0] += shift;
            }
        }
        let mut scaf = Vec::new();
        for (k, f, ko, extra) in &self.scaf {
            let ko = if *ko as usize >= at { ko + shift } else { *ko };
            scaf.push_out(cat!("[", quote(k), ",", *f, ",", ko, extra, "]"));
        }
        let cmap: Vec<String> = self
            .cmap
            .iter()
            .map(|(slot, v, lit)| {
                if *lit {
                    cat!("[", quote(slot), ",1,", v, "]")
                } else {
                    cat!("[", quote(slot), ",0,", quote(v), "]")
                }
            })
            .collect();
        KText {
            text,
            segs,
            scaf: cat!("[", join(&scaf, ","), "]"),
            cmap: cat!("[", join(&cmap, ","), "]"),
            used: self.used,
        }
    }

    fn declares_arguments(&self, text: &str) -> bool {
        text.contains("var $djs_arguments = ")
    }
}

#[derive(Default)]
struct Flat {
    text: String,
    segs: Vec<u32>,
    helpers: Option<usize>,
}

impl Flat {
    fn walk(&mut self, p: &P, ftext: &str, scaf: &mut [(&'static str, u32, u32, String)]) {
        match p {
            P::S(t) => self.text.push_str(t),
            P::V(a, b) => {
                if b > a {
                    self.segs.push_out(self.text.len() as u32);
                    self.segs.push_out(*a);
                    self.segs.push_out(b - a);
                    self.text
                        .push_str(ftext.get(*a as usize..*b as usize).unwrap_or(""));
                }
            }
            P::Mark(i) => {
                if let Some(r) = scaf.get_mut(*i) {
                    r.2 = self.text.len() as u32;
                }
            }
            P::Helpers => self.helpers = Some(self.text.len()),
            P::L(list) => {
                for q in list {
                    self.walk(q, ftext, scaf);
                }
            }
        }
    }
}

fn push_joined(out: &mut Vec<P>, list: Vec<P>) {
    for (x, p) in list.into_iter().enumerate() {
        if x > 0 {
            out.push_out(s(", "));
        }
        out.push_out(p);
    }
}

/// A value that can stand in place of a temporary.
fn literal(v: &str) -> bool {
    matches!(v, "undefined" | "true" | "false" | "null")
        || v.parse::<f64>().is_ok()
            && !v.contains(['e', 'E', 'x', 'X', 'o', 'O', 'b', 'B', 'n', 'i', 'N'])
        || (v.len() >= 2
            && v.starts_with('"')
            && v.ends_with('"')
            && !v[1..v.len() - 1].contains(['"', '\\']))
}

/// A value the dump's factories make (`$dump.s3.f`): written after the block's code.
fn has_factory_ref(v: &str) -> bool {
    let b = v.as_bytes();
    let mut i = 0;
    while let Some(at) = v[i..].find("$dump.s") {
        let j = i + at + "$dump.s".len();
        if b.get(j).is_some_and(u8::is_ascii_digit) {
            return true;
        }
        i = j;
    }
    false
}

fn kind_name(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Body => "a function body",
        NodeKind::Block => "a block",
        NodeKind::Expr => "an expression statement",
        NodeKind::Decl(_) => "a declaration",
        NodeKind::If => "if",
        NodeKind::Label => "a label",
        NodeKind::While => "while",
        NodeKind::DoWhile => "do-while",
        NodeKind::For => "for",
        NodeKind::ForIn => "for-in",
        NodeKind::ForOf => "for-of",
        NodeKind::ForAwait => "for await",
        NodeKind::Try => "try",
        NodeKind::Switch => "switch",
        NodeKind::Return => "return",
        NodeKind::Throw => "throw",
        NodeKind::Await => "await",
        NodeKind::Yield => "yield",
        NodeKind::YieldStar => "yield* (0203)",
        NodeKind::Call { .. } => "a call",
        NodeKind::New => "new",
        NodeKind::ImportCall => "import() (0203)",
        NodeKind::Member { .. } => "a member",
        NodeKind::Binary => "a binary operator",
        NodeKind::Logical => "a logical operator",
        NodeKind::Assign => "an assignment",
        NodeKind::Unary => "a unary operator",
        NodeKind::Update => "an update (0203)",
        NodeKind::Conditional => "a conditional",
        NodeKind::Sequence => "a sequence",
        NodeKind::Template => "a template literal",
        NodeKind::TaggedTemplate => "a tagged template (0203)",
        NodeKind::Array => "an array literal",
        NodeKind::Object => "an object literal",
        NodeKind::ArrayPattern => "an array pattern (0203)",
        NodeKind::ObjectPattern => "an object pattern (0203)",
        NodeKind::Default => "a default value (0203)",
    }
}
