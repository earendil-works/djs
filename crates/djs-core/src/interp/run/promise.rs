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

use super::intrinsics::{PROMISE_CONSTRUCTOR, id};
use crate::heap::cells::{Interp, Roots, ValueArray};
use crate::heap::{Heap, HeapError, Lookup, ObjectClass, Property, Ref, atoms};
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

/// Whether ResolvePromise ( promise, v ) fulfills with `v` without running user code
/// (D3130): `v` is no object, or its `then` is absent or a data property that is not
/// callable, found without meeting a Proxy or a getter. (The caller's promise is never
/// `v`: no self-resolution.)
pub(crate) fn plain_fulfillment(heap: &Heap, v: Value) -> bool {
    let Some(o) = v.as_object() else {
        return true;
    };
    match heap.lookup_property(o, atoms::THEN) {
        Lookup::Absent => true,
        Lookup::Found(_, Property::Data { value, .. }) => {
            !value.as_object().is_some_and(|f| heap.is_callable(f))
        }
        _ => false,
    }
}

/// PromiseResolve ( %Promise%, v ) when it is `v` itself without running user code
/// (D3130): a Promise whose `constructor`, a data property found without a Proxy or a
/// getter on the way, is %Promise%.
pub(crate) fn plain_promise(heap: &Heap, v: Value) -> Option<Obj> {
    let p = as_promise(heap, v)?;
    let ctor = realm::installed_intrinsic(heap, PROMISE_CONSTRUCTOR);
    match heap.lookup_property(p, atoms::CONSTRUCTOR) {
        Lookup::Found(_, Property::Data { value, .. }) if value == ctor => Some(p),
        _ => None,
    }
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

/// Room in the base image's queues (D2067): the job queue (jobs), the host inbox
/// (results) and the unhandled rejections. A VM's first uses need no new cell, so when
/// they come does not move the session's cells, and the queues are emptied, never
/// dropped.
const BASE_JOBS: u32 = 8;
const BASE_INBOX: u32 = 4;
const BASE_REJECTIONS: u32 = 8;

/// Gives the heap its empty queues (the base image's), their slots `undefined` as a
/// taken entry leaves them, so that a used queue and an unused one are the same bytes.
pub(crate) fn preallocate_queues(heap: &mut Heap) -> Result<(), HeapError> {
    let alloc = |heap: &mut Heap, cap: u32| -> Result<u32, HeapError> {
        let q = heap.alloc_value_array(cap)?;
        for k in 0..cap {
            heap.arena_mut()
                .write(ValueArray::VALUES.at(q, k), Value::UNDEFINED);
        }
        Ok(q)
    };
    if jobs(heap) == 0 {
        let q = alloc(heap, BASE_JOBS * JOB_SIZE)?;
        heap.arena_mut().set(0, Roots::JOBS, Ref(q));
    }
    for (field, cap) in [
        (Interp::INBOX, BASE_INBOX * INBOX_SIZE),
        (Interp::REJECTIONS, BASE_REJECTIONS),
    ] {
        if heap.arena().get::<Ref>(state::interp(heap), field).0 == 0 {
            let q = alloc(heap, cap)?;
            let i = state::interp(heap);
            heap.arena_mut().set(i, field, Ref(q));
        }
    }
    Ok(())
}

/// Empties the value array `q` (if any), keeping its cell.
fn empty(heap: &mut Heap, q: u32) {
    if q != 0 {
        for k in 0..heap.value_array_len(q) {
            heap.value_array_set(q, k, Value::UNDEFINED);
        }
        heap.arena_mut().set(q, ValueArray::LEN, 0);
    }
}

/// Drops every queued job (`Vm::abort`).
pub(crate) fn clear_jobs(heap: &mut Heap) {
    empty(heap, jobs(heap));
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
            heap.is_callable(o)
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
                untrack_rejection(heap, p, false);
            }
        }
    }
    set_flags(heap, p, f | FLAG_HANDLED);
    Ok(())
}

// ---------------------------------------------------------------- the host inbox

/// Values per inbox entry: the promise, ok (bool), the value or reason, and the clock
/// (`undefined`: leave it).
pub(crate) const INBOX_SIZE: u32 = 4;

/// Appends a completed async host call to the host inbox (D1670). The run loop applies it
/// as its own macrotask once the job queue is empty, so where a result lands does not
/// depend on when the embedder delivered it.
pub(crate) fn inbox_push(
    heap: &mut Heap,
    p: Obj,
    result: Result<Value, Value>,
    now: Option<f64>,
) -> Result<(), HeapError> {
    let i = state::interp(heap);
    let q = heap.arena().get::<Ref>(i, Interp::INBOX).0;
    if q != 0 {
        // Move the live entries to the front before growing.
        let head = heap.arena().get(i, Interp::INBOX_HEAD);
        let len = heap.value_array_len(q);
        if head > 0 && len + INBOX_SIZE > heap.value_array_capacity(q) {
            for k in head..len {
                let v = heap.value_array_get(q, k);
                heap.value_array_set(q, k - head, v);
            }
            for k in len - head..len {
                heap.value_array_set(q, k, Value::UNDEFINED);
            }
            heap.arena_mut().set(q, ValueArray::LEN, len - head);
            heap.arena_mut().set(i, Interp::INBOX_HEAD, 0);
        }
    }
    let len = if q == 0 { 0 } else { heap.value_array_len(q) };
    let q2 = heap.value_array_reserve(q, len + INBOX_SIZE)?;
    let (ok, v) = match result {
        Ok(v) => (true, v),
        Err(e) => (false, e),
    };
    let now = now.map_or(Value::UNDEFINED, Value::number);
    for (k, x) in [Value::object(p), Value::bool(ok), v, now]
        .into_iter()
        .enumerate()
    {
        heap.value_array_set(q2, len + k as u32, x);
    }
    heap.arena_mut().set(q2, ValueArray::LEN, len + INBOX_SIZE);
    let i = state::interp(heap);
    heap.arena_mut().set(i, Interp::INBOX, Ref(q2));
    Ok(())
}

/// Takes the oldest inbox entry: `(promise, result, now)`.
fn inbox_take(heap: &mut Heap) -> Option<(Obj, Result<Value, Value>, Option<f64>)> {
    let i = state::interp(heap);
    let q = heap.arena().get::<Ref>(i, Interp::INBOX).0;
    if q == 0 {
        return None;
    }
    let head = heap.arena().get(i, Interp::INBOX_HEAD);
    if head >= heap.value_array_len(q) {
        return None;
    }
    let mut e = [Value::UNDEFINED; INBOX_SIZE as usize];
    for (k, x) in e.iter_mut().enumerate() {
        *x = heap.value_array_get(q, head + k as u32);
        // A taken entry must not keep its values alive.
        heap.value_array_set(q, head + k as u32, Value::UNDEFINED);
    }
    let head = head + INBOX_SIZE;
    if head >= heap.value_array_len(q) {
        heap.arena_mut().set(q, ValueArray::LEN, 0);
        heap.arena_mut().set(i, Interp::INBOX_HEAD, 0);
    } else {
        heap.arena_mut().set(i, Interp::INBOX_HEAD, head);
    }
    let p = e[0].as_object().expect("an inbox entry's promise");
    let result = if e[1] == Value::TRUE {
        Ok(e[2])
    } else {
        Err(e[2])
    };
    Some((p, result, e[3].as_number()))
}

/// Applies the oldest inbox entry, if any: sets the clock it carries and queues the job
/// that settles its promise (D1374: a job, so that a thenable result is adopted the way a
/// resolve function would). The job slot is reserved first, so running out of memory
/// leaves the entry in the inbox.
pub(crate) fn inbox_apply(heap: &mut Heap) -> Result<bool, HeapError> {
    if !has_inbox(heap) {
        return Ok(false);
    }
    let q = reserve_jobs(heap, 1)?;
    let (p, result, now) = inbox_take(heap).expect("checked above");
    if let Some(ms) = now {
        realm::set_intrinsic(heap, realm::Intrinsic::Clock, Value::number(ms));
    }
    let job = realm::installed_intrinsic(heap, HOST_CALL_SETTLE);
    let (ok, v) = match result {
        Ok(v) => (true, v),
        Err(e) => (false, e),
    };
    push_job(
        heap,
        q,
        job,
        [Value::object(p), Value::bool(ok), v, Value::UNDEFINED],
    );
    Ok(true)
}

/// Whether the host inbox holds results.
pub(crate) fn has_inbox(heap: &Heap) -> bool {
    let i = state::interp(heap);
    let q = heap.arena().get::<Ref>(i, Interp::INBOX).0;
    q != 0 && heap.arena().get(i, Interp::INBOX_HEAD) < heap.value_array_len(q)
}

/// Drops the host inbox (`Vm::abort`).
pub(crate) fn clear_inbox(heap: &mut Heap) {
    let i = state::interp(heap);
    let q = heap.arena().get::<Ref>(i, Interp::INBOX).0;
    empty(heap, q);
    heap.arena_mut().set(i, Interp::INBOX_HEAD, 0);
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
/// Sets [[PromiseIsHandled]] as a `then` would, without adding a reaction: the embedder
/// takes care of the outcome (D1423).
pub(crate) fn mark_handled(heap: &mut Heap, p: Obj) {
    let f = flags(heap, p);
    if f & FLAG_HANDLED == 0 {
        if state_of(heap, p) == REJECTED {
            // The embedder consumes the outcome itself: withdrawn even when reported.
            untrack_rejection(heap, p, true);
        }
        set_flags(heap, p, f | FLAG_HANDLED);
    }
}

/// HostPromiseRejectionTracker ( promise, "handle" ). A JS handler added after a
/// microtask checkpoint does not withdraw a rejection the checkpoint reported (D1670);
/// the embedder (`reported`) can.
fn untrack_rejection(heap: &mut Heap, p: Obj, reported: bool) {
    let i = state::interp(heap);
    let r = heap.arena().get::<Ref>(i, Interp::REJECTIONS).0;
    if r == 0 {
        return;
    }
    let len = heap.value_array_len(r);
    let done = heap.arena().get(i, Interp::REPORTED).min(len);
    let from = if reported { 0 } else { done };
    let Some(pos) = (from..len).find(|&k| heap.value_array_get(r, k) == Value::object(p)) else {
        return;
    };
    if pos < done {
        heap.arena_mut().set(i, Interp::REPORTED, done - 1);
    }
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
    empty(heap, r);
    heap.arena_mut().set(i, Interp::REPORTED, 0);
    out
}

/// A microtask checkpoint (the job queue ran empty, D1670): the rejections that are still
/// unhandled now count as reported, whether the embedder takes them now or after more
/// macrotasks ran (HTML's "notify about rejected promises").
pub(crate) fn rejection_checkpoint(heap: &mut Heap) {
    let i = state::interp(heap);
    let r = heap.arena().get::<Ref>(i, Interp::REJECTIONS).0;
    let len = if r == 0 { 0 } else { heap.value_array_len(r) };
    heap.arena_mut().set(i, Interp::REPORTED, len);
}

/// Forgets the tracked rejections (`Vm::abort`).
pub(crate) fn clear_rejections(heap: &mut Heap) {
    let i = state::interp(heap);
    let r = heap.arena().get::<Ref>(i, Interp::REJECTIONS).0;
    empty(heap, r);
    heap.arena_mut().set(i, Interp::REPORTED, 0);
}
