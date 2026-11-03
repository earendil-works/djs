//! Suspension maps (D2563, docs/continuation-dumps.md "Suspension maps"): for every
//! `await` and `yield` of a function, what a frame suspended there holds, described by
//! source names and roles rather than by this build's slots and pcs, so that a later
//! build can rebuild the frame from a dump and an exporter can name what it finds.
//!
//! Computed on demand by compiling again with recording on
//! ([`suspension_maps`]); nothing is stored in templates and a normal compile records
//! nothing. Without the `dump` feature the parser's hooks compile to nothing.
//!
//! The operand roles come from the constructs that leave values on the operand stack
//! while a nested expression is parsed (a call's callee and `this`, a binary operator's
//! left operand, a loop's iterator record): each pushes the roles of the values it keeps
//! when it emits them and pops them when it consumes them. At a site the role stack is
//! the operand stack below the awaited value. Resolution checks its length against the
//! stack depth of the final code: a mismatch, or a construct that marked its values as
//! not described yet ([`Role::Unsupported`]), makes the site unsupported instead of
//! mislabeled.

use core::fmt;

/// The loop kinds whose iterator records stay on the operand stack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Loop {
    /// `for (… of …)`.
    Of,
    /// `for (… in …)`.
    In,
    /// `for await (… of …)`.
    Await,
    /// `yield* v`: the delegated iterator and its `next`, kept for the whole delegation.
    Star,
}

impl Loop {
    pub fn name(self) -> &'static str {
        match self {
            Loop::Of => "for-of",
            Loop::In => "for-in",
            Loop::Await => "for-await",
            Loop::Star => "yield*",
        }
    }
}

/// What an operand below the awaited value is, named by the construct that keeps it
/// (docs/portable-snapshots.md 3.5, extended where djs's lowering keeps more values).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// `f(…)`, `o.m(…)`: the function, `this` (undefined for a plain call), the
    /// arguments evaluated so far.
    CallCallee,
    CallThis,
    CallArg(u16),
    /// The argument array of a call with a spread, so far.
    CallArgs,
    /// `new X(…)`: the constructor and the arguments so far.
    NewCallee,
    NewArg(u16),
    NewArgs,
    /// `super(…)`: the parent constructor and the arguments so far.
    SuperCallee,
    SuperArg(u16),
    SuperArgs,
    /// `o[k]`: the object while the key is evaluated.
    MemberObject,
    /// `super[k]`: `this` and the home object's prototype while the key is evaluated.
    SuperThis,
    SuperBase,
    /// An object literal under construction, and its computed key while the value is
    /// evaluated.
    ObjectLiteral,
    ObjectKey,
    /// An array literal's leading elements, kept on the stack for one `array_from`;
    /// after a hole, a spread or 256 elements the array under construction.
    ArrayElement(u16),
    ArrayLiteral,
    /// `a + b`, `a < b`, `a in b`, …: the left operand while the right one is evaluated.
    BinaryLeft,
    /// `o.p = v`, `o[k] = v`, `o.#x = v` (and compound forms): the object, the key (a
    /// private name's symbol), the old value of a compound assignment.
    AssignObject,
    AssignKey,
    AssignOld,
    /// `[a, b] = v`: the value (the expression's result) while the pattern runs.
    AssignValue,
    /// The parts of a template literal so far (strings, joined at the end).
    TemplatePart(u16),
    /// `import(s, o)`: the specifier while the options are evaluated.
    ImportSpecifier,
    /// `%Name(…)` in privileged code: the arguments so far.
    IntrinsicArg(u16),
    /// A loop's iterator record: the iterator (for-in: the key iterator) and, for for-of
    /// and for-await, its `next` method.
    LoopIterator(Loop),
    LoopNext(Loop),
    /// A destructuring pattern under way: an object pattern's value, the keys a rest
    /// property excludes so far, a computed key while its target and default are
    /// evaluated; an array pattern's iterator (`undefined` once done) and its `next`.
    PatternSource,
    PatternExcluded,
    PatternKey,
    PatternNext,
    PatternIterator,
    /// A statement list's `using` declarations: the resource Record its disposal reads,
    /// below its statements.
    UsingRecord,
    /// `switch (d) { case t: … }`: the discriminant, twice (kept and compared) while a
    /// case test is evaluated.
    SwitchDiscriminant,
    /// The exception a `for await` rethrows after closing its iterator (the loop's body
    /// threw).
    Exception,
    /// A subroutine entered with `gosub` (a finally block, a `using` disposal, a loop's
    /// iterator close): the value it carries (`undefined`, the exception or the return
    /// value) and the return address, an int pc of this build. A dump writes the
    /// return address as the index of its `gosub` among those to the subroutine, in
    /// emission order (`Site::returns`). The subroutine is the function's `i`-th
    /// (recorder order), which is not part of the role's name.
    FinallyValue,
    FinallyReturn(u16),
    /// Values of a construct that is not described yet (class definitions): the site is unsupported.
    Unsupported(&'static str),
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Role::CallCallee => f.write_str("call.callee"),
            Role::CallThis => f.write_str("call.this"),
            Role::CallArg(i) => write!(f, "call.arg[{i}]"),
            Role::CallArgs => f.write_str("call.args"),
            Role::NewCallee => f.write_str("new.callee"),
            Role::NewArg(i) => write!(f, "new.arg[{i}]"),
            Role::NewArgs => f.write_str("new.args"),
            Role::SuperCallee => f.write_str("super.callee"),
            Role::SuperArg(i) => write!(f, "super.arg[{i}]"),
            Role::SuperArgs => f.write_str("super.args"),
            Role::MemberObject => f.write_str("member.object"),
            Role::SuperThis => f.write_str("super.this"),
            Role::SuperBase => f.write_str("super.base"),
            Role::ObjectLiteral => f.write_str("object.literal"),
            Role::ObjectKey => f.write_str("object.key"),
            Role::ArrayElement(i) => write!(f, "array.element[{i}]"),
            Role::ArrayLiteral => f.write_str("array.literal"),
            Role::BinaryLeft => f.write_str("binary.left"),
            Role::AssignObject => f.write_str("assign.object"),
            Role::AssignKey => f.write_str("assign.key"),
            Role::AssignOld => f.write_str("assign.old"),
            Role::AssignValue => f.write_str("assign.value"),
            Role::TemplatePart(i) => write!(f, "template.part[{i}]"),
            Role::ImportSpecifier => f.write_str("import.specifier"),
            Role::IntrinsicArg(i) => write!(f, "intrinsic.arg[{i}]"),
            Role::LoopIterator(l) => write!(f, "{}.iterator", l.name()),
            Role::LoopNext(l) => write!(f, "{}.next", l.name()),
            Role::SwitchDiscriminant => f.write_str("switch.discriminant"),
            Role::UsingRecord => f.write_str("using.record"),
            Role::PatternSource => f.write_str("pattern.source"),
            Role::PatternExcluded => f.write_str("pattern.excluded"),
            Role::PatternKey => f.write_str("pattern.key"),
            Role::PatternNext => f.write_str("pattern.next"),
            Role::PatternIterator => f.write_str("pattern.iterator"),
            Role::Exception => f.write_str("exception"),
            Role::FinallyValue => f.write_str("finally.value"),
            Role::FinallyReturn(_) => f.write_str("finally.return"),
            Role::Unsupported(c) => write!(f, "unsupported:{c}"),
        }
    }
}

#[cfg(feature = "dump")]
impl Role {
    /// The role's name and its index, as `Display` writes them: `("call.arg", Some(2))`
    /// for `call.arg[2]`, `("unsupported", None)` for any unsupported construct.
    pub fn parts(self) -> (&'static str, Option<u16>) {
        match self {
            Role::CallCallee => ("call.callee", None),
            Role::CallThis => ("call.this", None),
            Role::CallArg(i) => ("call.arg", Some(i)),
            Role::CallArgs => ("call.args", None),
            Role::NewCallee => ("new.callee", None),
            Role::NewArg(i) => ("new.arg", Some(i)),
            Role::NewArgs => ("new.args", None),
            Role::SuperCallee => ("super.callee", None),
            Role::SuperArg(i) => ("super.arg", Some(i)),
            Role::SuperArgs => ("super.args", None),
            Role::MemberObject => ("member.object", None),
            Role::SuperThis => ("super.this", None),
            Role::SuperBase => ("super.base", None),
            Role::ObjectLiteral => ("object.literal", None),
            Role::ObjectKey => ("object.key", None),
            Role::ArrayElement(i) => ("array.element", Some(i)),
            Role::ArrayLiteral => ("array.literal", None),
            Role::BinaryLeft => ("binary.left", None),
            Role::AssignObject => ("assign.object", None),
            Role::AssignKey => ("assign.key", None),
            Role::AssignOld => ("assign.old", None),
            Role::AssignValue => ("assign.value", None),
            Role::TemplatePart(i) => ("template.part", Some(i)),
            Role::ImportSpecifier => ("import.specifier", None),
            Role::IntrinsicArg(i) => ("intrinsic.arg", Some(i)),
            Role::LoopIterator(Loop::Of) => ("for-of.iterator", None),
            Role::LoopIterator(Loop::In) => ("for-in.iterator", None),
            Role::LoopIterator(Loop::Await) => ("for-await.iterator", None),
            Role::LoopIterator(Loop::Star) => ("yield*.iterator", None),
            Role::LoopNext(Loop::Of) => ("for-of.next", None),
            Role::LoopNext(Loop::In) => ("for-in.next", None),
            Role::LoopNext(Loop::Await) => ("for-await.next", None),
            Role::LoopNext(Loop::Star) => ("yield*.next", None),
            Role::SwitchDiscriminant => ("switch.discriminant", None),
            Role::UsingRecord => ("using.record", None),
            Role::PatternSource => ("pattern.source", None),
            Role::PatternExcluded => ("pattern.excluded", None),
            Role::PatternKey => ("pattern.key", None),
            Role::PatternNext => ("pattern.next", None),
            Role::PatternIterator => ("pattern.iterator", None),
            Role::Exception => ("exception", None),
            Role::FinallyValue => ("finally.value", None),
            Role::FinallyReturn(_) => ("finally.return", None),
            Role::Unsupported(_) => ("unsupported", None),
        }
    }
}

/// What kind of suspension point a site is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SiteKind {
    /// `await x`.
    Await,
    /// `yield x` of a generator.
    Yield,
    /// An async generator's `yield x`: the await of `x`, the yield itself, and the await
    /// of the value a `return(v)` resumes it with.
    AsyncYieldAwait,
    AsyncYield,
    AsyncYieldReturn,
    /// An async generator's `return x`: the await of `x`.
    AsyncReturn,
    /// `for await`: the await of each `next()` result, and of `return()` when the loop
    /// is left early.
    ForAwaitNext,
    ForAwaitClose,
    /// `await using`: the await of a disposal.
    AwaitUsing,
    /// `yield*`: its yields (a sync generator passes the inner result through, an async
    /// one yields its value) and, in async generators, the awaits of the inner results
    /// and of a `return(v)` value.
    YieldStar,
    YieldStarAwait,
}

impl SiteKind {
    /// The name's prefix: `await#1`, `yield#2`, `for-await.next#1`, …
    #[cfg_attr(not(feature = "dump"), allow(dead_code))]
    pub fn name(self) -> &'static str {
        match self {
            SiteKind::Await => "await",
            SiteKind::Yield => "yield",
            SiteKind::AsyncYieldAwait => "yield.await",
            SiteKind::AsyncYield => "yield",
            SiteKind::AsyncYieldReturn => "yield.return",
            SiteKind::AsyncReturn => "return.await",
            SiteKind::ForAwaitNext => "for-await.next",
            SiteKind::ForAwaitClose => "for-await.close",
            SiteKind::AwaitUsing => "await-using",
            SiteKind::YieldStar => "yield*",
            SiteKind::YieldStarAwait => "yield*.await",
        }
    }
}

#[cfg(feature = "dump")]
pub use maps::*;

#[cfg(feature = "dump")]
mod maps {
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::fmt::Write;

    use super::super::func::{FuncState, ScopeId, VarKind};
    use super::{Role, SiteKind};
    use crate::heap::Heap;

    /// The parser's record of one function: the role stack and the sites so far.
    #[derive(Default)]
    pub(in crate::compiler) struct Recorder {
        pub roles: Vec<Role>,
        pub sites: Vec<SiteRec>,
        /// The computed property keys this function evaluates (`[` to `]`).
        pub computed_keys: Vec<(u32, u32)>,
        /// The entry labels of its subroutines (`Role::FinallyReturn`'s index).
        pub subs: Vec<crate::bytecode::Label>,
    }

    /// A site as the parser saw it.
    pub(in crate::compiler) struct SiteRec {
        pub kind: SiteKind,
        /// Source offset of the construct (the `await`/`yield` keyword, the `for`).
        pub pos: u32,
        /// Phase-1 index of the `await`/`yield` instruction (kept up to date by loop
        /// rotation).
        pub p1: u32,
        /// The innermost scope at the site.
        pub scope: ScopeId,
        /// The role stack at the site: the operands below the awaited value.
        pub roles: Vec<Role>,
    }

    /// What a frame slot holds at a site.
    #[derive(Clone, PartialEq, Eq, Debug)]
    pub enum Slot {
        /// A binding: its name and the source offset (from the function's start) of
        /// the scope that declares it.
        Binding { name: String, scope: u32 },
        /// A compiler-made local by its stable name (`<state>`, `this`, `<home>`, …).
        Hidden(String),
        /// An argument slot that no binding lives in (the call's argument `i`).
        Argument(u16),
        /// An operand, by role.
        Operand(Role),
        /// Nothing at this site (a binding whose scope does not enclose it).
        Nothing,
    }

    /// One suspension point.
    #[derive(Clone, Debug)]
    pub struct Site {
        /// `await#1`, `yield.return#2`, …: the kind's prefix and the site's number among
        /// those of its kind in the function, in source order (from 1).
        pub name: String,
        pub kind: SiteKind,
        /// Source offset of the construct, from the function's start.
        pub pos: u32,
        /// Where execution resumes in this build's code (after the suspending
        /// instruction).
        pub resume_pc: u32,
        /// The argument slots (padded to the parameter count).
        pub args: Vec<Slot>,
        /// The local slots.
        pub locals: Vec<Slot>,
        /// Which local slots hold a Box at this site (a binding a closure captures): the
        /// loader puts the value in a Box there.
        pub boxed: Vec<bool>,
        /// The operands below the awaited value, bottom first.
        pub operands: Vec<Role>,
        /// For each `Role::FinallyReturn` operand, bottom first: the return addresses
        /// (pcs after each `gosub` to its subroutine, in emission order) a dump names by
        /// index.
        pub returns: Vec<Vec<u32>>,
        /// Why the frame cannot be described here: the construct, or `None`.
        pub unsupported: Option<String>,
    }

    /// The sites of one function.
    #[derive(Clone, Debug)]
    pub struct FunctionMap {
        /// The function's name (`<anonymous>` for none).
        pub name: String,
        /// Its source span in the compiled source (`Function.prototype.toString`'s).
        pub start: u32,
        pub end: u32,
        pub sites: Vec<Site>,
        /// The source spans (`[` to `]`) of the computed property keys its code
        /// evaluates: of its object literals, of the classes it defines, of its
        /// destructuring patterns. A dump that writes a class or method by its text
        /// evaluates such a key again (D2623).
        pub computed_keys: Vec<(u32, u32)>,
    }

    impl FunctionMap {
        /// The map as text, for the pin test and for reading: a header with every slot
        /// that holds something at some site, then one line per site with its operands
        /// and the slots that hold nothing there (`out`).
        pub fn text(&self, src: &[u8]) -> String {
            let mut out = String::new();
            let _ = writeln!(out, "function {} @{}..{}", self.name, self.start, self.end);
            // A slot holds the same thing wherever it holds something.
            let table = |pick: fn(&Site) -> &Vec<Slot>| -> Vec<Slot> {
                let n = self.sites.iter().map(|s| pick(s).len()).max().unwrap_or(0);
                (0..n)
                    .map(|i| {
                        self.sites
                            .iter()
                            .find_map(|s| pick(s).get(i).filter(|x| **x != Slot::Nothing))
                            .cloned()
                            .unwrap_or(Slot::Nothing)
                    })
                    .collect()
            };
            let (args, locals) = (table(|s| &s.args), table(|s| &s.locals));
            for (kind, slots) in [("arg", &args), ("loc", &locals)] {
                let line: Vec<String> = slots
                    .iter()
                    .enumerate()
                    .filter_map(|(i, slot)| {
                        let text = match slot {
                            Slot::Binding { name, scope } => alloc::format!("{name}@{scope}"),
                            Slot::Hidden(n) => n.clone(),
                            Slot::Argument(a) => alloc::format!("argument {a}"),
                            Slot::Operand(r) => alloc::format!("{r}"),
                            Slot::Nothing => return None,
                        };
                        Some(alloc::format!("{kind}{i} {text}"))
                    })
                    .collect();
                if !line.is_empty() {
                    let _ = writeln!(out, "  {}", line.join(", "));
                }
            }
            for s in &self.sites {
                let line = line_of(src, self.start, s.pos);
                let _ = write!(out, "  {} @{} ({line}) pc {}:", s.name, s.pos, s.resume_pc);
                match &s.unsupported {
                    Some(u) => {
                        let _ = write!(out, " unsupported {u}");
                    }
                    None if s.operands.is_empty() => out.push_str(" -"),
                    None => {
                        for r in &s.operands {
                            let _ = write!(out, " {r}");
                        }
                    }
                }
                let mut dead = Vec::new();
                for (kind, slots, all) in [("arg", &s.args, &args), ("loc", &s.locals, &locals)] {
                    for (i, slot) in slots.iter().enumerate() {
                        if *slot == Slot::Nothing && all.get(i).is_some_and(|x| *x != Slot::Nothing)
                        {
                            dead.push(alloc::format!("{kind}{i}"));
                        }
                    }
                }
                if !dead.is_empty() {
                    let _ = write!(out, " (out: {})", dead.join(" "));
                }
                out.push('\n');
            }
            out
        }
    }

    /// `line:col` of `pos` (from the function's start `base`) in `src`, 1-based.
    fn line_of(src: &[u8], base: u32, pos: u32) -> String {
        let at = ((base + pos) as usize).min(src.len());
        let line = src[..at].iter().filter(|&&b| b == b'\n').count() + 1;
        let col = at
            - src[..at]
                .iter()
                .rposition(|&b| b == b'\n')
                .map_or(0, |p| p + 1)
            + 1;
        alloc::format!("{line}:{col}")
    }

    /// Builds the map of function `f` from the parser's record and the final code.
    /// `slots[v]` is where binding `v` lives: `Some((false, i, _))` argument `i`,
    /// `Some((true, i, boxed))` local `i`, holding a Box if `boxed`; `n_locals` local slots; `params` argument slots;
    /// `at[k]` the final pc of site `k`'s instruction (`None`: not emitted, dead code);
    /// `depth[k]` the operand depth there, the suspended value included; `subs[i]` the
    /// return addresses of subroutine `i` (`Recorder::subs`).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::compiler) fn build(
        heap: &Heap,
        f: &FuncState,
        rec: &Recorder,
        slots: &[Option<(bool, u16, bool)>],
        n_locals: usize,
        params: usize,
        at: &[Option<(u32, u32)>],
        depth: &[Option<u32>],
        subs: &[Vec<u32>],
    ) -> FunctionMap {
        let base = f.src_start;
        let scope_pos = |s: ScopeId| f.scopes[s as usize].pos.saturating_sub(base);
        // Site numbers per kind, in source order (stable sort: emission order breaks
        // ties, as for the three sites of an async generator's `yield`).
        // (Sites in dead code are not emitted: they get no number.)
        let mut order: Vec<usize> = (0..rec.sites.len()).filter(|&k| at[k].is_some()).collect();
        crate::sort::sort_by(&mut order, |&a, &b| rec.sites[a].pos < rec.sites[b].pos);
        let mut number = alloc::vec![0u32; rec.sites.len()];
        let mut counts: Vec<(&'static str, u32)> = Vec::new();
        for &k in &order {
            let prefix = rec.sites[k].kind.name();
            let n = match counts.iter_mut().find(|c| c.0 == prefix) {
                Some(c) => {
                    c.1 += 1;
                    c.1
                }
                None => {
                    counts.push((prefix, 1));
                    1
                }
            };
            number[k] = n;
        }
        let mut sites = Vec::new();
        for &k in &order {
            let s = &rec.sites[k];
            let Some((_, resume)) = at[k] else { continue };
            // The scopes enclosing the site.
            let mut chain = Vec::new();
            let mut sc = Some(s.scope);
            while let Some(id) = sc {
                chain.push(id);
                sc = f.scopes[id as usize].parent;
            }
            let mut args = alloc::vec![None; params];
            let mut locals = alloc::vec![Slot::Nothing; n_locals];
            let mut boxed = alloc::vec![false; n_locals];
            for (i, a) in args.iter_mut().enumerate() {
                *a = Some(Slot::Argument(i as u16));
            }
            for (v, var) in f.vars.iter().enumerate() {
                let Some((local, i, in_box)) = slots[v] else {
                    continue;
                };
                let name = heap.atom_name(var.name);
                let slot = match var.kind {
                    VarKind::This
                    | VarKind::Arguments
                    | VarKind::NewTarget
                    | VarKind::Internal
                    | VarKind::HomeObject
                    | VarKind::ActiveFunc
                    | VarKind::Hidden
                    | VarKind::PrivateName
                    | VarKind::PrivateMethod
                    | VarKind::PrivateAccessor => Slot::Hidden(name),
                    _ if var.kind == VarKind::FuncName || chain.contains(&var.scope) => {
                        Slot::Binding {
                            name,
                            scope: scope_pos(var.scope),
                        }
                    }
                    _ => Slot::Nothing,
                };
                if local {
                    // A slot shared by bindings of disjoint scopes holds the one in
                    // scope here.
                    if slot != Slot::Nothing || locals[i as usize] == Slot::Nothing {
                        boxed[i as usize] = in_box && slot != Slot::Nothing;
                        locals[i as usize] = slot;
                    }
                } else if let Some(a) = args.get_mut(i as usize) {
                    *a = Some(slot);
                }
            }
            let unsupported = match s.roles.iter().find_map(|r| match r {
                Role::Unsupported(c) => Some(*c),
                _ => None,
            }) {
                Some(c) => Some(String::from(c)),
                None => match depth[k] {
                    Some(d) if d as usize == s.roles.len() + 1 => None,
                    Some(d) => Some(alloc::format!(
                        "{} operands, {} described",
                        d.saturating_sub(1),
                        s.roles.len()
                    )),
                    None => Some(String::from("unknown operand depth")),
                },
            };
            sites.push(Site {
                name: alloc::format!("{}#{}", s.kind.name(), number[k]),
                kind: s.kind,
                pos: s.pos.saturating_sub(base),
                resume_pc: resume,
                args: args
                    .into_iter()
                    .map(|a| a.unwrap_or(Slot::Nothing))
                    .collect(),
                locals,
                boxed,
                returns: match unsupported {
                    Some(_) => Vec::new(),
                    None => s
                        .roles
                        .iter()
                        .filter_map(|r| match r {
                            Role::FinallyReturn(i) => {
                                Some(subs.get(*i as usize).cloned().unwrap_or_default())
                            }
                            _ => None,
                        })
                        .collect(),
                },
                operands: if unsupported.is_some() {
                    Vec::new()
                } else {
                    s.roles.clone()
                },
                unsupported,
            });
        }
        let name = if f.name.is_null() {
            String::from("<anonymous>")
        } else {
            heap.atom_name(f.name)
        };
        FunctionMap {
            name,
            start: f.src_start,
            end: f.src_end,
            sites,
            computed_keys: rec.computed_keys.clone(),
        }
    }
}

// ---- The parser's hooks ----------------------------------------------------------------

#[cfg(feature = "dump")]
impl super::parser::Parser<'_, '_> {
    /// The construct keeps values with roles `rs` on the stack (pushed in this order).
    #[inline]
    pub(super) fn roles_push(&mut self, rs: &[Role]) {
        if let Some(r) = &mut self.f.smap {
            r.roles.extend_from_slice(rs);
        }
    }

    /// The construct consumed the top `n` values it kept.
    #[inline]
    pub(super) fn roles_pop(&mut self, n: usize) {
        if let Some(r) = &mut self.f.smap {
            let len = r.roles.len().saturating_sub(n);
            r.roles.truncate(len);
        }
    }

    /// Takes the top `n` roles off (values the code consumed for a while), for
    /// `roles_push` to put back.
    #[inline]
    pub(super) fn roles_take(&mut self, n: usize) -> alloc::vec::Vec<Role> {
        match &mut self.f.smap {
            Some(r) => {
                let at = r.roles.len().saturating_sub(n);
                r.roles.split_off(at)
            }
            None => alloc::vec::Vec::new(),
        }
    }

    /// Replaces the top `n` roles by `rs` (an `array_from` collapsing its elements).
    #[inline]
    pub(super) fn roles_replace(&mut self, n: usize, rs: &[Role]) {
        self.roles_pop(n);
        self.roles_push(rs);
    }

    /// A subroutine starts at `l` (placed next): its index for `Role::FinallyReturn`.
    #[inline]
    pub(super) fn subroutine(&mut self, l: crate::bytecode::Label) -> u16 {
        match &mut self.f.smap {
            Some(r) => {
                r.subs.push(l);
                (r.subs.len() - 1) as u16
            }
            None => 0,
        }
    }

    /// A suspension point: the `await`/`yield` instruction is emitted next.
    #[inline]
    pub(super) fn site(&mut self, kind: SiteKind, pos: u32) {
        let p1 = self.f.code.pos();
        let scope = self.scope;
        if let Some(r) = &mut self.f.smap {
            let roles = r.roles.clone();
            r.sites.push(maps::SiteRec {
                kind,
                pos,
                p1,
                scope,
                roles,
            });
        }
    }
}

#[cfg(not(feature = "dump"))]
impl super::parser::Parser<'_, '_> {
    #[inline(always)]
    pub(super) fn roles_push(&mut self, _rs: &[Role]) {}
    #[inline(always)]
    pub(super) fn roles_pop(&mut self, _n: usize) {}
    #[inline(always)]
    pub(super) fn roles_replace(&mut self, _n: usize, _rs: &[Role]) {}
    #[inline(always)]
    pub(super) fn roles_take(&mut self, _n: usize) -> alloc::vec::Vec<Role> {
        alloc::vec::Vec::new()
    }
    #[inline(always)]
    pub(super) fn subroutine(&mut self, _l: crate::bytecode::Label) -> u16 {
        0
    }
    #[inline(always)]
    pub(super) fn site(&mut self, _kind: SiteKind, _pos: u32) {}
}
