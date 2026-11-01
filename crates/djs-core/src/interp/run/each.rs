//! `%EachDense` (0137): the dense part of the loops of forEach, map, filter, some, every,
//! find, findIndex and reduce. The callback gets one frame (continuation `Each`) that is
//! entered again for every element, instead of a return into the builtin and a new call
//! per element.
//!
//! The loop's state is the intrinsic's own arguments, which stay on the stack right below
//! the callback's frame: `[mode, o, k, len, f, thisArg, a, kValue]` (kValue added here:
//! it is what filter and find keep, and the callback may assign its parameter). A throw
//! unwinds the frame as usual and leaves them behind on the caller's operand stack, which
//! its handler resets. Collections, fuel and heap images see an ordinary frame and
//! ordinary stack values.

use super::*;
use crate::builtins::dense::index;

/// The loop state below the callback's frame.
const STATE: u32 = 8;
const MODE: u32 = 0;
const O: u32 = 1;
const K: u32 = 2;
const LEN: u32 = 3;
const F: u32 = 4;
const THIS: u32 = 5;
const A: u32 = 6;
const KVALUE: u32 = 7;

// The modes (builtins/intrinsics.d.ts).
const FOR_EACH: i32 = 0;
const MAP: i32 = 1;
const FILTER: i32 = 2;
const SOME: i32 = 3;
const EVERY: i32 = 4;
const FIND: i32 = 5;
const FIND_INDEX: i32 = 6;
const REDUCE: i32 = 7;

impl Exec<'_> {
    /// `%EachDense(mode, o, k, len, f, thisArg, a)` from `call_intrinsic`: the arguments
    /// are at `base`, the caller resumes at `next`.
    pub(super) fn each_dense(&mut self, base: u32, next: u32) -> Step {
        let k = self.get(base + K);
        let start = self.each_ok(base).then(|| k.as_number().and_then(index));
        if let Some(Some(k)) = start
            && let Some(x) = self.each_element(base, k)
        {
            self.ensure(base + STATE)?;
            self.sp = base + STATE;
            return self.each_enter(base, k, x, next);
        }
        self.sp = base;
        self.push(k);
        self.pc = next;
        Ok(None)
    }

    /// Whether the loop can run at all: a known mode over an Array and a plain closure
    /// (as `n_call_closure` takes). `a`, where a mode stores, must be an Array, whose
    /// CreateDataProperty runs no user code: for filter a fresh one (a plain species),
    /// so that its length is the count of the elements kept; for reduce a box with the
    /// accumulator as element 0.
    fn each_ok(&self, base: u32) -> bool {
        let Some(f) = self.get(base + F).as_object() else {
            return false;
        };
        if !self.is_class(f, ObjectClass::Function)
            || self
                .heap
                .arena()
                .get(self.template_of(f), FuncTemplate::FLAGS)
                & self.slow_call_flags
                != 0
        {
            return false;
        }
        let array = |i: u32| {
            self.get(base + i)
                .as_object()
                .filter(|&o| self.is_class(o, ObjectClass::Array))
        };
        let (Some(o), a) = (array(O), array(A)) else {
            return false;
        };
        match self.get(base + MODE).as_int().unwrap_or(-1) {
            FOR_EACH | SOME | EVERY | FIND_INDEX => true,
            MAP | FIND => a.is_some(),
            FILTER => a.is_some() && crate::builtins::fast::plain_species(self.heap, o),
            REDUCE => a.is_some_and(|a| self.heap.get_element_fast(a, 0).is_some()),
            _ => false,
        }
    }

    /// Element `k` of the loop's array if it is below `len` and present in the dense store.
    fn each_element(&self, base: u32, k: u32) -> Option<Value> {
        let len = self.get(base + LEN).as_number()?;
        if f64::from(k) >= len {
            return None;
        }
        self.heap
            .get_element_fast(self.get(base + O).as_object()?, k)
    }

    /// The callback's arguments for element `k` (`x`, already in KVALUE): `x, k, o`, for
    /// reduce `accumulator, x, k, o`. The count is the second value.
    fn each_args(&self, base: u32, kv: Value) -> ([Value; 4], u32) {
        let (x, o) = (self.get(base + KVALUE), self.get(base + O));
        if self.get(base + MODE).as_int() == Some(REDUCE) {
            let a = self.get(base + A).as_object();
            let acc = a.and_then(|a| self.heap.get_element_fast(a, 0));
            ([acc.unwrap_or(Value::UNDEFINED), x, kv, o], 4)
        } else {
            ([x, kv, o, Value::UNDEFINED], 3)
        }
    }

    /// Enters the callback's frame for element `k` (`x`), with the current frame (the
    /// builtin's) as its caller resuming at `ret_pc`. `sp` is `base + STATE`.
    fn each_enter(&mut self, base: u32, k: u32, x: Value, ret_pc: u32) -> Step {
        let kv = conv::number(f64::from(k));
        self.set(base + K, kv);
        self.set(base + KVALUE, x);
        self.fuel -= super::native::CALL_FUEL;
        let cb = base + STATE;
        let (args, argc) = self.each_args(base, kv);
        self.ensure(cb + 2 + argc)?;
        self.set(cb, self.get(base + F));
        self.set(cb + 1, self.get(base + THIS));
        for i in 0..argc {
            self.set(cb + 2 + i, args[i as usize]);
        }
        self.sp = cb + 2 + argc;
        self.call(cb, argc, Continuation::Each, ret_pc)
    }

    /// With the callback's frame popped (the builtin's current at its return pc): enters
    /// it for element `next`, or gives the builtin `next`, where its own loop goes on.
    fn each_next(&mut self, base: u32, next: u32) -> Option<Status> {
        if let Some(x) = self.each_element(base, next) {
            return match self.each_enter(base, next, x, self.pc) {
                Ok(st) => st,
                Err(f) => self.each_throw(f),
            };
        }
        self.each_finish(base, f64::from(next));
        None
    }

    /// The builtin (current frame) gets `r`: the k its loop goes on from, or -1 - k when
    /// the callback's result at k decided the search.
    fn each_finish(&mut self, base: u32, r: f64) {
        self.sp = base;
        self.push(conv::number(r));
        // A return is a safepoint (as in `do_return`).
        self.safepoint();
    }

    /// The callback (the current frame, continuation `Each`) returns `v`, from the hot
    /// loop or `do_return`: completes the step, then reuses the frame for the next
    /// element (its arguments and locals written again, `pc` 0, as `call` would write a
    /// new one), or pops it and gives the builtin the k its loop goes on from.
    #[inline(never)]
    pub(super) fn each_return(&mut self, v: Value) -> Option<Status> {
        let base = self.bp - STATE;
        let mode = self.get(base + MODE).as_int().unwrap_or(FOR_EACH);
        let k = self.each_k(base);
        let stop = if mode == FOR_EACH {
            Ok(false)
        } else {
            self.each_complete(base, v)
        };
        let next = match stop {
            Ok(false) => k + 1,
            r => {
                self.pop_frame();
                return match r {
                    Err(f) => self.each_throw(f),
                    _ => {
                        self.each_finish(base, -1.0 - f64::from(k));
                        None
                    }
                };
            }
        };
        // Only the callback's own frame is reused: an async callback's return replaces
        // it by a builtin's (`tail_call_builtin`).
        let x = self.each_element(base, next);
        let (Some(x), true) = (x, self.get(self.bp) == self.get(base + F)) else {
            self.pop_frame();
            return self.each_next(base, next);
        };
        let kv = conv::number(f64::from(next));
        self.set(base + K, kv);
        self.set(base + KVALUE, x);
        self.fuel -= super::native::CALL_FUEL;
        // The frame as `call` writes it: arguments padded with undefined, the header's
        // NEW_TARGET undefined, undefined locals (the callee, `this` and the rest of the
        // header are never written).
        let (bp, hp, lp) = (self.bp, self.hp, self.lp);
        let (args, argc) = if mode == REDUCE {
            self.each_args(base, kv)
        } else {
            ([x, kv, self.get(base + O), Value::UNDEFINED], 3)
        };
        for j in 0..hp - bp - 2 {
            let v = if j < argc {
                args[j as usize]
            } else {
                Value::UNDEFINED
            };
            self.set(bp + 2 + j, v);
        }
        self.set(hp + frame::NEW_TARGET, Value::UNDEFINED);
        let top = lp + self.locals();
        for i in lp..top {
            self.set(i, Value::UNDEFINED);
        }
        self.sp = top;
        self.pc = 0;
        // Frame entry is a safepoint.
        self.safepoint();
        None
    }

    /// The loop's current k.
    fn each_k(&self, base: u32) -> u32 {
        let k = self.get(base + K);
        match k.as_int() {
            Some(i) => i as u32,
            None => k.as_number().and_then(index).unwrap_or(0),
        }
    }

    /// Completes the step of element k with the callback's result `v`; true if that
    /// decided the search (some, every, find, findIndex). A store to `a` (map at k,
    /// filter at its length, find's and reduce's box at 0) of KVALUE: map and reduce put
    /// `v` there first, which the step no longer needs. At the heap cap the store
    /// collects and is tried once more (D1900).
    fn each_complete(&mut self, base: u32, v: Value) -> Result<bool, Fault> {
        let mode = self.get(base + MODE).as_int().unwrap_or(FOR_EACH);
        let truthy = mode != MAP && mode != REDUCE && conv::to_boolean(self.heap, v);
        let (stop, store) = match mode {
            MAP | REDUCE => {
                self.set(base + KVALUE, v);
                (false, true)
            }
            FILTER => (false, truthy),
            SOME | FIND_INDEX => (truthy, false),
            EVERY => (!truthy, false),
            FIND => (truthy, truthy),
            _ => (false, false),
        };
        if !store {
            return Ok(stop);
        }
        let put = |ex: &mut Self| {
            let a = ex.get(base + A).as_object().expect("each: a");
            let i = match mode {
                MAP => ex.each_k(base),
                FILTER => ex.heap.array_length(a),
                _ => 0,
            };
            let v = ex.get(base + KVALUE);
            if ex.heap.create_element_fast(a, i, v)? {
                return Ok(());
            }
            let key = ex.heap.atom_from_index(i)?;
            if !ex.heap.create_data_property(a, key, v)? {
                return type_error("could not define property");
            }
            Ok(())
        };
        match put(self) {
            Err(e) if e.is_out_of_memory() && !self.host_oom => {
                self.safepoint_collect();
                if self.host_oom {
                    return Err(e);
                }
                put(self)?;
            }
            r => r?,
        }
        Ok(stop)
    }

    /// A failed step, thrown as the builtin's CreateDataPropertyOrThrow at its resume
    /// point (the builtin's frame is current).
    fn each_throw(&mut self, f: Fault) -> Option<Status> {
        let pc = self.pc;
        let exc = self.fault_value(f, Some(pc));
        self.throw_value(exc, self.call_site_key(pc))
    }
}
