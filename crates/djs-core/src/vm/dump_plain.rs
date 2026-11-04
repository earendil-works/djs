//! Plain dumps (format 3, D2830, docs/plain-js-dumps.md): the dump as a Script that runs
//! given only the host's functions, in djs and in node. Every suspended activation is a
//! *continuation* (dump_cont.rs): a function whose body is the rest of the original from
//! its site, which is also the factory of the scopes its frame shares with closures.
//!
//! The exporter writes the format 2 lines as always (its checker compiles them), with
//! the plain forms at the source (`$dump.bN` for builtins, `$djs.resume(...)` for open
//! calls, host functions by name), and records here what the lines do not say: the
//! scope groups' factories (variables, closures and what each captures) and the
//! activations (function, site, slots, cells). The writer then
//!
//! - *merges* the factories whose variables a frame shares: their variables become
//!   declarations of continuations, their closures are made inside them (placement);
//! - *schedules* the lines: a continuation is called where its activation was, unless
//!   code written before needs what its prefix makes; then it is called there and what
//!   lies between goes into a callback its prefix calls right before the site;
//! - writes the helpers the dump uses on a plain global `$dump`, deleted at the end, and
//!   the run's handle as the Script's completion value.

use alloc::string::String;
use alloc::vec::Vec;

use super::*;
use crate::compiler::Site;

/// What the plain writer needs besides the format 2 lines.
#[derive(Default)]
pub(super) struct Ir {
    /// Builtins no expression on the globals reaches, by path: `$dump.b{i}`.
    pub builtins: Vec<String>,
    pub factories: Vec<Factory>,
    pub acts: Vec<Act>,
    /// The line of `out` that writes a factory or an activation.
    pub lines: Vec<(usize, Rec)>,
    /// `Math.random`'s state.
    pub seed: (i32, i32),
    /// The handles (at most one is written).
    pub handles: Vec<(String, String)>,
}

#[derive(Clone, Copy)]
pub(super) enum Rec {
    Factory(usize),
    Act(usize),
}

/// A scope group's factory (`$dump.sN = (() => {...})()`), by scopes: scope 0 is the
/// factory, the others nested factories whose objects their parent spreads.
pub(super) struct Factory {
    /// `sN`.
    pub name: String,
    pub group: usize,
    pub scopes: Vec<Scope>,
}

pub(super) struct Scope {
    /// The enclosing scope (`usize::MAX` for scope 0).
    pub parent: usize,
    /// Statements that are not variables or closures (class definitions).
    pub other: Vec<String>,
    pub vars: Vec<Var>,
    pub props: Vec<Prop>,
}

pub(super) struct Var {
    pub name: String,
    /// Its line in the factory (empty in its TDZ).
    pub line: String,
    /// `None`: declared without a value (set later by `$set`).
    pub init: Option<String>,
    /// In its TDZ (declared after the factory's `return`).
    pub tdz: bool,
    /// Its box's node.
    pub boxn: usize,
    /// A frame shares it: the key of its cell (`$cB`).
    pub cell: Option<String>,
}

/// A member of a scope's returned object.
pub(super) struct Prop {
    pub key: String,
    /// As written in the object: `key`, `key: expr`, a method.
    pub text: String,
    /// The boxes (variables) it captures.
    pub caps: Vec<usize>,
}

pub(super) struct Act {
    /// `aN`.
    pub name: String,
    /// The function's node and the expression the dump reaches it by.
    pub fnode: usize,
    pub fexpr: String,
    /// The function's text and its kind (`func_flags::KIND_*`).
    pub ftext: String,
    pub site: Site,
    pub awaiting: Option<String>,
    pub this: String,
    pub args: Vec<String>,
    pub slots: Vec<Slot>,
    /// An async generator's requests: type, value, promise.
    pub queue: Vec<(&'static str, String, String)>,
    /// `suspend::ASYNC`, `GENERATOR`, `ASYNC_GENERATOR`.
    pub kind: i32,
}

pub(super) struct Slot {
    pub name: String,
    pub value: String,
    /// A variable the frame shares with closures: its box's node.
    pub cell: Option<usize>,
}

// ---------------------------------------------------------------- references

/// What a piece of the dump's text refers to: `$dump.X` (a location), `$dump.X.k` (a
/// key of a factory's object, `key` set) and bare names (the dump's top-level
/// declarations among them).
#[derive(Default)]
pub(super) struct Refs {
    /// `(name, key)`: `$dump.name` or `$dump.name.key`.
    pub dump: Vec<(String, Option<String>)>,
    pub names: Vec<String>,
}

fn ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'$' || c >= 0x80
}

fn ident_part(c: u8) -> bool {
    ident_start(c) || c.is_ascii_digit()
}

/// The references of `text`, JavaScript the exporter wrote (data, factories, function
/// sources): strings, template text, comments and regular expressions are skipped; a
/// name after `.` is a property, not a reference. It errs on the side of too many
/// names (an object literal's keys count), which only adds orderings that hold anyway.
pub(super) fn refs(text: &str) -> Refs {
    let b = text.as_bytes();
    let mut out = Refs::default();
    let mut i = 0;
    // Template literal nesting: brace depth at each `${`.
    let mut tpl: Vec<u32> = Vec::new();
    let mut depth: u32 = 0;
    // Whether a `/` here starts a regular expression (after an operator or `(`).
    let mut regex_ok = true;
    let mut after_dot = false;
    while i < b.len() {
        let c = b[i];
        match c {
            b' ' | b'\t' | b'\n' | b'\r' => {
                i += 1;
                continue;
            }
            b'"' | b'\'' => {
                i += 1;
                while i < b.len() && b[i] != c {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
                regex_ok = false;
                after_dot = false;
                continue;
            }
            b'`' => {
                i = template_text(b, i + 1, &mut tpl, depth);
                regex_ok = false;
                after_dot = false;
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
                continue;
            }
            b'/' if regex_ok => {
                i += 1;
                let mut class = false;
                while i < b.len() && (b[i] != b'/' || class) && b[i] != b'\n' {
                    match b[i] {
                        b'\\' => i += 1,
                        b'[' => class = true,
                        b']' => class = false,
                        _ => {}
                    }
                    i += 1;
                }
                i += 1;
                while i < b.len() && ident_part(b[i]) {
                    i += 1;
                }
                regex_ok = false;
                after_dot = false;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                if tpl.last() == Some(&depth) {
                    tpl.pop();
                    i = template_text(b, i + 1, &mut tpl, depth);
                    regex_ok = false;
                    after_dot = false;
                    continue;
                }
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
        if ident_start(c) {
            let s = i;
            while i < b.len() && ident_part(b[i]) {
                i += 1;
            }
            let word = &text[s..i];
            if !after_dot {
                if word == "$dump" && b.get(i) == Some(&b'.') {
                    let (name, j) = ident_at(text, i + 1);
                    if !name.is_empty() {
                        let (key, k) = if b.get(j) == Some(&b'.') {
                            ident_at(text, j + 1)
                        } else {
                            ("", j)
                        };
                        let key = (!key.is_empty()).then(|| String::from(key));
                        if !out.dump.iter().any(|(n, k)| n == name && *k == key) {
                            out.dump.push_out((String::from(name), key));
                        }
                        i = k;
                        regex_ok = false;
                        after_dot = false;
                        continue;
                    }
                } else if !out.names.iter().any(|n| n == word) {
                    out.names.push_out(String::from(word));
                }
            }
            // After a keyword an expression starts (`return /x/`).
            regex_ok = matches!(
                word,
                "return" | "typeof" | "instanceof" | "in" | "of" | "new" | "delete" | "void"
                    | "throw" | "case" | "do" | "else" | "yield" | "await"
            );
            after_dot = false;
            continue;
        }
        if c.is_ascii_digit() {
            while i < b.len() && (ident_part(b[i]) || b[i] == b'.') {
                i += 1;
            }
            regex_ok = false;
            after_dot = false;
            continue;
        }
        after_dot = c == b'.' && !(b.get(i + 1) == Some(&b'.'));
        if c == b'.' && b.get(i + 1) == Some(&b'.') {
            // `...`
            i += 3;
            regex_ok = true;
            continue;
        }
        regex_ok = !matches!(c, b')' | b']' | b'}');
        i += 1;
    }
    out
}

/// Skips a template literal's text from `i` to its end or its next `${`.
fn template_text(b: &[u8], mut i: usize, tpl: &mut Vec<u32>, depth: u32) -> usize {
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'`' => return i + 1,
            b'$' if b.get(i + 1) == Some(&b'{') => {
                tpl.push(depth);
                return i + 2;
            }
            _ => i += 1,
        }
    }
    i
}

fn ident_at(text: &str, i: usize) -> (&str, usize) {
    let b = text.as_bytes();
    let mut j = i;
    while j < b.len() && ident_part(b[j]) {
        j += 1;
    }
    if i < b.len() && ident_start(b[i]) {
        (&text[i..j], j)
    } else {
        ("", i)
    }
}

// ---------------------------------------------------------------- the writer

/// A scope of a factory: (factory, scope).
type Sc = (usize, usize);

/// Where code is written: the dump's top level, block `i` of activation `a`'s
/// continuation, the block of a merged scope's variables no frame takes (a dead group),
/// or an unmerged factory.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pl {
    Top,
    Block(usize, usize),
    Group(Sc),
    Fac(Sc),
}

/// What is written at a place.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Put {
    Prop(Sc, usize),
    Sub(Sc),
    Group(Sc),
    K(usize),
}

#[derive(Clone, PartialEq, Eq)]
enum IK {
    /// `let`/`const`/`var`/`class` at the top: the names it declares.
    Decl(Vec<String>),
    /// A function declaration (hoisted to the head).
    FnDecl,
    /// `$dump.name = ...`.
    Def(String),
    Stmt,
    Factory(usize),
    Act(usize),
}

struct Item {
    kind: IK,
    text: String,
    deps: Vec<usize>,
    /// An activation's: what its awaited promise needs; per slot, what its value needs.
    start_deps: Vec<usize>,
    slot_deps: Vec<(String, Vec<usize>)>,
    pure: bool,
}

/// A binding a block of a continuation declares (the generator's dry run lists them).
#[derive(Clone)]
pub(super) struct BDecl {
    pub name: String,
    /// Its slot name in the frame, if it has one.
    pub sname: Option<String>,
    pub live: bool,
}

/// What the generator asks of the dump while it writes a continuation.
pub(super) trait Ctx {
    /// The expression of slot `name`, if the frame has it.
    fn slot(&mut self, name: &str) -> Option<String>;
    /// A variable the frame shares with closures: `Some(None)` in its TDZ, `Some(value)`
    /// else; `None`: not shared.
    fn cell(&mut self, sname: &str) -> Option<Option<String>>;
    /// The binding's value exists only after the dump's callback ran.
    fn after_go(&self, sname: &str) -> bool;
    /// The code the dump puts in the continuation's block `index`.
    fn block(&mut self, index: usize, info: &[BDecl]) -> Result<String, String>;
    /// The call of the dump's callback, or "".
    fn go(&self) -> String;
    fn awaiting(&self) -> String;
}

/// A continuation's text and its origin record (D2833).
#[derive(Clone, Default)]
pub(super) struct KText {
    pub text: String,
    /// `[kStart, fStart, len]` triples.
    pub segs: Vec<u32>,
    /// The scaffold constructs and the temporaries' origins, as JSON.
    pub scaf: String,
    pub cmap: String,
    /// The slots it consumed.
    pub used: Vec<String>,
}

#[derive(Default)]
struct AState {
    /// Where its function lives: a prop of a factory scope.
    home: Option<(Sc, usize)>,
    /// Its slots that are variables of merged factories: slot name, (scope, var).
    cells: Vec<(String, Sc, usize)>,
    /// Its blocks' declarations, from the dry run.
    blocks: Vec<Vec<BDecl>>,
    place: Option<Pl>,
    /// Slot values written before the continuation (they read names it may shadow).
    pre: Vec<(String, String)>,
    after_go: Vec<String>,
    needs_go: bool,
    go: String,
    k: Option<KText>,
    blocks_done: bool,
}

/// One entry of the schedule: an item, or an activation's call with its callback.
enum Plan {
    Item(usize),
    Call(usize, Option<Vec<Plan>>),
}

struct W<'x> {
    ir: &'x Ir,
    items: Vec<Item>,
    acts: Vec<AState>,
    /// Per scope: whether a frame takes one of its variables.
    merged: Vec<Vec<bool>>,
    /// Per (factory, scope, var): the activation and slot that own it, and its block.
    owner: Vec<Vec<Vec<Option<(usize, String)>>>>,
    vblock: Vec<Vec<Vec<Option<Pl>>>>,
    /// Box node -> (scope, var).
    boxes: Vec<(usize, Sc, usize)>,
    placed: Vec<(Pl, Put)>,
    act_place: Vec<Option<Pl>>,
    group_place: Vec<(Sc, Pl)>,
    in_progress: Vec<Sc>,
    /// The dump's top-level names (roots, functions, classes).
    globals: Vec<String>,
    gen: &'x dyn Fn(&Act, &mut dyn Ctx) -> Result<KText, String>,
}

type R<T> = Result<T, String>;

fn refuse<T>(why: &str) -> R<T> {
    Err(String::from(why))
}

/// Writes the plain dump: `lines` are the format 2 lines (`out`, then the tail of
/// builtin changes, patches and finals); `gen` writes a continuation.
pub(super) fn write(
    ir: &Ir,
    lines: &[String],
    ts: bool,
    gen: &dyn Fn(&Act, &mut dyn Ctx) -> Result<KText, String>,
) -> R<String> {
    if ir.handles.len() > 1 {
        return refuse("a plain dump holds one handle");
    }
    let mut w = W {
        ir,
        items: Vec::new(),
        acts: (0..ir.acts.len()).map(|_| AState::default()).collect(),
        merged: ir
            .factories
            .iter()
            .map(|f| alloc::vec![false; f.scopes.len()])
            .collect(),
        owner: ir
            .factories
            .iter()
            .map(|f| f.scopes.iter().map(|s| alloc::vec![None; s.vars.len()]).collect())
            .collect(),
        vblock: ir
            .factories
            .iter()
            .map(|f| f.scopes.iter().map(|s| alloc::vec![None; s.vars.len()]).collect())
            .collect(),
        boxes: Vec::new(),
        placed: Vec::new(),
        act_place: alloc::vec![None; ir.acts.len()],
        group_place: Vec::new(),
        in_progress: Vec::new(),
        globals: Vec::new(),
        gen,
    };
    for (f, fac) in ir.factories.iter().enumerate() {
        for (s, sc) in fac.scopes.iter().enumerate() {
            for (v, var) in sc.vars.iter().enumerate() {
                w.boxes.push_out((var.boxn, (f, s), v));
            }
        }
    }
    w.read_items(lines)?;
    w.activations()?;
    w.place_all()?;
    let plan = w.schedule()?;
    w.emit(&plan, ts)
}

impl W<'_> {
    fn scope(&self, sc: Sc) -> &Scope {
        &self.ir.factories[sc.0].scopes[sc.1]
    }

    fn parent_scope(&self, sc: Sc) -> Option<Sc> {
        let p = self.scope(sc).parent;
        (p != usize::MAX).then_some((sc.0, p))
    }

    fn box_var(&self, b: usize) -> Option<(Sc, usize)> {
        self.boxes
            .iter()
            .find(|x| x.0 == b)
            .map(|&(_, sc, v)| (sc, v))
    }

    fn is_merged(&self, sc: Sc) -> bool {
        self.merged[sc.0][sc.1]
    }

    // ------------------------------------------------------------ items

    fn read_items(&mut self, lines: &[String]) -> R<()> {
        for (i, l) in lines.iter().enumerate() {
            let rec = self.ir.lines.iter().find(|x| x.0 == i).map(|x| x.1);
            let kind = match rec {
                Some(Rec::Factory(f)) => IK::Factory(f),
                Some(Rec::Act(a)) => IK::Act(a),
                None => {
                    if l.starts_with("$dump.handle(") {
                        continue;
                    }
                    if l.contains("$dump.fromSync(") {
                        return refuse("plain dumps: a for await over a sync iterator (0203)");
                    }
                    classify(l)
                }
            };
            match &kind {
                IK::Decl(names) => self.globals.extend(names.iter().cloned()),
                IK::FnDecl => {
                    if let Some(n) = declared_fn_name(l) {
                        self.globals.push_out(n);
                    }
                }
                _ => {}
            }
            self.items.push_out(Item {
                kind,
                text: l.clone(),
                deps: Vec::new(),
                start_deps: Vec::new(),
                slot_deps: Vec::new(),
                pure: false,
            });
        }
        Ok(())
    }

    // ------------------------------------------------------------ activations

    /// The function's home and cells of every activation, the merged scopes, and the
    /// blocks of each continuation (a dry run of the generator).
    fn activations(&mut self) -> R<()> {
        let ir = self.ir;
        for (a, act) in ir.acts.iter().enumerate() {
            // Its function: a closure of a factory (`$dump.sN.key`), or written at the top.
            if let Some(rest) = act.fexpr.strip_prefix("$dump.")
                && let Some((top, key)) = rest.split_once('.')
                && let Some(f) = ir.factories.iter().position(|f| f.name == top)
            {
                let found = ir.factories[f].scopes.iter().enumerate().find_map(|(s, sc)| {
                    sc.props.iter().position(|p| p.key == key).map(|p| ((f, s), p))
                });
                match found {
                    Some(h) => self.acts[a].home = Some(h),
                    None => return refuse("an activation of a factory's cell"),
                }
            }
            for sl in &act.slots {
                if let Some(b) = sl.cell {
                    let Some((sc, v)) = self.box_var(b) else {
                        return refuse("a cell without its variable");
                    };
                    self.acts[a].cells.push_out((sl.name.clone(), sc, v));
                    self.owner[sc.0][sc.1][v] = Some((a, sl.name.clone()));
                    self.merged[sc.0][sc.1] = true;
                }
            }
        }
        for a in 0..ir.acts.len() {
            self.dry(a)?;
        }
        for a in 0..ir.acts.len() {
            for (i, b) in self.acts[a].blocks.clone().iter().enumerate() {
                for d in b {
                    let Some(sn) = &d.sname else { continue };
                    if let Some(&(_, sc, v)) = self.acts[a].cells.iter().find(|c| c.0 == *sn) {
                        self.vblock[sc.0][sc.1][v] = Some(Pl::Block(a, i));
                    }
                }
            }
            for &(ref sn, sc, v) in &self.acts[a].cells {
                if self.vblock[sc.0][sc.1][v].is_none() {
                    return Err(cat!("no block of the continuation declares ", sn));
                }
            }
        }
        for (f, fac) in ir.factories.iter().enumerate() {
            for (s, sc) in fac.scopes.iter().enumerate() {
                if self.merged[f][s] && !sc.other.is_empty() {
                    return refuse("plain dumps: a class in a scope a frame shares (0203)");
                }
            }
        }
        Ok(())
    }

    fn dry(&mut self, a: usize) -> R<()> {
        let gen = self.gen;
        let act = &self.ir.acts[a];
        let mut c = C {
            w: self,
            a,
            dry: true,
        };
        gen(act, &mut c)?;
        self.acts[a].blocks_done = true;
        Ok(())
    }

    /// A slot's value as continuation text: one that reads a name is evaluated before
    /// the continuation (its own declarations could shadow the name).
    fn slot_value(&mut self, a: usize, name: &str) -> Option<String> {
        let act = &self.ir.acts[a];
        let sl = act.slots.iter().find(|s| s.name == name)?;
        let r = refs(&sl.value);
        let reads = r.names.iter().any(|n| n != "undefined" && n != "true" && n != "false" && n != "null");
        let st = &mut self.acts[a];
        if !reads || st.after_go.iter().any(|n| n == name) {
            return Some(sl.value.clone());
        }
        if let Some(p) = st.pre.iter().find(|p| p.0 == name) {
            return Some(p.1.clone());
        }
        let r = cat!("$dump.", act.name, "_", st.pre.len());
        st.pre.push_out((String::from(name), r.clone()));
        Some(r)
    }
}

/// The generator's view of the dump while it writes activation `a`'s continuation.
struct C<'w, 'x> {
    w: &'w mut W<'x>,
    a: usize,
    dry: bool,
}

impl Ctx for C<'_, '_> {
    fn slot(&mut self, name: &str) -> Option<String> {
        self.w.slot_value(self.a, name)
    }

    fn cell(&mut self, sname: &str) -> Option<Option<String>> {
        let &(_, sc, v) = self.w.acts[self.a].cells.iter().find(|c| c.0 == sname)?;
        let var = &self.w.scope(sc).vars[v];
        Some(if var.tdz { None } else { Some(var.init.clone().unwrap_or_else(|| "undefined".into())) })
    }

    fn after_go(&self, sname: &str) -> bool {
        self.w.acts[self.a].after_go.iter().any(|n| n == sname)
    }

    fn block(&mut self, index: usize, info: &[BDecl]) -> R<String> {
        if self.dry {
            if !self.w.acts[self.a].blocks_done {
                self.w.acts[self.a].blocks.push_out(info.to_vec());
            }
            return Ok(String::new());
        }
        let lines = self.w.emit_place(Pl::Block(self.a, index))?;
        Ok(join(&lines, "\n"))
    }

    fn go(&self) -> String {
        if self.dry {
            String::new()
        } else {
            self.w.acts[self.a].go.clone()
        }
    }

    fn awaiting(&self) -> String {
        self.w.ir.acts[self.a]
            .awaiting
            .clone()
            .unwrap_or_else(|| "undefined".into())
    }
}

/// What a top-level line of the format 2 text is.
fn classify(l: &str) -> IK {
    if let Some(rest) = l.strip_prefix("$dump.") {
        let (name, after) = ident_at(rest, 0);
        if !name.is_empty() && rest[after..].starts_with(" = ") {
            return IK::Def(String::from(name));
        }
    }
    if declared_fn_name(l).is_some() {
        return IK::FnDecl;
    }
    for kw in ["let ", "const ", "var ", "class "] {
        if let Some(rest) = l.strip_prefix(kw) {
            let (name, _) = ident_at(rest, 0);
            return IK::Decl(alloc::vec![String::from(name)]);
        }
    }
    IK::Stmt
}

fn declared_fn_name(l: &str) -> Option<String> {
    let rest = l.strip_prefix("async ").unwrap_or(l);
    let rest = rest.strip_prefix("function")?;
    let rest = rest.strip_prefix('*').unwrap_or(rest).trim_start();
    let (name, _) = ident_at(rest, 0);
    (!name.is_empty()).then(|| String::from(name))
}

// ---------------------------------------------------------------- placement

impl W<'_> {
    fn parent_of(&mut self, pl: Pl) -> R<Option<Pl>> {
        Ok(match pl {
            Pl::Block(a, i) if i > 0 => Some(Pl::Block(a, i - 1)),
            Pl::Block(a, _) => Some(self.place_of_act(a)?),
            Pl::Group(sc) => Some(self.place_of_group(sc)?),
            Pl::Fac(sc) => match self.parent_scope(sc) {
                Some(p) => Some(self.factory_place(p)?),
                None => Some(Pl::Top),
            },
            Pl::Top => None,
        })
    }

    fn ancestors(&mut self, pl: Pl) -> R<Vec<Pl>> {
        let mut out = alloc::vec![pl];
        let mut p = pl;
        while let Some(q) = self.parent_of(p)? {
            if out.len() > 10_000 {
                return refuse("cyclic placement");
            }
            out.push_out(q);
            p = q;
        }
        Ok(out)
    }

    /// The innermost of `pls`, which must lie on one chain of places.
    fn innermost(&mut self, pls: &[Pl], what: &str) -> R<Option<Pl>> {
        let mut uniq: Vec<Pl> = Vec::new();
        for &p in pls {
            if !uniq.contains(&p) {
                uniq.push_out(p);
            }
        }
        for &c in &uniq {
            let anc = self.ancestors(c)?;
            if uniq.iter().all(|o| anc.contains(o)) {
                return Ok(Some(c));
            }
        }
        if uniq.is_empty() {
            return Ok(None);
        }
        Err(cat!(what, ": scopes of different frames"))
    }

    fn factory_place(&mut self, sc: Sc) -> R<Pl> {
        if self.is_merged(sc) {
            self.place_of_group(sc)
        } else {
            Ok(Pl::Fac(sc))
        }
    }

    /// The scopes whose variables code of `sc` can capture: it and its merged ancestors.
    fn chain(&self, sc: Sc) -> Vec<Sc> {
        let mut out = Vec::new();
        let mut p = Some(sc);
        while let Some(q) = p {
            if !self.is_merged(q) {
                break;
            }
            out.push_out(q);
            p = self.parent_scope(q);
        }
        out
    }

    fn dead(&self, sc: Sc, v: usize) -> bool {
        self.owner[sc.0][sc.1][v].is_none()
    }

    fn has_dead(&self, sc: Sc) -> bool {
        (0..self.scope(sc).vars.len()).any(|v| self.dead(sc, v))
    }

    /// The place of code of merged scope `sc` that captures `caps`.
    fn place_of_code(&mut self, sc: Sc, caps: &[usize]) -> R<Pl> {
        let chain = self.chain(sc);
        let mut pls = Vec::new();
        for &b in caps {
            let Some((vs, v)) = self.box_var(b) else { continue };
            if !chain.contains(&vs) {
                continue;
            }
            match self.vblock[vs.0][vs.1][v] {
                Some(pl) if !self.dead(vs, v) => pls.push_out(pl),
                _ => pls.push_out(Pl::Group(vs)),
            }
        }
        if let Some(pl) = self.innermost(&pls, "a closure")? {
            return Ok(pl);
        }
        // It captures nothing of the merged scopes: the nearest unmerged factory.
        let mut p = Some(sc);
        while let Some(q) = p {
            if !self.is_merged(q) {
                return self.factory_place(q);
            }
            p = self.parent_scope(q);
        }
        Ok(Pl::Top)
    }

    /// The place of the block of merged scope `sc`'s variables no frame takes.
    fn place_of_group(&mut self, sc: Sc) -> R<Pl> {
        if let Some(&(_, pl)) = self.group_place.iter().find(|g| g.0 == sc) {
            return Ok(pl);
        }
        if self.in_progress.contains(&sc) {
            return refuse("cyclic group placement");
        }
        self.in_progress.push_out(sc);
        let chain = self.chain(sc);
        let mut pls = Vec::new();
        for (_, caps) in self.factory_code(sc) {
            let touches = caps.iter().any(|&b| {
                self.box_var(b)
                    .is_some_and(|(vs, v)| vs == sc && self.dead(vs, v))
            });
            if !touches {
                continue;
            }
            for &b in &caps {
                let Some((vs, v)) = self.box_var(b) else { continue };
                if !chain.contains(&vs) || vs == sc {
                    continue;
                }
                match self.vblock[vs.0][vs.1][v] {
                    Some(pl) if !self.dead(vs, v) => pls.push_out(pl),
                    _ => {
                        let g = self.place_of_group(vs)?;
                        pls.push_out(g);
                    }
                }
            }
        }
        let pl = match self.innermost(&pls, "a dead scope")? {
            Some(pl) => pl,
            None => {
                let mut p = self.parent_scope(sc);
                while let Some(q) = p {
                    if !(self.is_merged(q) && !self.has_dead(q)) {
                        break;
                    }
                    p = self.parent_scope(q);
                }
                match p {
                    None => Pl::Top,
                    Some(q) if self.is_merged(q) => Pl::Group(q),
                    Some(q) => Pl::Fac(q),
                }
            }
        };
        self.in_progress.retain(|&x| x != sc);
        self.group_place.push_out((sc, pl));
        Ok(pl)
    }

    /// The code of a scope that is placed (its closures, its unmerged nested scopes),
    /// with the variables each captures.
    fn factory_code(&self, sc: Sc) -> Vec<(Put, Vec<usize>)> {
        let mut out = Vec::new();
        for (i, p) in self.scope(sc).props.iter().enumerate() {
            out.push_out((Put::Prop(sc, i), p.caps.clone()));
        }
        for (k, child) in self.ir.factories[sc.0].scopes.iter().enumerate() {
            if child.parent == sc.1 && !self.is_merged((sc.0, k)) {
                out.push_out((Put::Sub((sc.0, k)), self.free_caps((sc.0, k))));
            }
        }
        out
    }

    /// What the code of scope `sc` and its nested scopes captures from outside them.
    fn free_caps(&self, sc: Sc) -> Vec<usize> {
        let inside = |w: &Self, b: usize| {
            let mut p = w.box_var(b).map(|x| x.0);
            while let Some(q) = p {
                if q == sc {
                    return true;
                }
                p = w.parent_scope(q);
            }
            false
        };
        let mut out = Vec::new();
        for (k, s) in self.ir.factories[sc.0].scopes.iter().enumerate() {
            let mut p = Some((sc.0, k));
            let mut within = false;
            while let Some(q) = p {
                if q == sc {
                    within = true;
                    break;
                }
                p = self.parent_scope(q);
            }
            if !within {
                continue;
            }
            for pr in &s.props {
                for &b in &pr.caps {
                    if !inside(self, b) && !out.contains(&b) {
                        out.push_out(b);
                    }
                }
            }
        }
        out
    }

    fn code_place(&mut self, put: Put, caps: &[usize]) -> R<Pl> {
        let sc = match put {
            Put::Prop(sc, _) | Put::Sub(sc) => sc,
            _ => return refuse("internal: code_place"),
        };
        let sc = match put {
            Put::Sub(_) => self.parent_scope(sc).unwrap_or(sc),
            _ => sc,
        };
        let touches_dead = caps.iter().any(|&b| {
            self.box_var(b)
                .is_some_and(|(vs, v)| vs == sc && self.dead(vs, v))
        });
        if touches_dead {
            return Ok(Pl::Group(sc));
        }
        self.place_of_code(sc, caps)
    }

    /// Where activation `a`'s continuation is written: where its function is.
    fn place_of_act(&mut self, a: usize) -> R<Pl> {
        if let Some(pl) = self.act_place[a] {
            return Ok(pl);
        }
        let pl = match self.acts[a].home {
            None => Pl::Top,
            Some((sc, _)) if !self.is_merged(sc) => self.factory_place(sc)?,
            Some((sc, p)) => {
                let caps = self.scope(sc).props[p].caps.clone();
                self.code_place(Put::Prop(sc, p), &caps)?
            }
        };
        self.act_place[a] = Some(pl);
        Ok(pl)
    }

    fn place_all(&mut self) -> R<()> {
        let ir = self.ir;
        for (f, fac) in ir.factories.iter().enumerate() {
            for s in 0..fac.scopes.len() {
                let sc = (f, s);
                if !self.is_merged(sc) {
                    continue;
                }
                for (put, caps) in self.factory_code(sc) {
                    let pl = self.code_place(put, &caps)?;
                    self.placed.push_out((pl, put));
                }
                if self.has_dead(sc) {
                    let pl = self.place_of_group(sc)?;
                    self.placed.push_out((pl, Put::Group(sc)));
                }
            }
        }
        for a in 0..ir.acts.len() {
            let pl = self.place_of_act(a)?;
            if !matches!(pl, Pl::Top | Pl::Fac(_)) {
                self.placed.push_out((pl, Put::K(a)));
            }
            self.acts[a].place = Some(pl);
        }
        Ok(())
    }
}
