//! The construct path's instructions that `run_hot` declines, done without `step` (0236).
//!
//! An instruction that leaves the tier-0 loop for `step` costs about 500-1000 wasm
//! instructions before its own work starts (`step_once`, `step` and `exec` are large
//! functions). `new C()` took four such trips (`construct`, `check_ctor`, the return into
//! the constructing frame, `special_object` of new.target) and a derived constructor
//! eight, more than everything else they do. `Exec::decline_fast` sends these opcodes
//! here. Each handler completes its instruction exactly as `step` would (fuel, pc, stack,
//! frame, safepoints), or returns false having changed nothing and `step` runs it: an
//! error to throw, a callee or a `prototype` that needs the general path, a stack that
//! must grow, the heap cap.
//!
//! The compiler keeps most of them out of class constructors (0270): a call without
//! `new` is refused by the call paths (no `check_ctor`), class and builtin constructors
//! return their object themselves (their frames return as calls do, in `run_hot`),
//! `super(...)` in the constructor itself reads the super constructor and new.target
//! from the frame (`super_call_frame`, `super_call_forward`), and `this` needs no check
//! after a first `super()` that every run reaches. A class `new` now leaves the loop
//! once (`construct`), a derived one once more per `super()`.

use super::{Exec, NO_RETRY, fast};
use crate::bytecode::template::func_flags;
use crate::bytecode::{OPS, Op, special};
use crate::heap::ObjectClass;
use crate::interp::frame::{self, Continuation, HEADER};
use crate::value::Value;

const CONSTRUCT: u8 = Op::Construct as u8;
const SUPER_CALL: u8 = Op::SuperCall as u8;
const SUPER_CALL_FRAME: u8 = Op::SuperCallFrame as u8;
const SUPER_CALL_FORWARD: u8 = Op::SuperCallForward as u8;
const REST: u8 = Op::Rest as u8;
const GET_SUPER: u8 = Op::GetSuper as u8;
const CHECK_THIS: u8 = Op::CheckThis as u8;
const CHECK_THIS_UNINIT: u8 = Op::CheckThisUninit as u8;
const CHECK_CTOR_RETURN: u8 = Op::CheckCtorReturn as u8;

/// The opcodes `decline_construct` takes: the test before its call, so that the other
/// declined instructions pay no call on their way to `step`.
#[inline(always)]
pub(super) fn takes(op: u8) -> bool {
    matches!(
        op,
        CONSTRUCT
            | SUPER_CALL
            | SUPER_CALL_FRAME
            | SUPER_CALL_FORWARD
            | REST
            | fast::RETURN
            | fast::RETURN_UNDEF
            | fast::SPECIAL_OBJECT
            | CHECK_THIS
            | CHECK_THIS_UNINIT
            | CHECK_CTOR_RETURN
            | GET_SUPER
    )
}

/// Where `construct_fast` finds the arguments.
#[derive(Clone, Copy)]
enum Args {
    /// Above the callee (`f a1 .. an`).
    Above,
    /// Where the callee goes (`a1 .. an`, `super_call_frame`).
    Below,
    /// The frame's own (`super_call_forward`).
    Frame,
}

impl Exec<'_> {
    /// The instruction `op` at `pc` (declined by `run_hot`), if it is one of the
    /// construct path's and its case needs nothing of `step`.
    #[inline(never)]
    pub(super) fn decline_construct(&mut self, op: u8) -> bool {
        if self.pc >= self.code_len {
            return false;
        }
        let done = match op {
            CONSTRUCT | SUPER_CALL | SUPER_CALL_FRAME | SUPER_CALL_FORWARD => {
                self.construct_fast(op)
            }
            REST => {
                // An allocation that fails changes nothing: `step` runs it then.
                let Ok(a) = self.rest_array() else {
                    return false;
                };
                self.push(a);
                self.advance_op(Op::Rest)
            }
            fast::RETURN | fast::RETURN_UNDEF => self.return_constructed(op),
            fast::SPECIAL_OBJECT => {
                let v = match self.byte(self.pc + 1) {
                    special::NEW_TARGET => self.header(frame::NEW_TARGET),
                    special::ACTIVE_FUNCTION => self.get(self.bp),
                    _ => return false,
                };
                self.push(v);
                self.advance_op(Op::SpecialObject)
            }
            CHECK_THIS => !self.top(0).is_undefined() && self.advance_op(Op::CheckThis),
            CHECK_THIS_UNINIT => {
                if !self.top(0).is_undefined() {
                    return false;
                }
                self.sp -= 1;
                self.advance_op(Op::CheckThisUninit)
            }
            CHECK_CTOR_RETURN => {
                let v = self.top(0);
                (v.is_undefined() || v.as_object().is_some())
                    && self.advance_op(Op::CheckCtorReturn)
            }
            GET_SUPER => {
                // The home object or the active function: never a proxy.
                let Some(o) = self.top(0).as_object() else {
                    return false;
                };
                let p = self
                    .heap
                    .get_prototype_of(o)
                    .map_or(Value::NULL, Value::object);
                self.set(self.sp - 1, p);
                self.advance_op(Op::GetSuper)
            }
            _ => false,
        };
        // As `step_once` leaves it: a retried instruction is over (D1700).
        if done && self.retry_len == 0 {
            self.retry_len = NO_RETRY;
        }
        done
    }

    /// Charges `op` and moves past it (true, for the handlers' `&&`).
    fn advance_op(&mut self, op: Op) -> bool {
        let info = &OPS[op as usize];
        self.fuel -= info.cost.fuel() as i64;
        self.pc += info.size as u32;
        true
    }

    /// `construct argc` (`f a1 .. an`), `super_call argc` (`f a1 .. an newTarget`),
    /// `super_call_frame argc` (`a1 .. an`) or `super_call_forward` (the frame's
    /// arguments) of a bytecode function or builtin constructor whose frame fits the
    /// stack: what `construct_with` does after its checks. (`super_call_spread`, a
    /// spread in `super(...)`, goes through `step`.)
    fn construct_fast(&mut self, op: u8) -> bool {
        let sup = op != CONSTRUCT;
        // Where the arguments come from, and the new.target of a super call.
        let (base, argc, args, f, nt) = match op {
            SUPER_CALL_FRAME => {
                let argc = self.imm_u16(1);
                let nt = self.header(frame::NEW_TARGET);
                (
                    self.sp - argc,
                    argc,
                    Args::Below,
                    self.frame_super(),
                    Some(nt),
                )
            }
            SUPER_CALL_FORWARD => {
                let argc = self.header_int(frame::ARGC);
                let nt = self.header(frame::NEW_TARGET);
                (self.sp, argc, Args::Frame, self.frame_super(), Some(nt))
            }
            _ => {
                let argc = self.imm_u16(1);
                let base = self.sp - 1 - argc - sup as u32;
                let nt = sup.then(|| self.top(0));
                (base, argc, Args::Above, self.get(base), nt)
            }
        };
        let Some(fo) = f.as_object() else {
            return false;
        };
        // (Bound functions and proxies are not of this class.)
        if self.heap.object_class(fo) != ObjectClass::Function || !self.is_constructor(fo) {
            return false;
        }
        let t = self.template_of(fo);
        let th = super::hot::template_head(self.heap.arena().bytes(), t);
        // A native frame, or a sloppy `this` to bind (builtins have none; another
        // function's `this` is an object): `construct_call`'s other paths.
        let builtin =
            self.heap.cell_flags(fo.offset()) & crate::heap::cells::flags::OBJ_BUILTIN != 0;
        if th.flags & self.slow_call_flags & func_flags::NATIVE != 0
            || (builtin && th.flags & func_flags::SLOPPY_THIS != 0)
        {
            return false;
        }
        let top = base + 2 + argc.max(th.params) + HEADER + th.locals + th.stack;
        if top > self.heap.value_array_capacity(self.stack) {
            return false;
        }
        let info = &OPS[op as usize];
        let next = self.pc + info.size as u32;
        // A constructor's `new` entry, as `construct_with` tries it: a host refusal
        // leaves the instruction to `step`.
        if !sup && th.flags & self.slow_call_flags & func_flags::FAST != 0 {
            match self.try_fast_new(t, base, argc, next) {
                Ok(true) => {
                    self.fuel -= info.cost.fuel() as i64;
                    return true;
                }
                Ok(false) => {}
                Err(_) => return false,
            }
        }
        // Allocates `this` (or fails) before anything changes.
        let Ok(Some(this)) = self.construct_this(fo, nt) else {
            return false;
        };
        self.fuel -= info.cost.fuel() as i64;
        // `f this a1 .. an` (the frame overwrites what lies above).
        match args {
            Args::Above => {
                for i in (0..argc).rev() {
                    let v = self.get(base + 1 + i);
                    self.set(base + 2 + i, v);
                }
            }
            Args::Below => {
                for i in (0..argc).rev() {
                    let v = self.get(base + i);
                    self.set(base + 2 + i, v);
                }
            }
            Args::Frame => {
                for i in 0..argc {
                    let v = self.get(self.bp + 2 + i);
                    self.set(base + 2 + i, v);
                }
            }
        }
        self.set(base, f);
        self.set(base + 1, this);
        self.sp = base + 2 + argc;
        // Cannot fail: the frame fits and `this` needs no binding.
        let r = self.construct_call(base, argc, nt, next);
        debug_assert!(matches!(r, Ok(None)));
        true
    }

    /// `return` / `return_undef` from a frame `construct` entered: the result, or
    /// `this` for a non-object (`do_return`, which cannot fail for such a frame).
    fn return_constructed(&mut self, op: u8) -> bool {
        let flags = self.header_int(frame::FLAGS) as i32;
        if Continuation::from_flags(flags) != Continuation::ConstructResult {
            return false;
        }
        self.fuel -= OPS[op as usize].cost.fuel() as i64;
        let v = if op == fast::RETURN {
            self.pop()
        } else {
            Value::UNDEFINED
        };
        let r = self.do_return(v);
        debug_assert!(matches!(r, Ok(None)));
        true
    }
}
