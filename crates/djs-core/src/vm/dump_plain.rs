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
    pub fkind: u32,
    pub site: Site,
    pub syntax: alloc::rc::Rc<crate::compiler::Syntax>,
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
                "return"
                    | "typeof"
                    | "instanceof"
                    | "in"
                    | "of"
                    | "new"
                    | "delete"
                    | "void"
                    | "throw"
                    | "case"
                    | "do"
                    | "else"
                    | "yield"
                    | "await"
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
    write_k: &'x dyn Fn(&Act, &mut dyn Ctx) -> Result<KText, String>,
}

type R<T> = Result<T, String>;

fn refuse<T>(why: &str) -> R<T> {
    Err(String::from(why))
}

/// Writes the plain dump: `lines` are the format 2 lines (`out`, then the tail of
/// builtin changes, patches and finals); `write_k` writes a continuation.
pub(super) fn write(
    ir: &Ir,
    lines: &[String],
    ts: bool,
    write_k: &dyn Fn(&Act, &mut dyn Ctx) -> Result<KText, String>,
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
            .map(|f| {
                f.scopes
                    .iter()
                    .map(|s| alloc::vec![None; s.vars.len()])
                    .collect()
            })
            .collect(),
        vblock: ir
            .factories
            .iter()
            .map(|f| {
                f.scopes
                    .iter()
                    .map(|s| alloc::vec![None; s.vars.len()])
                    .collect()
            })
            .collect(),
        boxes: Vec::new(),
        placed: Vec::new(),
        act_place: alloc::vec![None; ir.acts.len()],
        group_place: Vec::new(),
        in_progress: Vec::new(),
        globals: Vec::new(),
        write_k,
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
            self.items.push(Item {
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
                let found = ir.factories[f]
                    .scopes
                    .iter()
                    .enumerate()
                    .find_map(|(s, sc)| {
                        sc.props
                            .iter()
                            .position(|p| p.key == key)
                            .map(|p| ((f, s), p))
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
        let write_k = self.write_k;
        let act = &self.ir.acts[a];
        let mut c = C {
            w: self,
            a,
            dry: true,
        };
        write_k(act, &mut c)?;
        self.acts[a].blocks_done = true;
        Ok(())
    }

    /// A slot's value as continuation text: one that reads a name is evaluated before
    /// the continuation (its own declarations could shadow the name).
    fn slot_value(&mut self, a: usize, name: &str) -> Option<String> {
        let act = &self.ir.acts[a];
        let sl = act.slots.iter().find(|s| s.name == name)?;
        let r = refs(&sl.value);
        let reads = r
            .names
            .iter()
            .any(|n| n != "undefined" && n != "true" && n != "false" && n != "null");
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
        Some(if var.tdz {
            None
        } else {
            Some(var.init.clone().unwrap_or_else(|| "undefined".into()))
        })
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
            let Some((vs, v)) = self.box_var(b) else {
                continue;
            };
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
                let Some((vs, v)) = self.box_var(b) else {
                    continue;
                };
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

// ---------------------------------------------------------------- scheduling

/// A dependency key: `$dump.name`, a key of a merged factory (`name.key`), a global.
#[derive(Clone, PartialEq, Eq)]
enum Key {
    Loc(String),
    FKey(String, String),
    Global(String),
}

impl W<'_> {
    fn keys_of(&self, text: &str) -> Vec<Key> {
        let r = refs(text);
        let mut out = Vec::new();
        for (name, key) in r.dump {
            let k = match (key, self.ir.factories.iter().position(|f| f.name == name)) {
                (Some(key), Some(_)) => Key::FKey(name, key),
                _ => Key::Loc(name),
            };
            if !out.contains(&k) {
                out.push_out(k);
            }
        }
        for n in r.names {
            if self.globals.contains(&n) {
                out.push_out(Key::Global(n));
            }
        }
        out
    }

    /// The activation whose continuation makes key `key` of factory `name`; `Ok(None)`
    /// when the factory makes it.
    fn creator_of(&mut self, name: &str, key: &str) -> R<Option<usize>> {
        let ir = self.ir;
        let Some(f) = ir.factories.iter().position(|x| x.name == name) else {
            return Ok(None);
        };
        let mut pl = None;
        for (s, sc) in ir.factories[f].scopes.iter().enumerate() {
            if let Some(p) = sc.props.iter().position(|p| p.key == key) {
                let caps = sc.props[p].caps.clone();
                pl = Some(if self.is_merged((f, s)) {
                    self.code_place(Put::Prop((f, s), p), &caps)?
                } else {
                    self.factory_place((f, s))?
                });
            }
        }
        if pl.is_none()
            && let Some(an) = key.strip_prefix("$K")
            && let Some(a) = ir.acts.iter().position(|a| a.name == an)
        {
            pl = self.acts[a].place;
        }
        let mut p = pl;
        let mut n = 0;
        while let Some(q) = p {
            n += 1;
            if n > 10_000 {
                return refuse("cyclic placement");
            }
            match q {
                Pl::Block(a, _) => return Ok(Some(a)),
                Pl::Fac(sc) if self.parent_scope(sc).is_none() => return Ok(None),
                _ => {}
            }
            p = self.parent_of(q)?;
        }
        Ok(None)
    }

    fn act_item(&self, a: usize) -> usize {
        self.items
            .iter()
            .position(|it| it.kind == IK::Act(a))
            .unwrap_or(usize::MAX)
    }

    fn dep_items(&mut self, keys: &[Key], me: usize, providers: &[(Key, usize)]) -> R<Vec<usize>> {
        let mut out = Vec::new();
        for k in keys {
            let d = match k {
                Key::FKey(n, key) => {
                    let merged_key = {
                        let f = self.ir.factories.iter().position(|x| x.name == *n);
                        f.is_some_and(|f| {
                            (0..self.ir.factories[f].scopes.len()).any(|s| self.is_merged((f, s)))
                        })
                    };
                    if merged_key {
                        match self.creator_of(n, key)? {
                            Some(a) => Some(self.act_item(a)),
                            None => providers
                                .iter()
                                .find(|p| p.0 == Key::Loc(n.clone()))
                                .map(|p| p.1),
                        }
                    } else {
                        providers
                            .iter()
                            .find(|p| p.0 == Key::Loc(n.clone()))
                            .map(|p| p.1)
                    }
                }
                k => providers.iter().find(|p| p.0 == *k).map(|p| p.1),
            };
            if let Some(d) = d
                && d != me
                && d != usize::MAX
                && !out.contains(&d)
            {
                out.push_out(d);
            }
        }
        Ok(out)
    }

    /// The text of what is placed at `pl` and below (for dependencies).
    fn placed_text(&self, pl: Pl, out: &mut String, depth: u32) {
        if depth > 64 {
            return;
        }
        for &(p, put) in &self.placed {
            if p != pl {
                continue;
            }
            match put {
                Put::Prop(sc, i) => {
                    cat_to!(out, self.scope(sc).props[i].text, "\n");
                }
                Put::Sub(sc) => self.scope_text(sc, out),
                Put::Group(sc) => {
                    for (v, var) in self.scope(sc).vars.iter().enumerate() {
                        if self.dead(sc, v) {
                            cat_to!(out, var.line, "\n");
                        }
                    }
                    self.placed_text(Pl::Group(sc), out, depth + 1);
                }
                Put::K(_) => {}
            }
        }
    }

    fn scope_text(&self, sc: Sc, out: &mut String) {
        for (k, s) in self.ir.factories[sc.0].scopes.iter().enumerate() {
            if k == sc.1 || s.parent == sc.1 && !self.is_merged((sc.0, k)) {
                if k != sc.1 {
                    self.scope_text((sc.0, k), out);
                    continue;
                }
                for l in &s.other {
                    cat_to!(out, l, "\n");
                }
                for v in &s.vars {
                    cat_to!(out, v.line, "\n");
                }
                for p in &s.props {
                    cat_to!(out, p.text, "\n");
                }
            }
        }
    }

    fn schedule(&mut self) -> R<Vec<Plan>> {
        let ir = self.ir;
        let mut providers: Vec<(Key, usize)> = Vec::new();
        for (i, it) in self.items.iter().enumerate() {
            match &it.kind {
                IK::Def(n) => providers.push_out((Key::Loc(n.clone()), i)),
                IK::Factory(f) => providers.push_out((Key::Loc(ir.factories[*f].name.clone()), i)),
                IK::Act(a) => providers.push_out((Key::Loc(ir.acts[*a].name.clone()), i)),
                IK::Decl(names) => {
                    for n in names {
                        providers.push_out((Key::Global(n.clone()), i));
                    }
                }
                IK::FnDecl | IK::Stmt => {}
            }
        }
        // An async generator's queued requests are made again on its continuation:
        // their promises come from the activation, not their placeholder definitions.
        let mut skip: Vec<usize> = Vec::new();
        for (a, act) in ir.acts.iter().enumerate() {
            let ai = self.act_item(a);
            for (_, _, p) in &act.queue {
                let Some(name) = p.strip_prefix("$dump.") else {
                    return refuse("a queued request's promise");
                };
                if let Some(d) = providers.iter().position(|x| x.0 == Key::Loc(name.into())) {
                    let di = providers[d].1;
                    if let IK::Def(_) = self.items[di].kind {
                        if !self.items[di].text.ends_with("= new Promise(() => {});") {
                            return refuse("a queued request's promise is not a placeholder");
                        }
                        skip.push_out(di);
                    }
                    providers[d].1 = ai;
                } else {
                    providers.push_out((Key::Loc(name.into()), ai));
                }
            }
        }
        // The dependencies of every item.
        for i in 0..self.items.len() {
            let kind = self.items[i].kind.clone();
            let (deps, start, slots) = match kind {
                IK::Act(a) => {
                    let act = &ir.acts[a];
                    let mut text = String::new();
                    for sl in &act.slots {
                        if sl.cell.is_none() {
                            cat_to!(&mut text, sl.value, "\n");
                        }
                    }
                    cat_to!(&mut text, act.this, "\n");
                    for &(_, sc, v) in &self.acts[a].cells {
                        let var = &self.scope(sc).vars[v];
                        if !var.tdz {
                            cat_to!(&mut text, var.init.as_deref().unwrap_or(""), "\n");
                        }
                    }
                    for b in 0..self.acts[a].blocks.len() {
                        self.placed_text(Pl::Block(a, b), &mut text, 0);
                    }
                    let keys = self.keys_of(&text);
                    let mut deps = self.dep_items(&keys, i, &providers)?;
                    // Its continuation's definition.
                    match self.acts[a].place {
                        Some(Pl::Fac(sc)) => {
                            let top = (sc.0, 0);
                            if sc != top && self.is_merged(top) {
                                let name = ir.factories[sc.0].name.clone();
                                let key = self.scope(sc).props.first().map(|p| p.key.clone());
                                if let Some(key) = key
                                    && let Some(c) = self.creator_of(&name, &key)?
                                {
                                    deps.push_out(self.act_item(c));
                                }
                            } else if let Some(p) = providers
                                .iter()
                                .find(|p| p.0 == Key::Loc(ir.factories[sc.0].name.clone()))
                            {
                                deps.push_out(p.1);
                            }
                        }
                        Some(pl) if pl != Pl::Top => {
                            let mut p = Some(pl);
                            while let Some(q) = p {
                                if let Pl::Block(b, _) = q {
                                    deps.push_out(self.act_item(b));
                                    break;
                                }
                                p = self.parent_of(q)?;
                            }
                        }
                        _ => {}
                    }
                    let start = match &act.awaiting {
                        Some(t) => {
                            let k = self.keys_of(t);
                            self.dep_items(&k, i, &providers)?
                        }
                        None => Vec::new(),
                    };
                    let mut slots = Vec::new();
                    for sl in &act.slots {
                        if sl.cell.is_none() {
                            let k = self.keys_of(&sl.value);
                            slots.push_out((sl.name.clone(), self.dep_items(&k, i, &providers)?));
                        }
                    }
                    (deps, start, slots)
                }
                IK::Factory(f) => {
                    if self.is_merged((f, 0)) {
                        (Vec::new(), Vec::new(), Vec::new())
                    } else {
                        let mut text = String::new();
                        self.scope_text((f, 0), &mut text);
                        let k = self.keys_of(&text);
                        (self.dep_items(&k, i, &providers)?, Vec::new(), Vec::new())
                    }
                }
                _ => {
                    let k = self.keys_of(&self.items[i].text.clone());
                    (self.dep_items(&k, i, &providers)?, Vec::new(), Vec::new())
                }
            };
            let it = &mut self.items[i];
            it.deps = deps;
            it.start_deps = start;
            it.slot_deps = slots;
        }
        // matchAll's iterator clones its regexp when made: the regexp's lastIndex (a
        // patch the dump writes later) must be set before.
        for i in 0..self.items.len() {
            let t = self.items[i].text.clone();
            if let IK::Def(_) = self.items[i].kind
                && let Some(at) = t.find("$dump.regexpIterator($dump.")
            {
                let (re, _) = ident_at(&t, at + "$dump.regexpIterator($dump.".len());
                let patch = cat!("$dump.", re, ".lastIndex = ");
                if let Some(p) = self
                    .items
                    .iter()
                    .position(|x| x.kind == IK::Stmt && x.text.starts_with(&patch))
                {
                    self.items[p].pure = true;
                    self.items[i].deps.push_out(p);
                }
            }
        }
        for i in 0..self.items.len() {
            let pure = match &self.items[i].kind {
                IK::Factory(_) | IK::Decl(_) | IK::FnDecl => true,
                IK::Def(_) => {
                    let t = &self.items[i].text;
                    ![
                        "Promise.all",
                        "Promise.any",
                        "Promise.race",
                        ".then",
                        ".next()",
                        "$djs.resume",
                        "(0, $dump.K",
                        "$dump.K",
                    ]
                    .iter()
                    .any(|p| t.contains(p))
                }
                _ => false,
            };
            self.items[i].pure |= pure;
        }
        // An activation whose dependencies need it (they need the closures its prefix
        // makes): those go into its callback, and the bindings that take them are
        // declared after the callback.
        for i in 0..self.items.len() {
            let IK::Act(a) = self.items[i].kind else {
                continue;
            };
            let deps = self.items[i].deps.clone();
            let cyc: Vec<usize> = deps.iter().copied().filter(|&d| self.needs(d, i)).collect();
            if cyc.is_empty() {
                continue;
            }
            self.items[i].deps.retain(|d| !cyc.contains(d));
            let slots = self.items[i].slot_deps.clone();
            for (k, ds) in slots {
                if ds.iter().any(|&d| cyc.contains(&d) || self.needs(d, i)) {
                    self.acts[a].after_go.push_out(k);
                }
            }
            self.acts[a].needs_go = true;
        }
        self.pull(&skip)
    }

    /// Whether item `from` needs item `target` (through dependencies).
    fn needs(&self, from: usize, target: usize) -> bool {
        let mut seen = alloc::vec![false; self.items.len()];
        let mut stack = alloc::vec![from];
        while let Some(x) = stack.pop() {
            if x == target {
                return true;
            }
            if x >= seen.len() || seen[x] {
                continue;
            }
            seen[x] = true;
            stack.extend(self.items[x].deps.iter().copied());
        }
        false
    }

    /// Pull scheduling: items in the dump's order, each after what it needs; an
    /// activation needed before its own place is called there, with a callback for
    /// what lies between.
    fn pull(&mut self, skip: &[usize]) -> R<Vec<Plan>> {
        let mut s = Sched {
            emitted: alloc::vec![false; self.items.len()],
            busy: alloc::vec![false; self.items.len()],
            stack: Vec::new(),
            out: alloc::vec![Vec::new()],
        };
        for &k in skip {
            s.emitted[k] = true;
        }
        for i in 0..self.items.len() {
            if let IK::Act(a) = self.items[i].kind {
                if let Some(pos) = s.stack.iter().position(|&x| x == i) {
                    if pos != s.stack.len() - 1 {
                        return refuse("crossing callbacks");
                    }
                    let sd = self.items[i].start_deps.clone();
                    self.ensure(&mut s, &sd, i)?;
                    s.stack.pop();
                    let cb = s.out.pop().unwrap_or_default();
                    // The call entry is the last of the enclosing list.
                    if let Some(Plan::Call(_, c)) = s.out.last_mut().and_then(|l| {
                        l.iter_mut()
                            .rev()
                            .find(|p| matches!(p, Plan::Call(x, Some(_)) if *x == a))
                    }) {
                        *c = Some(cb);
                    }
                    continue;
                }
                let deps = self.items[i].deps.clone();
                self.ensure(&mut s, &deps, i)?;
                if self.acts[a].needs_go && !s.emitted[i] {
                    return refuse("a frame needs data made after it");
                }
                let sd = self.items[i].start_deps.clone();
                self.ensure(&mut s, &sd, i)?;
                if s.emitted[i] {
                    continue;
                }
                s.emitted[i] = true;
                s.out
                    .last_mut()
                    .expect("a list")
                    .push_out(Plan::Call(a, None));
                continue;
            }
            let deps = self.items[i].deps.clone();
            self.ensure(&mut s, &deps, i)?;
            if !s.emitted[i] {
                s.emitted[i] = true;
                s.out.last_mut().expect("a list").push_out(Plan::Item(i));
            }
        }
        if !s.stack.is_empty() {
            return refuse("unclosed callback");
        }
        Ok(s.out.pop().unwrap_or_default())
    }

    fn ensure(&mut self, s: &mut Sched, deps: &[usize], upto: usize) -> R<()> {
        for &d in deps {
            if s.emitted[d] {
                continue;
            }
            if s.busy[d] {
                return refuse("cyclic dependencies");
            }
            if let IK::Act(a) = self.items[d].kind {
                // Called now, with a callback for what follows up to its place.
                s.busy[d] = true;
                let dd = self.items[d].deps.clone();
                self.ensure(s, &dd, upto)?;
                s.busy[d] = false;
                if s.emitted[d] {
                    continue;
                }
                s.emitted[d] = true;
                s.out
                    .last_mut()
                    .expect("a list")
                    .push_out(Plan::Call(a, Some(Vec::new())));
                s.stack.push_out(d);
                s.out.push_out(Vec::new());
                continue;
            }
            if d > upto && !self.items[d].pure {
                return refuse("reordering an effect");
            }
            s.busy[d] = true;
            let dd = self.items[d].deps.clone();
            self.ensure(s, &dd, upto)?;
            s.busy[d] = false;
            if !s.emitted[d] {
                s.emitted[d] = true;
                s.out.last_mut().expect("a list").push_out(Plan::Item(d));
            }
        }
        Ok(())
    }
}

struct Sched {
    emitted: Vec<bool>,
    busy: Vec<bool>,
    /// Open activations (items) whose callback is being filled.
    stack: Vec<usize>,
    /// The lists being filled: the top level, then each open callback.
    out: Vec<Vec<Plan>>,
}

// ---------------------------------------------------------------- emission

/// Helpers a plain dump defines on its `$dump` (D2830): plain functions, same names as
/// the format 2 loader primitives where those have a plain encoding.
const HELPERS: &[(&str, &str)] = &[
    ("R", "Reflect.apply"),
    ("G", "Reflect.get"),
    ("C", "Reflect.construct"),
    (
        "D",
        "(o, k, v) => Object.defineProperty(o, k, { value: v, writable: true, enumerable: true, configurable: true })",
    ),
    ("A", "Object.assign"),
    // An iterable over a saved iterator record whose first step is a placeholder: the
    // rest of the current iteration runs in it, the next ones take the iterator's steps.
    (
        "L",
        "(it, next) => { let first = true; const r = { next() { if (first) { first = false; return { value: undefined, done: false }; } return Reflect.apply(next ?? it.next, it, []); }, return(v) { const m = it.return; return m == null ? { done: true, value: v } : Reflect.apply(m, it, [v]); }, [Symbol.iterator]() { return r; } }; return r; }",
    ),
    (
        "N",
        "(it, next, p) => { let first = true; const r = { next() { if (first) { first = false; return p; } return Reflect.apply(next, it, []); }, return(v) { const m = it.return; return m == null ? { done: true, value: v } : Reflect.apply(m, it, [v]); }, [Symbol.asyncIterator]() { return r; } }; return r; }",
    ),
    (
        "Q",
        "(a, ...v) => { let n = a.length; for (const x of v) Object.defineProperty(a, n++, { value: x, writable: true, enumerable: true, configurable: true }); return a; }",
    ),
    (
        "regexpIterator",
        "(re, s, f) => { const it = $dump.MA.call(re, s); if (f & 4) while (!it.next().done); return it; }",
    ),
    ("MA", "RegExp.prototype[Symbol.matchAll]"),
    (
        "forIn",
        "(o, keys) => { let i = 0; const r = { next() { while (i < keys.length) { const k = keys[i++]; if (o == null || k in o) return { value: k, done: false }; } return { value: undefined, done: true }; }, [Symbol.iterator]() { return r; } }; return r; }",
    ),
    (
        "advance",
        "(it, n) => { const next = it.next; for (let i = 0; i < n; i++) Reflect.apply(next, it, []); return it; }",
    ),
    (
        "fn",
        "(o) => { const d = Object.getOwnPropertyDescriptor(o, Reflect.ownKeys(o)[0]); return 'value' in d ? d.value : d.get ?? d.set; }",
    ),
    (
        "spent",
        "(k) => { let f; new Promise((a, b) => { f = k ? b : a; a(); }); return f; }",
    ),
    (
        "rejected",
        "(e) => { const p = Promise.reject(e); p.then(undefined, () => {}); return p; }",
    ),
    ("bytes", "(b) => Uint8Array.fromBase64(b).buffer"),
    (
        "keep",
        "(o, keys) => { for (const k of Reflect.ownKeys(o)) if (!['length', 'name', 'prototype', 'constructor'].includes(k) && !keys.includes(k) && !delete o[k]) throw new TypeError('keep'); }",
    ),
];

/// Intrinsics without a global name, as plain expressions.
const INTRINSICS: &[(&str, &str)] = &[
    (
        "%ArrayIteratorPrototype%",
        "Object.getPrototypeOf([][Symbol.iterator]())",
    ),
    (
        "%MapIteratorPrototype%",
        "Object.getPrototypeOf(new Map()[Symbol.iterator]())",
    ),
    (
        "%SetIteratorPrototype%",
        "Object.getPrototypeOf(new Set()[Symbol.iterator]())",
    ),
    (
        "%StringIteratorPrototype%",
        "Object.getPrototypeOf(\"\"[Symbol.iterator]())",
    ),
    (
        "%RegExpStringIteratorPrototype%",
        "Object.getPrototypeOf(/(?:)/[Symbol.matchAll](\"\"))",
    ),
    (
        "%GeneratorFunction.prototype.prototype%",
        "Object.getPrototypeOf(function* () {}).prototype",
    ),
    (
        "%GeneratorPrototype%",
        "Object.getPrototypeOf(function* () {}).prototype",
    ),
    (
        "%AsyncGeneratorPrototype%",
        "Object.getPrototypeOf(async function* () {}).prototype",
    ),
    (
        "%IteratorPrototype%",
        "Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()))",
    ),
    (
        "%AsyncIteratorPrototype%",
        "Object.getPrototypeOf(Object.getPrototypeOf(async function* () {}).prototype)",
    ),
    (
        "%IteratorHelperPrototype%",
        "Object.getPrototypeOf([].values().map((x) => x))",
    ),
    (
        "%WrapForValidIteratorPrototype%",
        "Object.getPrototypeOf(Iterator.from({ next() {} }))",
    ),
    ("%TypedArray%", "Object.getPrototypeOf(Uint8Array)"),
];

/// A builtin's registry path (D2371) as a plain expression.
fn builtin_expr(path: &str) -> R<String> {
    let (getter, p) = match path.split_once(' ') {
        Some((g @ ("get" | "set"), rest)) => (Some(g), rest),
        _ => (None, path),
    };
    let mut parts: Vec<&str> = p.split('.').collect();
    let base = if parts[0].starts_with('%') {
        match INTRINSICS.iter().find(|x| x.0 == parts[0]) {
            Some(x) => String::from(x.1),
            None => return Err(cat!("plain dumps: the intrinsic ", parts[0])),
        }
    } else {
        String::from(parts[0])
    };
    if parts.len() == 1 {
        return Ok(base);
    }
    let last = parts.pop().unwrap_or_default();
    let mut obj = base;
    for q in &parts[1..] {
        cat_to!(&mut obj, ".", *q);
    }
    let key = match last.strip_prefix("@@") {
        Some(sym) => cat!("Symbol.", sym),
        None => quote(last),
    };
    Ok(match getter {
        Some(g) => cat!("Object.getOwnPropertyDescriptor(", obj, ", ", key, ").", g),
        None => cat!(obj, "[", key, "]"),
    })
}

impl W<'_> {
    fn emit(&mut self, plan: &[Plan], ts: bool) -> R<String> {
        let mut hoisted: Vec<String> = Vec::new();
        let body = self.emit_list(plan, 0, &mut hoisted)?;
        let ir = self.ir;
        let mut s = String::from(if ts {
            "\"djs-dump 3 typescript\";\n"
        } else {
            "\"djs-dump 3\";\n"
        });
        // djs runs all code strict (D015): so does node.
        s.push_str("\"use strict\";\n");
        cat_to!(&mut s, "$djs.seed(", ir.seed.0, ", ", ir.seed.1, ");\n");
        // The helpers the text uses.
        let mut text = join(&body, "\n");
        for it in &self.items {
            if it.kind == IK::FnDecl {
                cat_to!(&mut text, "\n", it.text);
            }
        }
        let used = refs(&text);
        let mut helpers: Vec<String> = Vec::new();
        for &(k, v) in HELPERS {
            let mut uses = used.dump.iter().any(|(n, _)| n == k);
            if k == "MA" {
                uses |= used.dump.iter().any(|(n, _)| n == "regexpIterator");
            }
            if uses {
                helpers.push_out(cat!(k, ": ", v));
            }
        }
        cat_to!(
            &mut s,
            "globalThis.$dump = { ",
            join(&helpers, ",\n  "),
            " };\n"
        );
        for (i, p) in ir.builtins.iter().enumerate() {
            cat_to!(&mut s, "$dump.b", i, " = ", builtin_expr(p)?, ";\n");
        }
        for (f, fac) in ir.factories.iter().enumerate() {
            if self.is_merged((f, 0)) {
                cat_to!(&mut s, "$dump.", fac.name, " = {};\n");
            }
        }
        for it in &self.items {
            if it.kind == IK::FnDecl {
                cat_to!(&mut s, it.text, "\n");
            }
        }
        for h in &hoisted {
            cat_to!(&mut s, h, "\n");
        }
        for l in &body {
            cat_to!(&mut s, l, "\n");
        }
        let handle = ir.handles.first().map_or("undefined", |h| h.1.as_str());
        cat_to!(
            &mut s,
            "try { ",
            handle,
            " } finally { delete globalThis.$dump; }\n"
        );
        Ok(s)
    }

    fn emit_list(
        &mut self,
        list: &[Plan],
        depth: u32,
        hoisted: &mut Vec<String>,
    ) -> R<Vec<String>> {
        let ir = self.ir;
        let mut lines = Vec::new();
        for p in list {
            match p {
                Plan::Call(a, cb) => {
                    if let Some(cb) = cb {
                        let name = cat!("$dump.g", ir.acts[*a].name);
                        self.acts[*a].go = cat!(name, "();");
                        let inner = self.emit_list(cb, depth + 1, hoisted)?;
                        lines.push_out(cat!(name, " = () => {\n", join(&inner, "\n"), "\n};"));
                    }
                    lines.extend(self.call_text(*a)?);
                }
                Plan::Item(i) => {
                    let kind = self.items[*i].kind.clone();
                    let text = self.items[*i].text.clone();
                    match &kind {
                        IK::FnDecl => {}
                        IK::Factory(f) => {
                            if !self.is_merged((*f, 0)) {
                                let t = self.factory_text((*f, 0))?;
                                lines.push_out(cat!(
                                    "$dump.",
                                    ir.factories[*f].name,
                                    " = ",
                                    t,
                                    ";"
                                ));
                            }
                        }
                        IK::Decl(names) if depth > 0 => {
                            let Some(rest) = text.strip_prefix("var ") else {
                                return refuse("a global lexical declaration inside a callback");
                            };
                            hoisted.push_out(cat!("var ", names[0], ";"));
                            lines.push_out(String::from(rest));
                        }
                        _ => lines.push_out(text),
                    }
                }
            }
        }
        Ok(lines)
    }

    fn kref(&self, a: usize) -> String {
        let act = &self.ir.acts[a];
        match self.acts[a].home {
            Some((sc, _)) => cat!("$dump.", self.ir.factories[sc.0].name, ".$K", act.name),
            None => cat!("$dump.K", act.name),
        }
    }

    /// The function a continuation stands for, as its origin record names it: the
    /// dump's reference, or its text.
    fn fref(&self, a: usize) -> String {
        let act = &self.ir.acts[a];
        let e = act.fexpr.as_str();
        let reference = e.split('.').all(|p| !p.is_empty() && is_ident(p));
        if reference {
            String::from(e)
        } else {
            quote(&act.ftext)
        }
    }

    fn call_text(&mut self, a: usize) -> R<Vec<String>> {
        let ir = self.ir;
        let act = &ir.acts[a];
        let k = self.ktext(a)?;
        let mut lines = Vec::new();
        for (name, r) in &self.acts[a].pre {
            let v = act
                .slots
                .iter()
                .find(|s| s.name == *name)
                .map_or("undefined", |s| s.value.as_str());
            lines.push_out(cat!(r, " = ", v, ";"));
        }
        let kref = self.kref(a);
        if self.acts[a].home.is_none() {
            lines.push_out(cat!(kref, " = ", k.text, ";"));
        }
        let segs: Vec<String> = k.segs.iter().map(|x| cat!(*x)).collect();
        lines.push_out(cat!(
            "Object.defineProperty(",
            kref,
            ", \"$djs\", { value: { f: ",
            self.fref(a),
            ", at: ",
            quote(&act.site.name),
            ", segs: [",
            join(&segs, ","),
            "], scaf: ",
            k.scaf,
            ", cmap: ",
            k.cmap,
            " } });"
        ));
        let me = cat!("$dump.", act.name);
        let generator = act.kind == crate::interp::run::suspend::GENERATOR
            || act.kind == crate::interp::run::suspend::ASYNC_GENERATOR;
        let at_yield = act.site.name.starts_with("yield#");
        if generator {
            if !self.fref(a).starts_with('"') {
                lines.push_out(cat!(kref, ".prototype = ", act.fexpr, ".prototype;"));
            }
            lines.push_out(cat!(me, " = ", kref, ".call(", act.this, ");"));
            if at_yield {
                lines.push_out(cat!(me, ".next();"));
            } else if act.kind != crate::interp::run::suspend::ASYNC_GENERATOR
                || act.queue.is_empty()
            {
                return refuse("a generator at an await without a request");
            }
            for (i, (t, v, p)) in act.queue.iter().enumerate() {
                if i == 0 && !at_yield {
                    lines.push_out(cat!(p, " = ", me, ".next();"));
                } else {
                    lines.push_out(cat!(p, " = ", me, ".", *t, "(", v, ");"));
                }
            }
        } else if self.is_arrow(a) {
            lines.push_out(cat!(me, " = (0, ", kref, ")();"));
        } else {
            lines.push_out(cat!(me, " = ", kref, ".call(", act.this, ");"));
        }
        Ok(lines)
    }

    fn is_arrow(&self, a: usize) -> bool {
        self.ir.acts[a].fkind == func_flags::KIND_ARROW
    }

    fn ktext(&mut self, a: usize) -> R<KText> {
        if let Some(k) = &self.acts[a].k {
            return Ok(k.clone());
        }
        let write_k = self.write_k;
        let act = &self.ir.acts[a];
        let k = {
            let mut c = C {
                w: self,
                a,
                dry: false,
            };
            write_k(act, &mut c)?
        };
        for sl in &act.slots {
            if sl.name.contains('@')
                && (sl.name.contains('.') || sl.name.contains('-'))
                && !k.used.contains(&sl.name)
            {
                return Err(cat!("plain dumps: unused slot ", sl.name));
            }
        }
        self.acts[a].k = Some(k.clone());
        Ok(k)
    }

    /// An unmerged scope's factory: its merged nested scopes left out (their code moved
    /// into continuations), the continuations of functions it holds added.
    fn factory_text(&mut self, sc: Sc) -> R<String> {
        let ir = self.ir;
        let fac = &ir.factories[sc.0];
        let scope = &fac.scopes[sc.1];
        let mut lines = alloc::vec![String::from("(() => {")];
        for l in &scope.other {
            lines.push_out(cat!("  ", l));
        }
        let mut props = Vec::new();
        for (k, child) in fac.scopes.iter().enumerate() {
            if child.parent == sc.1 && !self.is_merged((sc.0, k)) {
                let t = self.factory_text((sc.0, k))?;
                lines.push_out(cat!("const $s", k, " = ", t, ";"));
                props.push_out(cat!("...$s", k));
            }
        }
        for v in &scope.vars {
            if !v.tdz {
                lines.push_out(cat!("  ", v.line));
            }
        }
        for p in &scope.props {
            props.push_out(p.text.clone());
        }
        for a in 0..ir.acts.len() {
            if self.acts[a].place == Some(Pl::Fac(sc)) {
                let k = self.ktext(a)?;
                props.push_out(cat!("$K", ir.acts[a].name, ": ", k.text));
            }
        }
        lines.push_out(cat!("  return { ", join(&props, ", "), " };"));
        for v in &scope.vars {
            if v.tdz {
                lines.push_out(cat!("  let ", v.name, ";"));
            }
        }
        lines.push_out(String::from("})()"));
        Ok(join(&lines, "\n"))
    }

    /// What the dump writes at a place (a block of a continuation, a dead group).
    fn emit_place(&mut self, pl: Pl) -> R<Vec<String>> {
        let ir = self.ir;
        let mut lines = Vec::new();
        let mut assign: Vec<(String, Vec<String>)> = Vec::new();
        let flush = |assign: &mut Vec<(String, Vec<String>)>, lines: &mut Vec<String>| {
            for (t, ps) in assign.drain(..) {
                lines.push_out(cat!(
                    "Object.assign($dump.",
                    t,
                    ", { ",
                    join(&ps, ", "),
                    " });"
                ));
            }
        };
        let here: Vec<Put> = self
            .placed
            .iter()
            .filter(|x| x.0 == pl)
            .map(|x| x.1)
            .collect();
        for put in here {
            match put {
                Put::Prop(..) | Put::K(_) => {
                    let (top, text) = match put {
                        Put::Prop(sc2, i2) => (
                            ir.factories[sc2.0].name.clone(),
                            self.scope(sc2).props[i2].text.clone(),
                        ),
                        Put::K(a) => {
                            let Some((hsc, _)) = self.acts[a].home else {
                                return refuse("internal: a placed continuation without a home");
                            };
                            let k = self.ktext(a)?;
                            (
                                ir.factories[hsc.0].name.clone(),
                                cat!("$K", ir.acts[a].name, ": ", k.text),
                            )
                        }
                        _ => unreachable!(),
                    };
                    match assign.iter_mut().find(|x| x.0 == top) {
                        Some(x) => x.1.push_out(text),
                        None => assign.push_out((top, alloc::vec![text])),
                    }
                }
                Put::Sub(sc) => {
                    flush(&mut assign, &mut lines);
                    let t = self.factory_text(sc)?;
                    lines.push_out(cat!(
                        "Object.assign($dump.",
                        ir.factories[sc.0].name,
                        ", ",
                        t,
                        ");"
                    ));
                }
                Put::Group(sc) => {
                    flush(&mut assign, &mut lines);
                    let label = cat!("$djs_", ir.factories[sc.0].name, "_", sc.1);
                    let mut inner = Vec::new();
                    let mut tdz = Vec::new();
                    for (v, var) in self.scope(sc).vars.iter().enumerate() {
                        if !self.dead(sc, v) {
                            continue;
                        }
                        if var.tdz {
                            tdz.push_out(var.name.clone());
                        } else {
                            inner.push_out(var.line.clone());
                        }
                    }
                    inner.extend(self.emit_place(Pl::Group(sc))?);
                    if tdz.is_empty() {
                        lines.push_out(cat!("{\n", join(&inner, "\n"), "\n}"));
                    } else {
                        // Declared after a `break` out of the block: never initialized.
                        lines.push_out(cat!(
                            label,
                            ": {\n",
                            join(&inner, "\n"),
                            "\nbreak ",
                            label,
                            ";\nlet ",
                            join(&tdz, ", "),
                            ";\n}"
                        ));
                    }
                }
            }
        }
        flush(&mut assign, &mut lines);
        Ok(lines)
    }
}
