//! The instruction table: every opcode, declared once (docs/compiler.md 4.2).
//!
//! From the single `ops!` invocation below the macro generates:
//!
//! - [`Op`], a `repr(u8)` enum in table order (the interpreter matches on it),
//! - [`OPS`], one [`OpInfo`] per opcode: name, immediates, stack effect, cost, flags,
//! - a typed emitter method per opcode on [`CodeBuf`](super::CodeBuf)
//!   (`buf.get_loc(3)`, `buf.jump(label)`, ...),
//! - [`OPS_HASH`], a hash of the whole table for the snapshot build hash.
//!
//! The decoder, the disassembler and the stack checker work from [`OPS`], so they need no
//! per-opcode code.
//!
//! # Table syntax
//!
//! ```text
//! Name method(imm: Kind, ...) pop N [+ imm K] push M [+ imm K] cost Class [flags A | B];
//! ```
//!
//! `pop N + imm K` means "N plus the value of immediate K" (calls pop their arguments);
//! `push M + imm K` likewise.
//! Stack notation in the docs: `a b -> c` pops `b` then `a` and pushes `c`.

use super::imm::ImmKind;

/// Opcode flags (bits of [`OpInfo::flags`]).
pub mod flags {
    /// May throw a JS exception.
    pub const THROWS: u16 = 1 << 0;
    /// May run user code: pushes a frame (a call, or a slow path through a
    /// continuation, docs/compiler.md 7.3).
    pub const CALLS: u16 = 1 << 1;
    /// Only in phase-1 code; the resolution pass rewrites or removes it. The interpreter
    /// treats it as corrupt bytecode.
    pub const PHASE1: u16 = 1 << 2;
    /// Unconditional jump to its `Target` immediate: no fallthrough.
    pub const JUMP: u16 = 1 << 3;
    /// Conditional jump to its `Target` immediate, or fall through.
    pub const BRANCH: u16 = 1 << 4;
    /// No fallthrough and no static successor (`return`, `throw`, `ret`).
    pub const TERM: u16 = 1 << 5;
    /// `gosub`: pushes a return address and jumps; `ret` comes back to the next
    /// instruction.
    pub const GOSUB: u16 = 1 << 6;
    /// Allocates (may fail with out of memory).
    pub const ALLOC: u16 = 1 << 7;
}

/// Kinds for `special_object` (its `U8` immediate).
pub mod special {
    pub const ARGUMENTS: u8 = 0;
    pub const NEW_TARGET: u8 = 1;
    pub const HOME_OBJECT: u8 = 2;
    pub const ACTIVE_FUNCTION: u8 = 3;
    /// The frame's `this` slot (no allocation).
    pub const THIS: u8 = 4;
}

/// The declaration list of `global_decls idx` (Script GlobalDeclarationInstantiation):
/// `CONSTS[idx..]` holds int32 Values `n_var, n_func, n_let, n_const`, then that many
/// atom ids (as int32) in the same order.
pub mod global_decls {
    use alloc::vec::Vec;

    use crate::heap::Atom;
    use crate::value::Value;

    /// The declared names, by kind.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct GlobalDecls {
        pub vars: Vec<Atom>,
        pub funcs: Vec<Atom>,
        pub lets: Vec<Atom>,
        pub consts: Vec<Atom>,
    }

    impl GlobalDecls {
        pub fn is_empty(&self) -> bool {
            self.vars.is_empty()
                && self.funcs.is_empty()
                && self.lets.is_empty()
                && self.consts.is_empty()
        }

        /// The constant pool entries.
        pub fn encode(&self) -> Vec<Value> {
            let lists = [&self.vars, &self.funcs, &self.lets, &self.consts];
            let mut out: Vec<Value> = lists.iter().map(|l| Value::int(l.len() as i32)).collect();
            for l in lists {
                out.extend(l.iter().map(|a| Value::int(a.id() as i32)));
            }
            out
        }

        /// Reads the list starting at `consts[idx]`; `None` if malformed.
        pub fn decode(consts: &[Value], idx: usize) -> Option<GlobalDecls> {
            let count = |i: usize| -> Option<usize> {
                usize::try_from(consts.get(idx + i)?.as_int()?).ok()
            };
            let counts = [count(0)?, count(1)?, count(2)?, count(3)?];
            let mut at = idx + 4;
            let mut lists: [Vec<Atom>; 4] = Default::default();
            for (list, &n) in lists.iter_mut().zip(&counts) {
                for _ in 0..n {
                    let id = consts.get(at)?.as_int()?;
                    list.push(Atom::from_id(id as u32));
                    at += 1;
                }
            }
            let [vars, funcs, lets, consts] = lists;
            Some(GlobalDecls {
                vars,
                funcs,
                lets,
                consts,
            })
        }
    }
}

/// `FuncTemplate::CAPTURE_DESC` words (docs/compiler.md 5.4): where `fclosure` takes each
/// capture's Box from.
pub mod capture_desc {
    /// Set: from the creating closure's captures; clear: from a local slot of the
    /// creating frame.
    pub const OUTER: u32 = 1 << 31;

    pub const fn local(slot: u16) -> u32 {
        slot as u32
    }

    pub const fn outer(capture: u16) -> u32 {
        OUTER | capture as u32
    }

    /// `(is_outer, index)`
    pub const fn decode(word: u32) -> (bool, u16) {
        (word & OUTER != 0, word as u16)
    }
}

/// Fuel cost class. The interpreter charges [`CostClass::fuel`] per instruction; leaf
/// operations add charges proportional to their work (docs/compiler.md 7.6).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum CostClass {
    /// Never executed (phase-1 pseudo-ops) or no work (`nop`).
    Free,
    /// Stack shuffles, locals, constants, arithmetic fast paths.
    Basic,
    /// Property, global and iterator access.
    Access,
    /// Allocates a cell.
    Alloc,
    /// Calls and constructs.
    Call,
}

impl CostClass {
    /// Initial weights; tuned later by benchmarks.
    pub const fn fuel(self) -> u32 {
        match self {
            CostClass::Free => 0,
            CostClass::Basic => 1,
            CostClass::Access => 2,
            CostClass::Alloc => 3,
            CostClass::Call => 5,
        }
    }
}

/// Everything known about an opcode, from the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpInfo {
    /// The disassembler name (snake_case, also the emitter method name).
    pub name: &'static str,
    /// Immediate kinds, in encoding order.
    pub imms: &'static [ImmKind],
    /// Encoded size in bytes: the opcode byte plus the immediates.
    pub size: u8,
    /// Values popped (plus the value of immediate `pop_imm`, if set).
    pub pop: u8,
    pub pop_imm: Option<u8>,
    /// Values pushed (plus the value of immediate `push_imm`, if set).
    pub push: u8,
    pub push_imm: Option<u8>,
    pub cost: CostClass,
    /// [`flags`] bits.
    pub flags: u16,
}

impl OpInfo {
    pub const fn has(&self, flag: u16) -> bool {
        self.flags & flag != 0
    }

    /// Instructions that never continue with the next one.
    pub const fn no_fallthrough(&self) -> bool {
        self.flags & (flags::JUMP | flags::TERM) != 0
    }
}

macro_rules! ops {
    ($(
        $(#[$doc:meta])*
        $name:ident $method:ident ( $( $iname:ident : $ikind:ident ),* )
            pop $pop:literal $( + imm $pidx:literal )?
            push $push:literal $( + imm $qidx:literal )?
            cost $cost:ident
            $( flags $( $flag:ident )|+ )? ;
    )*) => {
        /// An opcode. The discriminant is the encoded byte; the order is the table order.
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
        #[repr(u8)]
        pub enum Op {
            $( $(#[$doc])* $name, )*
        }

        /// The number of opcodes.
        pub const OP_COUNT: usize = [$( Op::$name ),*].len();

        /// The instruction table, indexed by opcode byte.
        pub const OPS: &[OpInfo] = &[ $(
            OpInfo {
                name: trim_underscore(stringify!($method)),
                imms: &[ $( ImmKind::$ikind ),* ],
                size: 1 $( + ImmKind::$ikind.width() )*,
                pop: $pop,
                pop_imm: opt_u8!($( $pidx )?),
                push: $push,
                push_imm: opt_u8!($( $qidx )?),
                cost: CostClass::$cost,
                flags: 0 $( $( | flags::$flag )+ )?,
            },
        )* ];

        impl Op {
            /// All opcodes in table order.
            pub const ALL: [Op; OP_COUNT] = [ $( Op::$name ),* ];
        }

        impl<M: super::code::LabelMode> super::CodeBuf<M> {
            $(
                $(#[$doc])*
                // Emitter names mirror the opcode names (`drop`, `add`, `not`, ...).
                #[allow(clippy::should_implement_trait)]
                #[inline]
                pub fn $method(&mut self $( , $iname: <super::imm::$ikind as super::imm::Imm>::Ty )* ) {
                    self.begin(Op::$name);
                    $( <super::imm::$ikind as super::imm::Imm>::emit(self, $iname); )*
                }
            )*
        }
    };
}

/// `in_` -> `in`: emitter methods that would be Rust keywords end in `_`, the
/// disassembler names do not.
const fn trim_underscore(s: &'static str) -> &'static str {
    let b = s.as_bytes();
    if b.len() > 1 && b[b.len() - 1] == b'_' {
        match core::str::from_utf8(b.split_at(b.len() - 1).0) {
            Ok(t) => t,
            Err(_) => s,
        }
    } else {
        s
    }
}

macro_rules! opt_u8 {
    () => {
        None
    };
    ($v:literal) => {
        Some($v)
    };
}

impl Op {
    /// The opcode for an encoded byte.
    #[inline]
    pub const fn from_u8(b: u8) -> Option<Op> {
        if (b as usize) < OP_COUNT {
            Some(Op::ALL[b as usize])
        } else {
            None
        }
    }

    #[inline]
    pub const fn info(self) -> &'static OpInfo {
        &OPS[self as usize]
    }

    /// Encoded size in bytes (a compile-time constant for a known opcode).
    #[inline]
    pub const fn size(self) -> usize {
        OPS[self as usize].size as usize
    }

    pub const fn name(self) -> &'static str {
        OPS[self as usize].name
    }
}

ops! {
    // ---- Constants and literals ------------------------------------------------------

    /// `-> undefined`
    Undefined undefined() pop 0 push 1 cost Basic;
    /// `-> null`
    Null null() pop 0 push 1 cost Basic;
    /// `-> true`
    PushTrue push_true() pop 0 push 1 cost Basic;
    /// `-> false`
    PushFalse push_false() pop 0 push 1 cost Basic;
    /// `-> n`: a small integer constant.
    PushI8 push_i8(v: I8) pop 0 push 1 cost Basic;
    /// `-> n`: an integer constant.
    PushI32 push_i32(v: I32) pop 0 push 1 cost Basic;
    /// `-> v`: a constant pool entry (non-int32 numbers, strings).
    PushConst push_const(idx: Const) pop 0 push 1 cost Basic;
    /// `-> s`: the string of an atom.
    PushAtom push_atom(atom: Atom) pop 0 push 1 cost Basic;
    /// `-> {}`
    Object object() pop 0 push 1 cost Alloc flags ALLOC;
    /// `v1 .. vn -> array`: an array literal without holes or spread.
    ArrayFrom array_from(n: Count) pop 0 + imm 0 push 1 cost Alloc flags ALLOC;
    /// `array v -> array`
    ArrayPush array_push() pop 2 push 1 cost Basic flags ALLOC;
    /// `array -> array`: append a hole.
    ArrayHole array_hole() pop 1 push 1 cost Basic flags ALLOC;
    /// `array iterable -> array`: append the elements of an iterable (spread).
    ArraySpread array_spread() pop 2 push 1 cost Access flags THROWS | CALLS | ALLOC;
    /// `-> function`: create a closure of child template `child`.
    FClosure fclosure(child: Child) pop 0 push 1 cost Alloc flags ALLOC;
    /// `pattern flags -> regexp`
    RegExp regexp() pop 2 push 1 cost Alloc flags THROWS | ALLOC;
    /// `-> template object`: tagged templates, cached in constant slot `idx`.
    GetTemplateObject get_template_object(idx: Const) pop 0 push 1 cost Alloc flags ALLOC;
    /// `-> v`: `arguments`, `new.target`, home object, ... (function entry only).
    SpecialObject special_object(kind: U8) pop 0 push 1 cost Alloc flags ALLOC;
    /// `-> array`: the actual arguments from `index` on (a rest parameter).
    Rest rest(index: Arg) pop 0 push 1 cost Alloc flags ALLOC;

    // ---- Stack shuffles ----------------------------------------------------------------

    /// `a ->`
    Drop drop() pop 1 push 0 cost Basic;
    /// `a -> a a`
    Dup dup() pop 1 push 2 cost Basic;
    /// `a b -> a b a b`
    Dup2 dup2() pop 2 push 4 cost Basic;
    /// `a b -> b a`
    Swap swap() pop 2 push 2 cost Basic;
    /// `a b -> b`
    Nip nip() pop 2 push 1 cost Basic;
    /// `a b -> b a b`
    Insert2 insert2() pop 2 push 3 cost Basic;
    /// `a b c -> c a b c`
    Insert3 insert3() pop 3 push 4 cost Basic;
    /// `a b c d -> d a b c d`
    Insert4 insert4() pop 4 push 5 cost Basic;
    /// `x a b -> a b x`
    Rot3L rot3l() pop 3 push 3 cost Basic;
    /// `a b x -> x a b`
    Rot3R rot3r() pop 3 push 3 cost Basic;
    /// `x a1 .. an -> x a1 .. an x`: copy the value under the top `n` (`pick 0` is
    /// `dup`). Destructuring reaches its iterator under assignment-target references.
    Pick pick(n: U8) pop 1 + imm 0 push 2 + imm 0 cost Basic;
    /// `x a1 .. an v -> v a1 .. an`: overwrite the value under the top `n` + 1.
    Poke poke(n: U8) pop 2 + imm 0 push 1 + imm 0 cost Basic;

    // ---- Phase 1: scope-generic names and markers (docs/compiler.md 5.2) -------------

    /// `-> v`: read a name.
    NameGet name_get(r: NameRef) pop 0 push 1 cost Free flags PHASE1 | THROWS | CALLS;
    /// `v ->`: assign a name.
    NamePut name_put(r: NameRef) pop 1 push 0 cost Free flags PHASE1 | THROWS | CALLS;
    /// `v -> v`: assign a name, keep the value.
    NameSet name_set(r: NameRef) pop 1 push 1 cost Free flags PHASE1 | THROWS | CALLS;
    /// `v ->`: initialize a declaration (no TDZ check, const allowed).
    NameInit name_init(r: NameRef) pop 1 push 0 cost Free flags PHASE1 | THROWS;
    /// `-> v`: read a name for `typeof` (undeclared globals give `undefined`).
    NameTypeof name_typeof(r: NameRef) pop 0 push 1 cost Free flags PHASE1 | THROWS | CALLS;
    /// Scope entry: TDZ setup, Boxes, hoisted block functions.
    EnterScope enter_scope(scope: Scope) pop 0 push 0 cost Free flags PHASE1;
    /// Scope exit.
    LeaveScope leave_scope(scope: Scope) pop 0 push 0 cost Free flags PHASE1;
    /// Start of a `for (let ...)` iteration: fresh copies of the scope's captured
    /// bindings (`rebox`, CreatePerIterationEnvironment).
    IterScope iter_scope(scope: Scope) pop 0 push 0 cost Free flags PHASE1;
    /// A label position (symbolic labels, docs/compiler.md 4.3).
    Label label(id: LabelId) pop 0 push 0 cost Free flags PHASE1;

    // ---- Locals, arguments, Boxes, captures ----------------------------------------

    /// `-> v`
    GetLoc get_loc(loc: Loc) pop 0 push 1 cost Basic;
    /// `v ->`
    PutLoc put_loc(loc: Loc) pop 1 push 0 cost Basic;
    /// `v -> v`
    SetLoc set_loc(loc: Loc) pop 1 push 1 cost Basic;
    /// `-> v`, ReferenceError if uninitialized (TDZ).
    GetLocChk get_loc_chk(loc: Loc) pop 0 push 1 cost Basic flags THROWS;
    /// `v ->`, ReferenceError if uninitialized.
    PutLocChk put_loc_chk(loc: Loc) pop 1 push 0 cost Basic flags THROWS;
    /// `v -> v`, ReferenceError if uninitialized.
    SetLocChk set_loc_chk(loc: Loc) pop 1 push 1 cost Basic flags THROWS;
    /// Store the uninitialized marker.
    SetLocUninit set_loc_uninit(loc: Loc) pop 0 push 0 cost Basic;
    /// `-> v`
    GetArg get_arg(arg: Arg) pop 0 push 1 cost Basic;
    /// `v ->`
    PutArg put_arg(arg: Arg) pop 1 push 0 cost Basic;
    /// `v -> v`
    SetArg set_arg(arg: Arg) pop 1 push 1 cost Basic;
    /// `-> v`: the value of the Box in local `loc`.
    GetBox get_box(loc: Loc) pop 0 push 1 cost Basic;
    /// `v ->`
    PutBox put_box(loc: Loc) pop 1 push 0 cost Basic;
    /// `v -> v`
    SetBox set_box(loc: Loc) pop 1 push 1 cost Basic;
    /// `-> v` with a TDZ check.
    GetBoxChk get_box_chk(loc: Loc) pop 0 push 1 cost Basic flags THROWS;
    /// `v ->` with a TDZ check.
    PutBoxChk put_box_chk(loc: Loc) pop 1 push 0 cost Basic flags THROWS;
    /// `v -> v` with a TDZ check.
    SetBoxChk set_box_chk(loc: Loc) pop 1 push 1 cost Basic flags THROWS;
    /// `loc = new Box(uninitialized)`
    NewBox new_box(loc: Loc) pop 0 push 0 cost Alloc flags ALLOC;
    /// `v ->`: `loc = new Box(v)`.
    NewBoxInit new_box_init(loc: Loc) pop 1 push 0 cost Alloc flags ALLOC;
    /// `loc = new Box(current value)`: per-iteration bindings.
    Rebox rebox(loc: Loc) pop 0 push 0 cost Alloc flags ALLOC;
    /// `-> v`: the value of the closure's capture `cap`.
    GetCap get_cap(cap: Cap) pop 0 push 1 cost Basic;
    /// `v ->`
    PutCap put_cap(cap: Cap) pop 1 push 0 cost Basic;
    /// `v -> v`
    SetCap set_cap(cap: Cap) pop 1 push 1 cost Basic;
    /// `-> v` with a TDZ check.
    GetCapChk get_cap_chk(cap: Cap) pop 0 push 1 cost Basic flags THROWS;
    /// `v ->` with a TDZ check.
    PutCapChk put_cap_chk(cap: Cap) pop 1 push 0 cost Basic flags THROWS;
    /// `v -> v` with a TDZ check.
    SetCapChk set_cap_chk(cap: Cap) pop 1 push 1 cost Basic flags THROWS;

    // ---- Globals ---------------------------------------------------------------------

    /// `-> v`: global lexical record, then the global object; ReferenceError if missing.
    GetGlobal get_global(atom: Atom) pop 0 push 1 cost Access flags THROWS | CALLS;
    /// `-> v`: like `get_global`, but `undefined` if missing (`typeof`).
    GetGlobalTypeof get_global_typeof(atom: Atom) pop 0 push 1 cost Access flags THROWS | CALLS;
    /// `v ->`: ReferenceError if missing, TypeError for a global const.
    PutGlobal put_global(atom: Atom) pop 1 push 0 cost Access flags THROWS | CALLS;
    /// `v ->`: initialize a Script top-level let/const/class.
    InitGlobalLex init_global_lex(atom: Atom) pop 1 push 0 cost Access flags THROWS | ALLOC;
    /// Script GlobalDeclarationInstantiation from the declaration list in constant `idx`.
    GlobalDecls global_decls(idx: Const) pop 0 push 0 cost Access flags THROWS | ALLOC;
    /// TypeError `'x' is read-only`.
    ThrowConstAssign throw_const_assign(atom: Atom) pop 0 push 0 cost Basic flags THROWS | TERM;

    // ---- Properties --------------------------------------------------------------------

    /// `obj -> v`
    GetField get_field(atom: Atom) pop 1 push 1 cost Access flags THROWS | CALLS;
    /// `obj -> f obj`: callee and `this` for a method call.
    GetMethod get_method(atom: Atom) pop 1 push 2 cost Access flags THROWS | CALLS;
    /// `obj v ->`
    PutField put_field(atom: Atom) pop 2 push 0 cost Access flags THROWS | CALLS | ALLOC;
    /// `obj key -> v`
    GetElem get_elem() pop 2 push 1 cost Access flags THROWS | CALLS;
    /// `obj key -> f obj`
    GetElemMethod get_elem_method() pop 2 push 2 cost Access flags THROWS | CALLS;
    /// `obj key v ->`
    PutElem put_elem() pop 3 push 0 cost Access flags THROWS | CALLS | ALLOC;
    /// `obj v -> obj`: CreateDataProperty (literals).
    DefineField define_field(atom: Atom) pop 2 push 1 cost Access flags THROWS | ALLOC;
    /// `obj key v -> obj`: computed keys.
    DefineElem define_elem() pop 3 push 1 cost Access flags THROWS | CALLS | ALLOC;
    /// `obj f -> obj`
    DefineGetter define_getter(atom: Atom) pop 2 push 1 cost Access flags THROWS | ALLOC;
    /// `obj f -> obj`
    DefineSetter define_setter(atom: Atom) pop 2 push 1 cost Access flags THROWS | ALLOC;
    /// `obj key f -> obj`
    DefineGetterElem define_getter_elem() pop 3 push 1 cost Access flags THROWS | CALLS | ALLOC;
    /// `obj key f -> obj`
    DefineSetterElem define_setter_elem() pop 3 push 1 cost Access flags THROWS | CALLS | ALLOC;
    /// `obj f -> obj`: also sets the home object.
    DefineMethod define_method(atom: Atom) pop 2 push 1 cost Access flags THROWS | ALLOC;
    /// `obj key f -> obj`
    DefineMethodElem define_method_elem() pop 3 push 1 cost Access flags THROWS | CALLS | ALLOC;
    /// `obj v -> obj`: a literal `__proto__: v` sets the prototype when `v` is an object
    /// or null (other values are ignored).
    SetProto set_proto() pop 2 push 1 cost Basic flags THROWS;
    /// `key f -> key f`: SetFunctionName(f, ToPropertyKey(key), prefix) for an anonymous
    /// function under a computed key (`prefix`: 0 none, 1 "get", 2 "set").
    SetFunctionName set_function_name(prefix: U8) pop 2 push 2 cost Basic flags THROWS | CALLS;
    /// `obj src -> obj`: object spread / rest (flags select the variant).
    CopyDataProps copy_data_props(mode: U8) pop 2 push 1 cost Access flags THROWS | CALLS | ALLOC;
    /// `obj key -> bool`
    Delete delete() pop 2 push 1 cost Access flags THROWS | CALLS;
    /// `key obj -> bool`
    In in_() pop 2 push 1 cost Access flags THROWS | CALLS;
    /// `obj -> v`: private field (brand check inside).
    GetPrivate get_private(atom: Atom) pop 1 push 1 cost Access flags THROWS | CALLS;
    /// `obj v ->`
    PutPrivate put_private(atom: Atom) pop 2 push 0 cost Access flags THROWS | CALLS;
    /// `obj v -> obj`
    DefinePrivate define_private(atom: Atom) pop 2 push 1 cost Access flags THROWS | ALLOC;
    /// `obj -> bool`: `#x in obj`.
    PrivateIn private_in(atom: Atom) pop 1 push 1 cost Access flags THROWS;
    /// `this home -> v`
    GetSuperField get_super_field(atom: Atom) pop 2 push 1 cost Access flags THROWS | CALLS;
    /// `this home v ->`
    PutSuperField put_super_field(atom: Atom) pop 3 push 0 cost Access flags THROWS | CALLS | ALLOC;
    /// `this home key -> v`
    GetSuperElem get_super_elem() pop 3 push 1 cost Access flags THROWS | CALLS;
    /// `this home key v ->`
    PutSuperElem put_super_elem() pop 4 push 0 cost Access flags THROWS | CALLS | ALLOC;
    /// `v -> v`: TypeError for `null`/`undefined` (destructuring).
    RequireObjectCoercible require_object_coercible() pop 1 push 1 cost Basic flags THROWS;

    // ---- Operators -----------------------------------------------------------------------

    /// `a -> -a`
    Neg neg() pop 1 push 1 cost Basic flags THROWS | CALLS;
    /// `a -> +a` (ToNumber)
    Plus plus() pop 1 push 1 cost Basic flags THROWS | CALLS;
    /// `a -> ~a`
    BNot bnot() pop 1 push 1 cost Basic flags THROWS | CALLS;
    /// `a -> !a`
    Not not() pop 1 push 1 cost Basic;
    /// `a -> typeof a`
    Typeof typeof_() pop 1 push 1 cost Basic;
    /// `a -> a + 1` (numeric)
    Inc inc() pop 1 push 1 cost Basic flags THROWS | CALLS;
    /// `a -> a - 1` (numeric)
    Dec dec() pop 1 push 1 cost Basic flags THROWS | CALLS;
    /// `a -> ToNumeric(a)` (postfix `++`/`--`)
    ToNumeric to_numeric() pop 1 push 1 cost Basic flags THROWS | CALLS;
    /// `a -> ToString(a)` (template substitutions)
    ToString to_string() pop 1 push 1 cost Basic flags THROWS | CALLS | ALLOC;
    /// `a b -> a + b`
    Add add() pop 2 push 1 cost Basic flags THROWS | CALLS | ALLOC;
    /// `a b -> a - b`
    Sub sub() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a * b`
    Mul mul() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a / b`
    Div div() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a % b`
    Mod mod_() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a ** b`
    Pow pow() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a << b`
    Shl shl() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a >> b`
    Sar sar() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a >>> b`
    Shr shr() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a & b`
    BAnd band() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a | b`
    BOr bor() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a ^ b`
    BXor bxor() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a < b`
    Lt lt() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a <= b`
    Le le() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a > b`
    Gt gt() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a >= b`
    Ge ge() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a == b`
    Eq eq() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a != b`
    Ne ne() pop 2 push 1 cost Basic flags THROWS | CALLS;
    /// `a b -> a === b`
    StrictEq strict_eq() pop 2 push 1 cost Basic;
    /// `a b -> a !== b`
    StrictNe strict_ne() pop 2 push 1 cost Basic;
    /// `a b -> a instanceof b`
    Instanceof instanceof() pop 2 push 1 cost Access flags THROWS | CALLS;
    /// `v -> v is null or undefined`
    IsNullish is_nullish() pop 1 push 1 cost Basic;
    /// `v -> v === undefined`
    IsUndefined is_undefined() pop 1 push 1 cost Basic;
    /// `s1 .. sn -> s`: join template literal parts in one allocation.
    Concat concat(n: Count) pop 0 + imm 0 push 1 cost Alloc flags THROWS | ALLOC;

    // ---- Control flow ------------------------------------------------------------------

    /// Unconditional jump.
    Jump jump(target: Target) pop 0 push 0 cost Basic flags JUMP;
    /// `v ->`: jump if ToBoolean(v) is false.
    JumpIfFalse jump_if_false(target: Target) pop 1 push 0 cost Basic flags BRANCH;
    /// `v ->`: jump if ToBoolean(v) is true.
    JumpIfTrue jump_if_true(target: Target) pop 1 push 0 cost Basic flags BRANCH;
    /// `-> ret`: push the return address (an int32) and jump (enter a `finally`).
    Gosub gosub(target: Target) pop 0 push 1 cost Basic flags GOSUB;
    /// `ret ->`: jump to the popped return address.
    Ret ret() pop 1 push 0 cost Basic flags TERM;
    /// `v ->`: throw `v`.
    Throw throw() pop 1 push 0 cost Basic flags THROWS | TERM;
    /// `v ->`: return `v`.
    Return return_() pop 1 push 0 cost Basic flags TERM;
    /// Return `undefined`.
    ReturnUndef return_undef() pop 0 push 0 cost Basic flags TERM;
    /// `f this a1 .. an -> r`
    Call call(argc: Argc) pop 2 + imm 0 push 1 cost Call flags THROWS | CALLS | ALLOC;
    /// `f a1 .. an -> r`: `new f(...)`, new.target = f.
    Construct construct(argc: Argc) pop 1 + imm 0 push 1 cost Call flags THROWS | CALLS | ALLOC;
    /// `f this args -> r`: call with an argument array (spread).
    CallSpread call_spread() pop 3 push 1 cost Call flags THROWS | CALLS | ALLOC;
    /// `f args -> r`
    ConstructSpread construct_spread() pop 2 push 1 cost Call flags THROWS | CALLS | ALLOC;
    /// `a1 .. an -> r`: `super(...)` in a derived constructor.
    SuperCall super_call(argc: Argc) pop 0 + imm 0 push 1 cost Call flags THROWS | CALLS | ALLOC;
    /// `a1 .. an -> r`: privileged code only (docs/compiler.md 7.3).
    CallIntrinsic call_intrinsic(id: Intrinsic, argc: Argc) pop 0 + imm 1 push 1 cost Call flags THROWS | CALLS | ALLOC;
    /// `obj -> iterator`
    GetIterator get_iterator() pop 1 push 1 cost Access flags THROWS | CALLS | ALLOC;
    /// `v -> v`: TypeError if `v` is not an object (iterator results).
    IterCheckObject iter_check_object() pop 1 push 1 cost Basic flags THROWS;
    /// `iterator ->`: call `iterator.return()` if present.
    IteratorClose iterator_close() pop 1 push 0 cost Access flags THROWS | CALLS;
    /// `obj -> for-in iterator` (a snapshot of the enumerable keys)
    ForInStart for_in_start() pop 1 push 1 cost Alloc flags THROWS | ALLOC;
    /// `it -> it key done`
    ForInNext for_in_next() pop 1 push 3 cost Access flags THROWS;
    /// TypeError if a class constructor is called without `new`.
    CheckCtor check_ctor() pop 0 push 0 cost Basic flags THROWS;
    /// `debugger` (no effect).
    Debugger debugger() pop 0 push 0 cost Free;
    /// No operation.
    Nop nop() pop 0 push 0 cost Free;
}

const _: () = {
    assert!(OP_COUNT <= 256);
    // Jump targets are always the last immediate: fixups and `Insn::jump_target` rely
    // on the offset being relative to the end of the instruction.
    let mut i = 0;
    while i < OP_COUNT {
        let info = &OPS[i];
        let mut j = 0;
        while j < info.imms.len() {
            if matches!(info.imms[j], ImmKind::Target) {
                assert!(
                    j + 1 == info.imms.len(),
                    "Target must be the last immediate"
                );
                assert!(
                    info.flags & (flags::JUMP | flags::BRANCH | flags::GOSUB) != 0,
                    "only jumps have a Target"
                );
            }
            j += 1;
        }
        let jumpy = info.flags & (flags::JUMP | flags::BRANCH | flags::GOSUB) != 0;
        assert!(
            !jumpy || (info.imms.len() == 1 && matches!(info.imms[0], ImmKind::Target)),
            "jumps have exactly one Target immediate"
        );
        if let Some(k) = info.pop_imm {
            assert!((k as usize) < info.imms.len(), "pop_imm out of range");
        }
        if let Some(k) = info.push_imm {
            assert!((k as usize) < info.imms.len(), "push_imm out of range");
        }
        i += 1;
    }
};

/// Hash of the instruction table: names, immediates, stack effects, costs, flags and
/// order. Part of the snapshot build hash (with the heap's `LAYOUT_HASH`, see
/// [`build_hash`]): bytecode is only valid for the table it was compiled with.
pub const OPS_HASH: u32 = hash_ops(OPS);

/// FNV-1a over the table.
pub const fn hash_ops(ops: &[OpInfo]) -> u32 {
    const fn mix(h: u32, b: u8) -> u32 {
        (h ^ b as u32).wrapping_mul(0x0100_0193)
    }
    let mut h: u32 = 0x811c_9dc5;
    let mut i = 0;
    while i < ops.len() {
        let op = &ops[i];
        let name = op.name.as_bytes();
        let mut j = 0;
        while j < name.len() {
            h = mix(h, name[j]);
            j += 1;
        }
        h = mix(h, 0);
        let mut j = 0;
        while j < op.imms.len() {
            h = mix(h, op.imms[j] as u8 + 1);
            j += 1;
        }
        h = mix(h, 0);
        h = mix(h, op.size);
        h = mix(h, op.pop);
        h = mix(
            h,
            match op.pop_imm {
                Some(k) => k + 1,
                None => 0,
            },
        );
        h = mix(h, op.push);
        h = mix(
            h,
            match op.push_imm {
                Some(k) => k + 1,
                None => 0,
            },
        );
        h = mix(h, op.cost as u8);
        h = mix(h, op.flags as u8);
        h = mix(h, (op.flags >> 8) as u8);
        i += 1;
    }
    h
}

/// Combines the heap layout hash and [`OPS_HASH`] into one build hash for heap images
/// that contain bytecode (snapshots, P4).
pub const fn build_hash(layout_hash: u32, ops_hash: u32) -> u32 {
    (layout_hash ^ ops_hash.rotate_left(16)).wrapping_mul(0x9E37_79B1) ^ ops_hash
}
