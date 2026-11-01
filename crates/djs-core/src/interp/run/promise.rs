//! Promises and the job queue in Rust (D1370-D1373): the parts that never run user code.
//! `builtins/promise.ts` has the rest (resolving functions, thenables, `then`, the
//! combinators) and calls these through leaf intrinsics.
//!
//! A Promise object (`ObjectClass::Promise`) has four slots:
//!
//! ```text
//! 0  state      int32: PENDING, FULFILLED, REJECTED
//! 1  result     the value or reason
//! 2  reactions  internal ValueArray of [capability, onFulfilled, onRejected] triples
//!               while pending, else undefined
//! 3  flags      int32: FLAG_HANDLED ([[PromiseIsHandled]]), FLAG_RESOLVED
//! ```
//!
//! A *capability* is `undefined` (nothing to resolve: `await`), a Promise (an internal
//! capability of `%Promise%`, resolved directly, with `FLAG_RESOLVED` as its
//! `[[AlreadyResolved]]`), or a Record `[promise, resolve, reject]` (any other
//! constructor). Reactions run as jobs of the installed `%PromiseReactionJob`.
//!
//! The job queue is `Roots::JOBS`, a `ValueArray` of `JOB_SIZE`-value entries `[f, a0,
//! a1, a2, a3]` from `Interp::JOB_HEAD` to its `LEN`: a job calls `f(a0, a1, a2, a3)`
//! with `this` undefined in a frame of its own.

use alloc::vec::Vec;

use super::intrinsics::id;
use crate::heap::cells::{Interp, Roots, ValueArray};
use crate::heap::{Heap, HeapError, ObjectClass, Ref};
use crate::interp::realm;
use crate::interp::state;
use crate::value::{Obj, Value};

pub(crate) const PENDING: i32 = 0;
pub(crate) const FULFILLED: i32 = 1;
pub(crate) const REJECTED: i32 = 2;

/// `[[PromiseIsHandled]]`.
pub(crate) const FLAG_HANDLED: i32 = 1;
/// `[[AlreadyResolved]]` of an internal capability.
pub(crate) const FLAG_RESOLVED: i32 = 2;

const STATE: u32 = 0;
const RESULT: u32 = 1;
const REACTIONS: u32 = 2;
const FLAGS: u32 = 3;

/// Values per job entry: the function and four arguments.
pub(crate) const JOB_SIZE: u32 = 5;

pub(super) const PROMISE_REACTION_JOB: u16 = id("PromiseReactionJob");
const HOST_CALL_SETTLE: u16 = id("HostCallSettle");

/// A new pending promise.
pub(crate) fn alloc(heap: &mut Heap, proto: Option<Obj>) -> Result<Obj, HeapError> {
    let p = heap.alloc_object(ObjectClass::Promise, proto, ObjectClass::Promise.slots())?;
    heap.set_object_slot(p, STATE, Value::int(PENDING));
    heap.set_object_slot(p, RESULT, Value::UNDEFINED);
    heap.set_object_slot(p, REACTIONS, Value::UNDEFINED);
    heap.set_object_slot(p, FLAGS, Value::int(0));
    Ok(p)
}

/// `v` as a Promise object.
pub(crate) fn as_promise(heap: &Heap, v: Value) -> Option<Obj> {
    v.as_object()
        .filter(|&o| heap.object_class(o) == ObjectClass::Promise)
}

pub(crate) fn state_of(heap: &Heap, p: Obj) -> i32 {
    heap.object_slot(p, STATE).as_int().unwrap_or(PENDING)
}

pub(crate) fn result_of(heap: &Heap, p: Obj) -> Value {
    heap.object_slot(p, RESULT)
}

pub(crate) fn flags(heap: &Heap, p: Obj) -> i32 {
    heap.object_slot(p, FLAGS).as_int().unwrap_or(0)
}

pub(crate) fn set_flags(heap: &mut Heap, p: Obj, f: i32) {
    heap.set_object_slot(p, FLAGS, Value::int(f));
}

// ---------------------------------------------------------------- the job queue

fn jobs(heap: &Heap) -> u32 {
    heap.arena().get::<Ref>(0, Roots::JOBS).0
}

/// Room for `n` more jobs; returns the queue cell.
fn reserve_jobs(heap: &mut Heap, n: u32) -> Result<u32, HeapError> {
    let q = jobs(heap);
    let i = state::interp(heap);
    if q != 0 {
        // Move the live entries to the front before growing.
        let head = heap.arena().get(i, Interp::JOB_HEAD);
        let len = heap.value_array_len(q);
        if head > 0 && len + n * JOB_SIZE > heap.value_array_capacity(q) {
            for k in head..len {
                let v = heap.value_array_get(q, k);
                heap.value_array_set(q, k - head, v);
            }
            for k in len - head..len {
                heap.value_array_set(q, k, Value::UNDEFINED);
            }
            heap.arena_mut().set(q, ValueArray::LEN, len - head);
            heap.arena_mut().set(i, Interp::JOB_HEAD, 0);
        }
    }
    let len = if q == 0 { 0 } else { heap.value_array_len(q) };
    let q2 = heap.value_array_reserve(q, len + n * JOB_SIZE)?;
    if q2 != q {
        heap.arena_mut().set(0, Roots::JOBS, Ref(q2));
    }
    Ok(q2)
}

/// Appends a job (room reserved with `reserve_jobs`).
fn push_job(heap: &mut Heap, q: u32, f: Value, args: [Value; 4]) {
    let len = heap.value_array_len(q);
    heap.value_array_set(q, len, f);
    for (k, a) in args.into_iter().enumerate() {
        heap.value_array_set(q, len + 1 + k as u32, a);
    }
    heap.arena_mut().set(q, ValueArray::LEN, len + JOB_SIZE);
}

/// Queues the job `f(args)` (HostEnqueuePromiseJob).
pub(crate) fn enqueue(heap: &mut Heap, f: Value, args: [Value; 4]) -> Result<(), HeapError> {
    let q = reserve_jobs(heap, 1)?;
    push_job(heap, q, f, args);
    Ok(())
}

pub(crate) fn has_jobs(heap: &Heap) -> bool {
    let q = jobs(heap);
    q != 0 && heap.arena().get(state::interp(heap), Interp::JOB_HEAD) < heap.value_array_len(q)
}

/// Takes the oldest job: `[f, a0, a1, a2, a3]`.
pub(crate) fn dequeue(heap: &mut Heap) -> Option<[Value; JOB_SIZE as usize]> {
    if !has_jobs(heap) {
        return None;
    }
    let q = jobs(heap);
    let i = state::interp(heap);
    let head = heap.arena().get(i, Interp::JOB_HEAD);
    let mut job = [Value::UNDEFINED; JOB_SIZE as usize];
    for (k, v) in job.iter_mut().enumerate() {
        *v = heap.value_array_get(q, head + k as u32);
        // A taken job must not keep its values alive.
        heap.value_array_set(q, head + k as u32, Value::UNDEFINED);
    }
    let head = head + JOB_SIZE;
    if head >= heap.value_array_len(q) {
        heap.arena_mut().set(q, ValueArray::LEN, 0);
        heap.arena_mut().set(i, Interp::JOB_HEAD, 0);
    } else {
        heap.arena_mut().set(i, Interp::JOB_HEAD, head);
    }
    Some(job)
}

/// Drops every queued job (`Vm::abort`).
pub(crate) fn clear_jobs(heap: &mut Heap) {
    heap.arena_mut().set(0, Roots::JOBS, Ref(0));
    let i = state::interp(heap);
    heap.arena_mut().set(i, Interp::JOB_HEAD, 0);
}

// ---------------------------------------------------------------- settling and then

/// FulfillPromise / RejectPromise ( promise, value ), 27.2.1.4 and 27.2.1.7, with
/// TriggerPromiseReactions: every reaction becomes a job. A promise that is not pending
/// stays as it is.
pub(crate) fn settle(heap: &mut Heap, p: Obj, fulfilled: bool, v: Value) -> Result<(), HeapError> {
    if state_of(heap, p) != PENDING {
        return Ok(());
    }
    let reactions = heap.object_slot(p, REACTIONS).as_internal();
    let n = reactions.map_or(0, |r| heap.value_array_len(r) / 3);
    // Reserve first, so running out of memory leaves the promise pending.
    let q = reserve_jobs(heap, n)?;
    if !fulfilled && flags(heap, p) & FLAG_HANDLED == 0 {
        track_rejection(heap, p)?;
    }
    let job = realm::installed_intrinsic(heap, PROMISE_REACTION_JOB);
    if let Some(r) = reactions {
        for k in 0..n {
            let cap = heap.value_array_get(r, 3 * k);
            let handler = heap.value_array_get(r, 3 * k + if fulfilled { 1 } else { 2 });
            push_job(heap, q, job, [cap, handler, v, Value::bool(fulfilled)]);
        }
    }
    heap.set_object_slot(
        p,
        STATE,
        Value::int(if fulfilled { FULFILLED } else { REJECTED }),
    );
    heap.set_object_slot(p, RESULT, v);
    heap.set_object_slot(p, REACTIONS, Value::UNDEFINED);
    Ok(())
}

/// PerformPromiseThen ( promise, onFulfilled, onRejected, resultCapability ), 27.2.5.4.1
/// (the result is the capability's promise, which the caller has). Handlers that are
/// not callable are passed as `undefined` ("empty").
pub(crate) fn perform_then(
    heap: &mut Heap,
    p: Obj,
    on_fulfilled: Value,
    on_rejected: Value,
    cap: Value,
) -> Result<(), HeapError> {
    // An async function's state is the handler of an await (D1375).
    let callable = |v: Value| {
        if v.as_object().is_some_and(|o| {
            heap.object_class(o).is_callable()
                || super::suspend::as_generator(heap, v, super::suspend::ASYNC).is_some()
                || super::suspend::as_generator(heap, v, super::suspend::ASYNC_GENERATOR).is_some()
        }) {
            v
        } else {
            Value::UNDEFINED
        }
    };
    let (on_f, on_r) = (callable(on_fulfilled), callable(on_rejected));
    let f = flags(heap, p);
    match state_of(heap, p) {
        PENDING => {
            let r = heap.object_slot(p, REACTIONS).as_internal().unwrap_or(0);
            let len = if r == 0 { 0 } else { heap.value_array_len(r) };
            let r2 = heap.value_array_reserve(r, len + 3)?;
            heap.value_array_set(r2, len, cap);
            heap.value_array_set(r2, len + 1, on_f);
            heap.value_array_set(r2, len + 2, on_r);
            heap.arena_mut().set(r2, ValueArray::LEN, len + 3);
            heap.set_object_slot(p, REACTIONS, Value::internal(r2));
        }
        s => {
            let fulfilled = s == FULFILLED;
            let job = realm::installed_intrinsic(heap, PROMISE_REACTION_JOB);
            let handler = if fulfilled { on_f } else { on_r };
            let v = result_of(heap, p);
            enqueue(heap, job, [cap, handler, v, Value::bool(fulfilled)])?;
            if !fulfilled && f & FLAG_HANDLED == 0 {
                untrack_rejection(heap, p);
            }
        }
    }
    set_flags(heap, p, f | FLAG_HANDLED);
    Ok(())
}

/// Queues the settlement of an async host call's promise (D1374): a job, so that a
/// thenable result is adopted the way a resolve function would.
pub(crate) fn enqueue_host_settle(
    heap: &mut Heap,
    p: Obj,
    result: Result<Value, Value>,
) -> Result<(), HeapError> {
    let job = realm::installed_intrinsic(heap, HOST_CALL_SETTLE);
    let (ok, v) = match result {
        Ok(v) => (true, v),
        Err(e) => (false, e),
    };
    enqueue(
        heap,
        job,
        [Value::object(p), Value::bool(ok), v, Value::UNDEFINED],
    )
}

// ---------------------------------------------------------------- rejection tracking

/// HostPromiseRejectionTracker ( promise, "reject" ): remembered until handled or taken.
fn track_rejection(heap: &mut Heap, p: Obj) -> Result<(), HeapError> {
    let i = state::interp(heap);
    let r = heap.arena().get::<Ref>(i, Interp::REJECTIONS).0;
    let len = if r == 0 { 0 } else { heap.value_array_len(r) };
    let r2 = heap.value_array_reserve(r, len + 1)?;
    heap.value_array_set(r2, len, Value::object(p));
    heap.arena_mut().set(r2, ValueArray::LEN, len + 1);
    heap.arena_mut().set(i, Interp::REJECTIONS, Ref(r2));
    Ok(())
}

/// HostPromiseRejectionTracker ( promise, "handle" ).
fn untrack_rejection(heap: &mut Heap, p: Obj) {
    let i = state::interp(heap);
    let r = heap.arena().get::<Ref>(i, Interp::REJECTIONS).0;
    if r == 0 {
        return;
    }
    let len = heap.value_array_len(r);
    let Some(pos) = (0..len).find(|&k| heap.value_array_get(r, k) == Value::object(p)) else {
        return;
    };
    for k in pos + 1..len {
        let v = heap.value_array_get(r, k);
        heap.value_array_set(r, k - 1, v);
    }
    heap.value_array_set(r, len - 1, Value::UNDEFINED);
    heap.arena_mut().set(r, ValueArray::LEN, len - 1);
}

/// The rejected promises that have no handler, oldest first; forgets them.
pub(crate) fn take_rejections(heap: &mut Heap) -> Vec<Obj> {
    let i = state::interp(heap);
    let r = heap.arena().get::<Ref>(i, Interp::REJECTIONS).0;
    if r == 0 {
        return Vec::new();
    }
    let out = (0..heap.value_array_len(r))
        .filter_map(|k| heap.value_array_get(r, k).as_object())
        .collect();
    heap.arena_mut().set(i, Interp::REJECTIONS, Ref(0));
    out
}

/// Forgets the tracked rejections (`Vm::abort`).
pub(crate) fn clear_rejections(heap: &mut Heap) {
    let i = state::interp(heap);
    heap.arena_mut().set(i, Interp::REJECTIONS, Ref(0));
}
