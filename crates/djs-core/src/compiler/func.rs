//! Per-function compiler state: the scope tree, bindings, name references, constants
//! (docs/compiler.md 2.1, 5.1, 5.2).

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::bytecode::{CodeBuf, Phase1};
use crate::heap::Atom;
use crate::value::Value;

pub(super) type ScopeId = u32;
pub(super) type VarId = u32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ScopeKind {
    /// The function body (Script: the global scope of the script).
    Body,
    Block,
    /// The scope of a `for (let ...)` head.
    For,
    /// A `switch` case block: jumps land in its middle, so TDZ checks are never elided.
    Switch,
}

pub(super) struct Scope {
    pub parent: Option<ScopeId>,
    pub kind: ScopeKind,
    pub vars: Vec<VarId>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum VarKind {
    Var,
    Let,
    Const,
    Param,
    /// The `this` pseudo-binding (docs/compiler.md 5.7).
    This,
}

impl VarKind {
    pub fn is_lexical(self) -> bool {
        matches!(self, VarKind::Let | VarKind::Const)
    }
}

pub(super) struct Var {
    pub name: Atom,
    pub kind: VarKind,
    /// Where the binding lives (for `var`: the body scope).
    pub scope: ScopeId,
    /// Source offset where initialization completes; accesses at or after it need no TDZ
    /// check (docs/compiler.md 5.6). `u32::MAX` until the declaration is complete.
    pub init_end: u32,
    /// Script top-level `var`/`let`/`const`: a global binding, not a local.
    pub global: bool,
    /// Parameter index for `Param`.
    pub param: u16,
}

/// A name reference in phase-1 code: the `name_*` op's immediate indexes this table.
#[derive(Clone, Copy, Debug)]
pub(super) struct NameRef {
    pub atom: Atom,
    /// The innermost scope at the reference.
    pub scope: ScopeId,
    /// Source offset of the identifier.
    pub pos: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum FuncKind {
    Script,
    AsyncBody,
}

pub(super) struct FuncState {
    pub kind: FuncKind,
    pub code: CodeBuf<Phase1>,
    pub scopes: Vec<Scope>,
    pub vars: Vec<Var>,
    pub refs: Vec<NameRef>,
    pub consts: Vec<Value>,
    pub str_consts: BTreeMap<Vec<u16>, u32>,
    pub num_consts: BTreeMap<u64, u32>,
    /// `(phase-1 code position, source offset)`, in increasing code position.
    pub lines: Vec<(u32, u32)>,
    /// `var` declarations as `(name, scope where the declaration appeared)`, for
    /// var/let conflicts.
    pub var_decls: Vec<(Atom, ScopeId)>,
    pub this_var: Option<VarId>,
    pub param_count: u16,
}

pub(super) const BODY_SCOPE: ScopeId = 0;

impl FuncState {
    pub fn new(kind: FuncKind) -> Self {
        FuncState {
            kind,
            code: CodeBuf::phase1(),
            scopes: alloc::vec![Scope {
                parent: None,
                kind: ScopeKind::Body,
                vars: Vec::new(),
            }],
            vars: Vec::new(),
            refs: Vec::new(),
            consts: Vec::new(),
            str_consts: BTreeMap::new(),
            num_consts: BTreeMap::new(),
            lines: Vec::new(),
            var_decls: Vec::new(),
            this_var: None,
            param_count: 0,
        }
    }

    pub fn new_scope(&mut self, parent: ScopeId, kind: ScopeKind) -> ScopeId {
        self.scopes.push(Scope {
            parent: Some(parent),
            kind,
            vars: Vec::new(),
        });
        (self.scopes.len() - 1) as ScopeId
    }

    pub fn add_var(&mut self, name: Atom, kind: VarKind, scope: ScopeId) -> VarId {
        let id = self.vars.len() as VarId;
        let immediate = !kind.is_lexical();
        self.vars.push(Var {
            name,
            kind,
            scope,
            init_end: if immediate { 0 } else { u32::MAX },
            global: false,
            param: 0,
        });
        self.scopes[scope as usize].vars.push(id);
        id
    }

    /// The binding named `name` declared directly in `scope`.
    pub fn find_in_scope(&self, scope: ScopeId, name: Atom) -> Option<VarId> {
        self.scopes[scope as usize]
            .vars
            .iter()
            .copied()
            .find(|&v| self.vars[v as usize].name == name)
    }

    /// Resolves `name` from `scope` outward within this function.
    pub fn lookup(&self, mut scope: ScopeId, name: Atom) -> Option<VarId> {
        loop {
            if let Some(v) = self.find_in_scope(scope, name) {
                return Some(v);
            }
            scope = self.scopes[scope as usize].parent?;
        }
    }

    /// `inner` is `outer` or nested in it.
    pub fn scope_within(&self, mut inner: ScopeId, outer: ScopeId) -> bool {
        loop {
            if inner == outer {
                return true;
            }
            match self.scopes[inner as usize].parent {
                Some(p) => inner = p,
                None => return false,
            }
        }
    }
}
