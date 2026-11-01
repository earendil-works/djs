//! Per-function compiler state: the scope tree, bindings, name references, captures,
//! constants (docs/compiler.md 2.1, 5.1 to 5.4).

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::bytecode::{CodeBuf, Phase1};
use crate::heap::Atom;
use crate::value::Value;

pub(super) type ScopeId = u32;
pub(super) type VarId = u32;
pub(super) type FuncId = u32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ScopeKind {
    /// The function scope: parameters, `var`s, body-level declarations (Script: the
    /// global scope of the script).
    Body,
    /// The scope that binds a named function expression's own name, outside the body.
    FuncName,
    Block,
    /// The scope of a `for (let ...)` head.
    For,
    /// A `switch` case block: jumps land in its middle, so TDZ checks are never elided.
    Switch,
    /// A `catch` clause: the parameter and the declarations of the catch block.
    Catch,
}

pub(super) struct Scope {
    pub parent: Option<ScopeId>,
    pub kind: ScopeKind,
    pub vars: Vec<VarId>,
    /// Function declarations hoisted to this scope's entry: `(binding, child index)`.
    pub funcs: Vec<(VarId, u32)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum VarKind {
    Var,
    Let,
    Const,
    /// A function declaration: initialized at its scope's entry (hoisted).
    Func,
    /// A parameter. Simple parameter lists keep them in argument slots; otherwise they
    /// are locals initialized in order, with TDZ (see `Var::init_end`).
    Param,
    /// The `this` pseudo-binding (docs/compiler.md 5.7).
    This,
    /// `arguments` (non-arrow functions).
    Arguments,
    /// `new.target`.
    NewTarget,
    /// A named function expression's own name (immutable).
    FuncName,
    /// A `catch (e)` parameter, initialized by the handler.
    CatchParam,
}

impl VarKind {
    pub fn is_lexical(self) -> bool {
        matches!(self, VarKind::Let | VarKind::Const)
    }

    /// Assignment throws a TypeError.
    pub fn is_immutable(self) -> bool {
        matches!(self, VarKind::Const | VarKind::FuncName)
    }
}

pub(super) struct Var {
    pub name: Atom,
    pub kind: VarKind,
    /// Where the binding lives (for `var`: the body scope).
    pub scope: ScopeId,
    /// Source offset where initialization completes; accesses at or after it need no TDZ
    /// check (docs/compiler.md 5.6). `u32::MAX` until the declaration is complete; 0 for
    /// bindings that are never in TDZ.
    pub init_end: u32,
    /// Script top-level `var`/`let`/`const`/function: a global binding, not a local.
    pub global: bool,
    /// Parameter index for `Param`.
    pub param: u16,
    /// Some inner function refers to it: it lives in a Box.
    pub captured: bool,
    /// Referenced at all (for the pseudo-bindings, which cost prologue code).
    pub used: bool,
}

/// A name reference in phase-1 code: the `name_*` op's immediate indexes this table.
#[derive(Clone, Copy, Debug)]
pub(super) struct NameRef {
    pub atom: Atom,
    /// The innermost scope at the reference.
    pub scope: ScopeId,
    /// Source offset of the identifier.
    pub pos: u32,
    /// The reference initializes its binding (`name_init`): never TDZ-checked.
    pub init: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum FuncKind {
    Script,
    AsyncBody,
    /// A function declaration or expression.
    Normal,
    Arrow,
}

impl FuncKind {
    /// Has its own `this`/`arguments`/`new.target` (arrows inherit them).
    pub fn has_own_this(self) -> bool {
        !matches!(self, FuncKind::Arrow)
    }

    pub fn is_top_level(self) -> bool {
        matches!(self, FuncKind::Script | FuncKind::AsyncBody)
    }
}

/// Where a closure takes a capture's Box from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Capture {
    /// A binding of the parent function (its storage is a Box in a local slot).
    Local(VarId),
    /// Capture `n` of the parent function.
    Outer(u16),
}

/// A try region `[start, end)` whose exceptions go to `target` with the operand stack
/// cut to `depth` (then the exception is pushed).
#[derive(Clone, Copy, Debug)]
pub(super) struct HandlerLabels {
    pub start: crate::bytecode::Label,
    pub end: crate::bytecode::Label,
    pub target: crate::bytecode::Label,
    pub depth: u32,
}

pub(super) struct FuncState {
    pub id: FuncId,
    pub kind: FuncKind,
    /// The enclosing function and the scope in it where this one was created.
    pub parent: Option<(FuncId, ScopeId)>,
    /// Child functions in creation order: `fclosure n` creates `children[n]`.
    pub children: Vec<FuncId>,
    /// The function's name (`name` property, stack traces).
    pub name: Atom,
    pub code: CodeBuf<Phase1>,
    pub scopes: Vec<Scope>,
    pub vars: Vec<Var>,
    pub refs: Vec<NameRef>,
    /// Filled by resolution: what each capture slot holds.
    pub captures: Vec<Capture>,
    pub capture_names: Vec<Atom>,
    pub consts: Vec<Value>,
    pub str_consts: BTreeMap<Vec<u16>, u32>,
    pub num_consts: BTreeMap<u64, u32>,
    /// Try regions in phase-1 labels, innermost first (docs/compiler.md 7.4).
    pub handlers: Vec<HandlerLabels>,
    /// `(phase-1 code position, source offset)`, in increasing code position.
    pub lines: Vec<(u32, u32)>,
    /// `var` declarations as `(name, scope where the declaration appeared)`, for
    /// var/let conflicts.
    pub var_decls: Vec<(Atom, ScopeId)>,
    /// Parameters in order (for `Param` bindings).
    pub params: Vec<VarId>,
    /// Formal parameter count without a rest parameter (the argument area).
    pub param_count: u16,
    /// `fn.length`: parameters before the first default or rest.
    pub length: u16,
    /// Only plain identifiers: parameters stay in their argument slots.
    pub simple_params: bool,
    /// Index of a rest parameter.
    pub rest: Option<u16>,
    pub this_var: Option<VarId>,
    pub arguments_var: Option<VarId>,
    pub new_target_var: Option<VarId>,
    pub func_name_var: Option<VarId>,
    /// Source span (for `Function.prototype.toString`, D009).
    pub src_start: u32,
    pub src_end: u32,
}

pub(super) const BODY_SCOPE: ScopeId = 0;

impl FuncState {
    pub fn new(id: FuncId, kind: FuncKind, parent: Option<(FuncId, ScopeId)>) -> Self {
        FuncState {
            id,
            kind,
            parent,
            children: Vec::new(),
            name: Atom::NULL,
            code: CodeBuf::phase1(),
            scopes: alloc::vec![Scope {
                parent: None,
                kind: ScopeKind::Body,
                vars: Vec::new(),
                funcs: Vec::new(),
            }],
            vars: Vec::new(),
            refs: Vec::new(),
            captures: Vec::new(),
            capture_names: Vec::new(),
            consts: Vec::new(),
            str_consts: BTreeMap::new(),
            num_consts: BTreeMap::new(),
            handlers: Vec::new(),
            lines: Vec::new(),
            var_decls: Vec::new(),
            params: Vec::new(),
            param_count: 0,
            length: 0,
            simple_params: true,
            rest: None,
            this_var: None,
            arguments_var: None,
            new_target_var: None,
            func_name_var: None,
            src_start: 0,
            src_end: 0,
        }
    }

    pub fn new_scope(&mut self, parent: ScopeId, kind: ScopeKind) -> ScopeId {
        self.scopes.push(Scope {
            parent: Some(parent),
            kind,
            vars: Vec::new(),
            funcs: Vec::new(),
        });
        (self.scopes.len() - 1) as ScopeId
    }

    pub fn add_var(&mut self, name: Atom, kind: VarKind, scope: ScopeId) -> VarId {
        let id = self.vars.len() as VarId;
        let immediate = !matches!(kind, VarKind::Let | VarKind::Const | VarKind::Param);
        self.vars.push(Var {
            name,
            kind,
            scope,
            init_end: if immediate { 0 } else { u32::MAX },
            global: false,
            param: 0,
            captured: false,
            used: false,
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

    /// The capture slot for `c`, adding it if new.
    pub fn capture_slot(&mut self, c: Capture, name: Atom) -> u16 {
        if let Some(i) = self.captures.iter().position(|&x| x == c) {
            return i as u16;
        }
        self.captures.push(c);
        self.capture_names.push(name);
        (self.captures.len() - 1) as u16
    }
}
