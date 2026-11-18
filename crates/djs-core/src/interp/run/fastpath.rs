//! Calls of `@fast` builtins (D1850): the leaf fast path of `builtins::fast` runs in
//! place of the call when it accepts the arguments.

#[cfg(djs_prelude)]
use alloc::string::String;

use super::Exec;
use crate::builtins::fast::{FAST, MAX_ARGS, Shadow};
use crate::bytecode::template::func_flags;
use crate::bytecode::{OPS, Op};
#[cfg(djs_prelude)]
use crate::heap::Atom;
use crate::heap::cells::FuncTemplate;
use crate::heap::{HeapError, ObjectClass};
#[cfg(djs_prelude)]
use crate::interp::ErrorKind;
use crate::interp::conv::{Fault, FaultKind};
use crate::interp::frame::Continuation;
use crate::value::Value;

/// The `RetryCall` site of a collection at the heap cap (no shadow slot).
pub(super) const COLLECT_SITE: u32 = 31;

/// Fuel per fast call on top of the call instruction's: a builtin's call runs a few
/// instructions at least.
const FAST_FUEL: i64 = 2;

impl Exec<'_> {
    /// The `call` instruction at `pc` when its callee is a `@fast` builtin whose fast path
    /// accepts the arguments: true when it completed. Declining leaves everything
    /// unchanged, and `step` makes the full call.
    #[inline]
    pub(super) fn fast_leaf_call(&mut self, op: u8) -> bool {
        // `call argc` or a short form (`call2`, D2443).
        let (argc, size) = if op == super::fast::CALL {
            (self.imm_u16(1), Op::Call.size() as u32)
        } else {
            ((op - super::fast::CALL0) as u32, Op::Call0.size() as u32)
        };
        let base = self.sp - 2 - argc;
        let Some(o) = self.get(base).as_object() else {
            return false;
        };
        if !self.is_class(o, ObjectClass::Function) {
            return false;
        }
        let t = self.template_of(o);
        if u32::from(self.heap.arena().get::<u16>(t, FuncTemplate::FLAGS))
            & self.slow_call_flags
            & func_flags::FAST
            == 0
        {
            return false;
        }
        // (`call0`..`call3` cost as much as `call`: the table's check.)
        let info = &OPS[Op::Call as usize];
        let next = self.pc + size;
        let idx = self.heap.arena().get(t, FuncTemplate::FAST) as usize;
        // A generated (`@rust`) builtin: straight to its function (a conversion or an
        // error goes the general way, `own_call` through `step`).
        if let Some(leaf) = crate::builtins::fast::leaf_of(idx)
            && argc <= 3
        {
            let mut args = [Value::UNDEFINED; 3];
            for (i, a) in args.iter_mut().enumerate().take(argc as usize) {
                *a = self.get(base + 2 + i as u32);
            }
            let this = self.get(base + 1);
            let empty = crate::builtins::fast::Shadow::EMPTY;
            return match (leaf.run)(self, this, &args[..argc as usize], &empty, &leaf.data) {
                Ok(v) => {
                    self.sp = base;
                    self.push(v);
                    self.pc = next;
                    self.fuel -= info.cost.fuel() as i64 + FAST_FUEL;
                    true
                }
                Err(f) => {
                    if matches!(&*f.0, FaultKind::Heap(HeapError::HostOutOfMemory)) {
                        self.host_out_of_memory();
                    }
                    false
                }
            };
        }
        // `arr.push(x)`, the commonest @fast call, straight to the heap's append (0231):
        // what `try_fast_call` and its `array_push` entry do, without the generic steps.
        if argc == 1
            && self.heap.arena().get(t, FuncTemplate::FAST) as usize
                == crate::builtins::fast::ARRAY_PUSH
            && let Some(a) = self.get(base + 1).as_object()
            && self.is_class(a, ObjectClass::Array)
        {
            match self.heap.array_append_fast(a, self.get(base + 2)) {
                Ok(Some(n)) => {
                    self.sp = base;
                    self.push(crate::interp::conv::number(f64::from(n)));
                    self.pc = next;
                    self.fuel -= info.cost.fuel() as i64 + FAST_FUEL;
                    return true;
                }
                Ok(None) => {}
                Err(e) => {
                    if matches!(e, HeapError::HostOutOfMemory) {
                        self.host_out_of_memory();
                    }
                    return false;
                }
            }
        }
        match self.try_fast_call(t, base, argc, Continuation::PushResult, next) {
            Ok(true) => {
                self.fuel -= info.cost.fuel() as i64;
                true
            }
            Ok(false) => false,
            Err(f) => {
                // The host refused memory: never silently retried (D1690); the run
                // stops at the next instruction boundary. The cap (OutOfMemory) is
                // deterministic, so `step`'s retry throws it.
                if matches!(&*f.0, FaultKind::Heap(HeapError::HostOutOfMemory)) {
                    self.host_out_of_memory();
                }
                false
            }
        }
    }

    #[cfg(djs_prelude)]
    /// `%MarkFast(f)`: gives the builtin `f` the fast path named like it. A name without
    /// an entry fails the prelude (at base-image build time).
    pub(super) fn mark_fast(&mut self, f: Value) -> Result<(), Fault> {
        let bad = |msg: String| Err(Fault::error(ErrorKind::InternalError, msg));
        let Some(o) = f
            .as_object()
            .filter(|&o| self.is_class(o, ObjectClass::Function))
        else {
            return bad(String::from("%MarkFast: not a function"));
        };
        let t = self.template_of(o);
        let name = self
            .heap
            .atom_name(Atom::from_id(self.heap.arena().get(t, FuncTemplate::NAME)));
        let Some(i) = crate::builtins::fast::index(&name) else {
            return bad(crate::text::cat!(
                "%MarkFast: no fast path for ",
                name,
                " (builtins/fast.rs)"
            ));
        };
        let a = self.heap.arena_mut();
        let flags = u32::from(a.get::<u16>(t, FuncTemplate::FLAGS));
        a.set(t, FuncTemplate::FLAGS, (flags | func_flags::FAST) as u16);
        // The table is far below 2^8 entries (a u8 field, D2448; a test checks).
        a.set(t, FuncTemplate::FAST, i as u8);
        Ok(())
    }

    /// Tries the fast path of the `@fast` builtin template `t` for the call at `base`
    /// (`f this a1 .. an`). On success the call is complete: the result went where `cont`
    /// says and `pc` is `ret_pc`. No collection happens here (a native caller keeps Rust
    /// locals across an inline call); the allocation sets `gc_pending` for the next
    /// safepoint as usual.
    #[inline(never)]
    /// `new F(a1 .. an)` (`F a1 .. an` at `base`) with new.target F of a constructor with
    /// a `new` entry (0236): true when it completed, the object at `base` and pc at
    /// `next`. Declining leaves everything unchanged, as for a call.
    pub(super) fn try_fast_new(
        &mut self,
        t: u32,
        base: u32,
        argc: u32,
        next: u32,
    ) -> Result<bool, Fault> {
        let idx = self.heap.arena().get(t, FuncTemplate::FAST) as usize;
        let Some(new) = FAST.get(idx).and_then(|f| f.new) else {
            return Ok(false);
        };
        if argc as usize > MAX_ARGS {
            return Ok(false);
        }
        let mut args = [Value::UNDEFINED; MAX_ARGS];
        for (i, a) in args.iter_mut().enumerate().take(argc as usize) {
            *a = self.get(base + 1 + i as u32);
        }
        let v = match new(self.heap, &args[..argc as usize]) {
            Ok(Some(v)) => v,
            Ok(None) => return Ok(false),
            // As for a call: a host refusal stops the run, anything else is the builtin's.
            Err(f) if matches!(&*f.0, FaultKind::Heap(HeapError::HostOutOfMemory)) => {
                return Err(f);
            }
            Err(_) => return Ok(false),
        };
        self.fuel -= FAST_FUEL;
        self.sp = base;
        self.push(v);
        self.pc = next;
        Ok(true)
    }

    /// The generated function of the `@rust` builtin template `t` (D3260).
    pub(super) fn owned(&self, t: u32) -> Option<&'static crate::builtins::fast::Leaf> {
        crate::builtins::fast::leaf_of(self.heap.arena().get(t, FuncTemplate::FAST) as usize)
    }

    /// The call of a `@rust` builtin (D3260): its generated function runs with every
    /// continuation. A conversion site it reaches runs its operation (`%ToPrimitive`,
    /// `%GetSite`) under `RetryCall`; the call is then made again with the result in the
    /// site's shadow slot. At the heap cap it is made again once after a collection. An
    /// error is thrown with the builtin as the innermost (native) frame.
    #[inline(never)]
    pub(super) fn own_call(
        &mut self,
        f: &'static crate::builtins::fast::Leaf,
        base: u32,
        argc: u32,
        cont: Continuation,
        ret_pc: u32,
    ) -> super::Step {
        use crate::interp::conv::FaultKind;
        let shadow = self.own_shadow.take().unwrap_or(Shadow::EMPTY);
        let n = (argc as usize).min(MAX_ARGS);
        let mut args = [Value::UNDEFINED; MAX_ARGS];
        for (i, a) in args.iter_mut().enumerate().take(n) {
            *a = self.get(base + 2 + i as u32);
        }
        let this = self.get(base + 1);
        match (f.run)(self, this, &args[..n], &shadow, &f.data) {
            Ok(v) => {
                self.fuel -= FAST_FUEL;
                self.sp = base;
                match cont {
                    Continuation::PushResult | Continuation::ConstructResult => self.push(v),
                    Continuation::DiscardResult => {}
                    Continuation::Replace(slot) | Continuation::Retry(slot) => self.set(slot, v),
                    Continuation::Entry | Continuation::Job => {
                        return Ok(self.bottom_done(cont, Ok(v)));
                    }
                    Continuation::Each | Continuation::RetryCall(_) => unreachable!(),
                }
                self.pc = ret_pc;
                Ok(None)
            }
            Err(e) => {
                let (site, id, a, b) = match *e.0 {
                    FaultKind::Convert(site, id, a, b) => (site, id, a, b),
                    // At the heap cap: once, a collection and the call again. The stash
                    // roots the shadow values; entering the no-op frame (`%ToPrimitive` of
                    // undefined returns at once) is the safepoint that collects, where no
                    // caller holds heap values in Rust locals.
                    _ if e.is_out_of_memory() && !shadow.collected && !self.host_oom => {
                        self.heap.request_collect();
                        (
                            COLLECT_SITE,
                            super::intrinsics::TO_PRIMITIVE,
                            Value::UNDEFINED,
                            Value::UNDEFINED,
                        )
                    }
                    _ => {
                        self.native_callee = self.get(base).as_object();
                        return Err(e);
                    }
                };
                let f = crate::interp::realm::installed_intrinsic(self.heap, id);
                // The stash: the call (continuation, return pc, argc), the site, the
                // shadow, and the key an error of the call is thrown at (`pc` now: the
                // call instruction, or the calling native frame's state).
                let stash = base + 2 + argc;
                self.sp = stash;
                self.ensure(stash + 14)?;
                for v in [
                    cont.to_flags(),
                    ret_pc as i32,
                    argc as i32,
                    site as i32,
                    shadow.mask as i32,
                    self.pc as i32,
                ] {
                    self.push(Value::int(v));
                }
                for v in shadow.vals {
                    self.push(v);
                }
                let cbase = self.sp;
                self.push(f);
                self.push(Value::UNDEFINED);
                self.push(a);
                self.push(b);
                self.call(cbase, 2, Continuation::RetryCall(stash), ret_pc)
            }
        }
    }

    /// A leaf intrinsic for generated builtins: the pure ones directly, the others through
    /// `call_intrinsic`.
    #[inline(never)]
    pub(crate) fn l_leaf(&mut self, id: u16, a: [Value; 3], n: u32) -> Result<Value, Fault> {
        if super::hot::is_pure_leaf(id)
            && let Some(v) = self.pure_leaf(id, a)?
        {
            return Ok(v);
        }
        self.n_leaf(id, &a[..n as usize])
    }

    /// A string literal of a generated builtin: its atom (looked up, not allocated per
    /// call; bytecode has its literals in the constant pool).
    #[inline(never)]
    pub(crate) fn l_str(&mut self, s: &'static str) -> Result<Value, Fault> {
        let atom = self.heap.atom_str(s)?;
        Ok(self.heap.atom_to_value(atom)?)
    }

    /// A generated builtin's request to run ToPrimitive of the object `a` with `hint` at
    /// conversion site `site` (out of line: the sites are many and cold).
    #[cold]
    #[inline(never)]
    pub(crate) fn l_primitive_site(&mut self, site: u32, a: Value, hint: &'static str) -> Fault {
        match self.l_str(hint) {
            Ok(b) => Fault::convert(site, super::intrinsics::TO_PRIMITIVE, a, b),
            Err(f) => f,
        }
    }

    /// An error with a literal message, thrown by a generated builtin.
    #[cold]
    #[inline(never)]
    pub(crate) fn l_error(kind: crate::interp::ErrorKind, msg: &'static str) -> Fault {
        Fault::error(kind, alloc::string::String::from(msg))
    }

    #[inline(never)]
    fn l_to_number_slow(&mut self, v: Value) -> Result<f64, Fault> {
        crate::interp::conv::to_number(self.heap, v)
    }

    #[inline(never)]
    fn l_to_string_slow(&mut self, v: Value) -> Result<Value, Fault> {
        Ok(Value::string(crate::interp::conv::to_string(self.heap, v)?))
    }

    /// ToString (conversion.ts) in a generated builtin, `k` the object's conversion site.
    #[inline]
    pub(crate) fn l_to_string_at(&mut self, v: Value, sh: &Shadow, k: u32) -> Result<Value, Fault> {
        if v.as_string().is_some() {
            return Ok(v);
        }
        self.l_to_string_site(v, sh, k)
    }

    #[inline(never)]
    fn l_to_string_site(&mut self, v: Value, sh: &Shadow, k: u32) -> Result<Value, Fault> {
        let p = self.l_site_primitive(v, sh, k, "string")?;
        self.l_to_string_slow(p)
    }

    /// ToNumber (conversion.ts) in a generated builtin, `k` the object's conversion site.
    #[inline]
    pub(crate) fn l_to_number_at(&mut self, v: Value, sh: &Shadow, k: u32) -> Result<f64, Fault> {
        match v.as_number() {
            Some(x) => Ok(x),
            None => self.l_to_number_site(v, sh, k),
        }
    }

    #[inline(never)]
    fn l_to_number_site(&mut self, v: Value, sh: &Shadow, k: u32) -> Result<f64, Fault> {
        let p = self.l_site_primitive(v, sh, k, "number")?;
        self.l_to_number_slow(p)
    }

    /// `v` itself, or for an object its primitive from site `k` (a request if the site
    /// has not run yet).
    #[inline]
    fn l_site_primitive(
        &mut self,
        v: Value,
        sh: &Shadow,
        k: u32,
        hint: &'static str,
    ) -> Result<Value, Fault> {
        if v.as_object().is_none() {
            Ok(v)
        } else if sh.mask & (1 << k) != 0 {
            Ok(sh.vals[k as usize])
        } else {
            Err(self.l_primitive_site(k, v, hint))
        }
    }

    pub(super) fn try_fast_call(
        &mut self,
        t: u32,
        base: u32,
        argc: u32,
        cont: Continuation,
        ret_pc: u32,
    ) -> Result<bool, Fault> {
        if argc as usize > MAX_ARGS {
            return Ok(false);
        }
        let push = match cont {
            Continuation::PushResult => true,
            Continuation::DiscardResult | Continuation::Replace(_) => false,
            _ => return Ok(false),
        };
        let idx = self.heap.arena().get(t, FuncTemplate::FAST) as usize;
        // A generated builtin completes here only with a result: a conversion or an
        // error takes `own_call` (through `Exec::call`).
        if let Some(f) = crate::builtins::fast::leaf_of(idx) {
            let mut args = [Value::UNDEFINED; MAX_ARGS];
            for (i, a) in args.iter_mut().enumerate().take(argc as usize) {
                *a = self.get(base + 2 + i as u32);
            }
            let this = self.get(base + 1);
            let empty = crate::builtins::fast::Shadow::EMPTY;
            return match (f.run)(self, this, &args[..argc as usize], &empty, &f.data) {
                Ok(v) => {
                    self.fuel -= FAST_FUEL;
                    self.sp = base;
                    match cont {
                        Continuation::Replace(slot) => self.set(slot, v),
                        _ if push => self.push(v),
                        _ => {}
                    }
                    self.pc = ret_pc;
                    Ok(true)
                }
                Err(f) if matches!(&*f.0, FaultKind::Heap(HeapError::HostOutOfMemory)) => Err(f),
                _ => Ok(false),
            };
        }
        let Some(ff) = FAST.get(idx) else {
            return Ok(false);
        };
        let mut args = [Value::UNDEFINED; MAX_ARGS];
        for (i, a) in args.iter_mut().enumerate().take(argc as usize) {
            *a = self.get(base + 2 + i as u32);
        }
        let this = self.get(base + 1);
        let v = match ff.call(self.heap, this, &args[..argc as usize]) {
            Ok(Some(v)) => v,
            Ok(None) => return Ok(false),
            // A host refusal stops the run (D1690). At the heap cap the entry declines
            // (D1910): the builtin's own allocations then collect and retry (D1700,
            // D1900). Entries allocate before they change anything, or only make stores
            // the builtin repeats. Any other error is the builtin's to throw, with its own
            // frame in the stack trace.
            Err(f) if matches!(&*f.0, FaultKind::Heap(HeapError::HostOutOfMemory)) => {
                return Err(f);
            }
            Err(_) => return Ok(false),
        };
        // Like the string leaf intrinsics the builtins call, no charge per code unit.
        self.fuel -= FAST_FUEL;
        self.sp = base;
        match cont {
            Continuation::Replace(slot) => self.set(slot, v),
            _ if push => self.push(v),
            _ => {}
        }
        self.pc = ret_pc;
        Ok(true)
    }
}

/// `native::num` out of line, for the generated builtins (size).
#[inline(never)]
pub(crate) fn lnum(n: f64) -> Value {
    crate::interp::run::native::num(n)
}

/// Typed leaves for the generated builtins (D3260): what the `pure_leaf` arms do,
/// called directly instead of through the dispatch.
#[allow(missing_docs)]
impl Exec<'_> {
    #[inline]
    fn l_idx(v: Value) -> u32 {
        v.as_number()
            .filter(|n| *n >= 0.0)
            .map_or(0, |n| n.min(u32::MAX as f64) as u32)
    }

    #[inline(never)]
    pub(crate) fn l_math(id: u16, x: f64, y: f64) -> f64 {
        super::intrinsics::math_leaf(id, x, y).unwrap_or(f64::NAN)
    }

    #[inline]
    pub(crate) fn l_integer(n: f64) -> f64 {
        // Integral int32 values (the common index) skip the libm call; -0 becomes +0.
        if (n as i32) as f64 == n {
            return n + 0.0;
        }
        let t = if n.is_nan() { 0.0 } else { djs_libm::trunc(n) };
        t + 0.0
    }

    #[inline]
    pub(crate) fn l_str_len(&self, s: Value) -> f64 {
        s.as_string().map_or(0.0, |s| self.heap.str_len(s) as f64)
    }

    fn l_flat(&mut self, s: Value) -> Result<crate::value::Str, Fault> {
        let s = s.as_string().expect("string");
        Ok(self.heap.contiguous(s)?)
    }

    #[inline(never)]
    pub(crate) fn l_char_code_at(&mut self, s: Value, n: f64) -> Result<Value, Fault> {
        let s = self.l_flat(s)?;
        Ok(if n >= 0.0 && n < self.heap.str_len(s) as f64 {
            Value::int(self.heap.char_code_at(s, n as u32).unwrap_or(0) as i32)
        } else {
            Value::NAN
        })
    }

    #[inline(never)]
    pub(crate) fn l_substring(&mut self, s: Value, a: Value, b: Value) -> Result<Value, Fault> {
        let s = self.l_flat(s)?;
        let r = crate::builtins::string::substring(self.heap, s, Self::l_idx(a), Self::l_idx(b))?;
        Ok(Value::string(r))
    }

    #[inline(never)]
    pub(crate) fn l_index_of(
        &mut self,
        s: Value,
        t: Value,
        from: Value,
        last: bool,
    ) -> Result<f64, Fault> {
        let s = self.l_flat(s)?;
        let t = t.as_string().expect("string");
        crate::builtins::string::index_of(self.heap, s, t, Self::l_idx(from), last)
    }

    #[inline(never)]
    pub(crate) fn l_trim(&mut self, s: Value, w: Value) -> Result<Value, Fault> {
        let s = self.l_flat(s)?;
        Ok(Value::string(crate::builtins::string::trim(
            self.heap,
            s,
            w.as_int().unwrap_or(0),
        )?))
    }

    #[inline(never)]
    pub(crate) fn l_case(&mut self, s: Value, upper: Value) -> Result<Value, Fault> {
        let s = self.l_flat(s)?;
        Ok(Value::string(crate::builtins::string::to_case(
            self.heap,
            s,
            upper.as_bool() == Some(true),
        )?))
    }

    #[inline(never)]
    pub(crate) fn l_pad(
        &mut self,
        s: Value,
        f: Value,
        n: f64,
        at_start: bool,
    ) -> Result<Value, Fault> {
        let (s, f) = (
            s.as_string().expect("string"),
            f.as_string().expect("string"),
        );
        Ok(Value::string(crate::builtins::string::pad(
            self.heap, s, f, n as u32, at_start,
        )?))
    }

    #[inline(never)]
    pub(crate) fn l_repeat(&mut self, s: Value, n: Value) -> Result<Value, Fault> {
        let s = self.l_flat(s)?;
        Ok(Value::string(crate::builtins::string::repeat(
            self.heap,
            s,
            Self::l_idx(n),
        )?))
    }

    #[inline(never)]
    pub(crate) fn l_num_format(&mut self, op: u8, x: f64, arg: Value) -> Result<Value, Fault> {
        Ok(Value::string(crate::builtins::number::format(
            self.heap, op, x, arg,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use crate::builtins::fast::FAST;

    /// Every entry of the table is marked by the prelude (`@fast` in builtins/*.ts):
    /// `%MarkFast` already fails the prelude for a tag without an entry, this catches an
    /// entry without a tag (dead code).
    #[test]
    fn every_fast_entry_is_marked() {
        let prelude = include_str!("../../../../../builtins/gen/prelude.js");
        for f in FAST {
            assert!(
                prelude.contains(&alloc::format!("%MarkFast({});", f.name)),
                "{} has a fast path but no @fast tag",
                f.name
            );
        }
        let marked = prelude.matches("%MarkFast(").count();
        let mut names: alloc::vec::Vec<&str> = FAST.iter().map(|f| f.name).collect();
        names.extend(crate::builtins::r#gen::leaves::LEAF_NAMES);
        names.sort();
        names.dedup();
        assert_eq!(
            marked,
            names.len(),
            "names in FAST are unique and all marked"
        );
    }
}
