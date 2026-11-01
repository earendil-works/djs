//! The `Interp` cell: run state and registers, and the arena roots the interpreter owns
//! (value stack, pending host calls).

use super::frame::Continuation;
use crate::heap::cells::{Interp, Roots, ValueArray};
use crate::heap::{CellKind, Heap, HeapError, Ref};
use crate::value::Value;

/// What the VM is doing, stored in `Interp::STATE`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum RunState {
    /// No call in progress.
    Idle = 0,
    /// A call is in progress and can run.
    Running = 1,
    /// Parked on a host call (`Interp::HOST_CALL`).
    HostCall = 2,
    /// The last call returned `Interp::RESULT`.
    Done = 3,
    /// The last call threw `Interp::RESULT`.
    Threw = 4,
    /// A host call failed: `Interp::RESULT` is thrown at the call when `run` resumes.
    ThrowPending = 5,
}

impl RunState {
    pub const fn from_u32(v: u32) -> RunState {
        match v {
            1 => RunState::Running,
            2 => RunState::HostCall,
            3 => RunState::Done,
            4 => RunState::Threw,
            5 => RunState::ThrowPending,
            _ => RunState::Idle,
        }
    }
}

/// `Interp::FLAGS`: the parked host call's continuation in the low bits, and whether it
/// was the entry callee (no JS frame to return to).
pub const PARK_ENTRY: u32 = 1 << 31;

pub(crate) fn park_flags(cont: Continuation, entry: bool) -> u32 {
    cont.to_flags() as u32 | if entry { PARK_ENTRY } else { 0 }
}

/// Initial value stack capacity, in slots.
pub const INITIAL_STACK: u32 = 1024;

/// The `Interp` cell.
#[inline]
pub(crate) fn interp(heap: &Heap) -> u32 {
    heap.arena().get(0, Roots::INTERP).0
}

/// The value stack cell.
#[inline]
pub(crate) fn stack(heap: &Heap) -> u32 {
    heap.arena().get(0, Roots::STACK).0
}

/// The pending host call table (a `ValueArray` of internal refs to `HostCall` cells).
#[inline]
pub(crate) fn host_calls(heap: &Heap) -> u32 {
    heap.arena().get(0, Roots::HOST_CALLS).0
}

pub(crate) fn state(heap: &Heap) -> RunState {
    RunState::from_u32(heap.arena().get(interp(heap), Interp::STATE))
}

pub(crate) fn set_state(heap: &mut Heap, s: RunState) {
    let i = interp(heap);
    heap.arena_mut().set(i, Interp::STATE, s as u32);
}

pub(crate) fn result(heap: &Heap) -> Value {
    heap.arena().get(interp(heap), Interp::RESULT)
}

pub(crate) fn set_result(heap: &mut Heap, v: Value) {
    let i = interp(heap);
    heap.arena_mut().set(i, Interp::RESULT, v);
}

/// A `ValueArray` whose `LEN` is its full capacity (the stack's slots are all addressable).
pub(crate) fn alloc_stack(heap: &mut Heap, cap: u32) -> Result<u32, HeapError> {
    let s = heap.alloc_value_array(cap)?;
    let cap = heap.value_array_capacity(s);
    heap.arena_mut().set(s, ValueArray::LEN, cap);
    Ok(s)
}

/// Creates the `Interp` cell, the value stack and the host call table.
pub(crate) fn init(heap: &mut Heap) -> Result<(), HeapError> {
    let i = heap.alloc_cell(CellKind::Interp, Interp::LAYOUT.min_size)?;
    let s = alloc_stack(heap, INITIAL_STACK)?;
    let calls = heap.alloc_value_array(4)?;
    let a = heap.arena_mut();
    a.set(i, Interp::RESULT, Value::UNDEFINED);
    a.set(i, Interp::NEXT_CALL_ID, 1);
    a.set(0, Roots::INTERP, Ref(i));
    a.set(0, Roots::STACK, Ref(s));
    a.set(0, Roots::HOST_CALLS, Ref(calls));
    Ok(())
}
