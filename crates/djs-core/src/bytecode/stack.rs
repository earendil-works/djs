//! The stack-depth checker (docs/compiler.md 4.4).
//!
//! Walks every reachable instruction of final code and computes the operand stack depth
//! before it, from the table's stack effects. It rejects code where the depth would go
//! negative, where two paths reach an instruction with different depths, where a jump
//! lands inside an instruction or outside the code, or where execution can fall off the
//! end. The result is the maximum depth, which the frame reserves (`FuncTemplate::STACK`).
//!
//! Successors: the next instruction, unless the op is `JUMP` or `TERM`; the jump target
//! for `JUMP`/`BRANCH`; for `gosub`, the target with the return address pushed and the
//! next instruction at the current depth (where `ret` comes back, having popped the
//! address). Handler targets are entry points with `depth + 1` (the exception).
//! Unreachable code is decoded but not checked.

use alloc::vec::Vec;

use super::decode::{DecodeError, decode};
use super::handlers::Handler;
use super::ops::flags;

/// Why code was rejected. `pc` is the position of the offending instruction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StackError {
    Decode(DecodeError),
    /// A phase-1 pseudo-op in final code.
    Phase1Op {
        pc: u32,
    },
    /// The instruction pops more values than the stack holds.
    Underflow {
        pc: u32,
        depth: u32,
        pops: u32,
    },
    /// Two paths reach `pc` with different depths.
    Mismatch {
        pc: u32,
        depth: u32,
        other: u32,
    },
    /// A jump (or handler) target that is not an instruction start.
    BadTarget {
        pc: u32,
        target: u32,
    },
    /// Execution continues past the last instruction.
    FallsOffEnd {
        pc: u32,
    },
    /// A handler region that is not a range of instruction starts.
    BadHandler {
        index: usize,
    },
    /// More than `u16::MAX` values.
    TooDeep {
        pc: u32,
    },
}

/// A position whose depth is not known (yet). Depths are below it (`TooDeep`).
const UNKNOWN: u16 = u16::MAX;

/// A handler depth that [`stack_depths`] skips: to be computed from the code.
pub const UNKNOWN_DEPTH: u32 = u32::MAX;

/// Checks final code and returns its maximum operand stack depth.
pub fn check_stack(code: &[u8], handlers: &[Handler]) -> Result<u16, StackError> {
    walk(code, handlers, false, &mut StackScratch::default())
}

/// The buffers of the stack checker, reused across the functions of a compile (0096:
/// four allocations per function otherwise). After [`check_stack_in`], `caches` holds
/// the positions of the `Cache` immediates in code order.
#[derive(Default)]
pub struct StackScratch {
    /// Whether each position is an instruction start, a bit each.
    starts: Vec<u64>,
    /// The depth before each position (u16: a compile's peak memory is the room the host
    /// allocator needs in front of the arena, D3368).
    depth: Vec<u16>,
    work: Vec<u32>,
    /// Forward jumps `(from, target)`, whose targets are checked once every start is known.
    forward: Vec<(u32, u32)>,
    pub caches: Vec<u32>,
}

/// [`check_stack`] with reused buffers; also finds the `Cache` immediates (for
/// `alloc_template`'s numbering, without decoding the code again).
pub fn check_stack_in(
    code: &[u8],
    handlers: &[Handler],
    scratch: &mut StackScratch,
) -> Result<u16, StackError> {
    walk(code, handlers, false, scratch)
}

/// The operand stack depth before each instruction (indexed by pc; [`UNKNOWN_DEPTH`]
/// for unreachable code and for positions inside instructions), with the same checks
/// as [`check_stack`]. Handlers whose depth is [`UNKNOWN_DEPTH`] are skipped, so code
/// reachable only through them stays unknown. The compiler uses this to find handler
/// depths it only knows relative to the code (array destructuring inside expressions).
pub fn stack_depths(code: &[u8], handlers: &[Handler]) -> Result<Vec<u32>, StackError> {
    let mut scratch = StackScratch::default();
    walk(code, handlers, true, &mut scratch)?;
    Ok(scratch
        .depth
        .iter()
        .map(|&d| {
            if d == UNKNOWN {
                UNKNOWN_DEPTH
            } else {
                u32::from(d)
            }
        })
        .collect())
}

// One pass in code order decodes each instruction, marks the starts, collects the
// `Cache` immediates and, where the depth is known by then (from the entry, a fallthrough
// or a forward jump), checks the instruction. Compiled code is nearly all reached that way;
// what is reached only by a backward jump waits in a work list and is decoded again
// (D3341: no list of decoded instructions, one decode per instruction).
fn walk(
    code: &[u8],
    handlers: &[Handler],
    skip_unknown: bool,
    scratch: &mut StackScratch,
) -> Result<u16, StackError> {
    let len = code.len() as u32;
    let mut w = Walk {
        len,
        decoded_to: 0,
        max: 0,
        s: scratch,
    };
    w.s.starts.clear();
    w.s.starts.resize((code.len() + 1).div_ceil(64), 0);
    w.s.depth.clear();
    w.s.depth.resize(code.len(), UNKNOWN);
    w.s.work.clear();
    w.s.forward.clear();
    w.s.caches.clear();
    if len == 0 {
        return Ok(0);
    }
    // The entry and the handlers (whose starts are checked after the pass).
    w.s.depth[0] = 0;
    for (index, h) in handlers.iter().enumerate() {
        if skip_unknown && h.depth == UNKNOWN_DEPTH {
            continue;
        }
        let entry = h
            .depth
            .checked_add(1)
            .filter(|&e| e < u32::from(UNKNOWN))
            .ok_or(StackError::BadHandler { index })?;
        w.reach(h.start, h.target, entry)
            .map_err(|_| StackError::BadHandler { index })?;
        w.max = w.max.max(entry);
    }
    let handler_targets = w.s.forward.len();

    // The pass. A stack error waits while decoding goes on: bytes that do not decode are
    // reported first.
    let mut first = Ok(());
    let mut pc = 0u32;
    while pc < len {
        let insn = decode(code, pc).map_err(StackError::Decode)?;
        let info = insn.info();
        if info.has(flags::PHASE1) {
            return Err(StackError::Phase1Op { pc });
        }
        let mut at = pc + 1;
        for &k in info.imms {
            if k == super::imm::ImmKind::Cache {
                w.s.caches.push(at);
            }
            at += k.width() as u32;
        }
        w.s.starts[pc as usize / 64] |= 1 << (pc % 64);
        w.decoded_to = pc + 1;
        if w.s.depth[pc as usize] != UNKNOWN && first.is_ok() {
            first = w.step(&insn);
        }
        pc = insn.next_pc();
    }
    w.decoded_to = len;
    let starts = &w.s.starts;
    let is_start = |p: u32| is_start(starts, p);
    for (index, h) in handlers.iter().enumerate() {
        if skip_unknown && h.depth == UNKNOWN_DEPTH {
            continue;
        }
        if !is_start(h.start) || !(h.end == len || is_start(h.end)) || h.start > h.end {
            return Err(StackError::BadHandler { index });
        }
    }
    for (i, &(from, target)) in w.s.forward.iter().enumerate() {
        if !is_start(target) {
            return Err(match i < handler_targets {
                true => StackError::BadHandler {
                    index: handlers
                        .iter()
                        .position(|h| h.target == target)
                        .unwrap_or(0),
                },
                false => StackError::BadTarget { pc: from, target },
            });
        }
    }
    first?;
    // What only backward jumps reach (every start is known now).
    while let Some(pc) = w.s.work.pop() {
        let insn = decode(code, pc).map_err(StackError::Decode)?;
        w.step(&insn)?;
    }
    Ok(w.max as u16)
}

/// Whether position `p` is an instruction start (`starts` as a bit set).
fn is_start(starts: &[u64], p: u32) -> bool {
    starts
        .get(p as usize / 64)
        .is_some_and(|w| w & (1 << (p % 64)) != 0)
}

struct Walk<'a> {
    len: u32,
    /// Positions below this were decoded: their starts are known.
    decoded_to: u32,
    max: u32,
    s: &'a mut StackScratch,
}

impl Walk<'_> {
    /// Records the depth `d` at `target`, reached from `from`. A position not yet decoded
    /// is checked to be an instruction start later (`forward`); a decoded one whose depth
    /// becomes known now waits in the work list.
    fn reach(&mut self, from: u32, target: u32, d: u32) -> Result<(), StackError> {
        if target >= self.len {
            return Err(if target == self.len {
                StackError::FallsOffEnd { pc: from }
            } else {
                StackError::BadTarget { pc: from, target }
            });
        }
        let decoded = target < self.decoded_to;
        if !decoded {
            self.s.forward.push((from, target));
        } else if !is_start(&self.s.starts, target) {
            return Err(StackError::BadTarget { pc: from, target });
        }
        let slot = &mut self.s.depth[target as usize];
        if *slot == UNKNOWN {
            *slot = d as u16;
            if decoded {
                self.s.work.push(target);
            }
        } else if u32::from(*slot) != d {
            return Err(StackError::Mismatch {
                pc: target,
                depth: u32::from(*slot),
                other: d,
            });
        }
        Ok(())
    }

    /// Checks `insn` at its depth and reaches its successors.
    fn step(&mut self, insn: &super::decode::Insn) -> Result<(), StackError> {
        let pc = insn.pc;
        let d = u32::from(self.s.depth[pc as usize]);
        let info = insn.info();
        let pops = insn.pops();
        if pops > d {
            return Err(StackError::Underflow { pc, depth: d, pops });
        }
        let after = d - pops + insn.pushes();
        self.max = self.max.max(after).max(d);
        if self.max >= u32::from(UNKNOWN) {
            return Err(StackError::TooDeep { pc });
        }
        if info.has(flags::JUMP) || info.has(flags::BRANCH) {
            let target = insn.jump_target().unwrap_or(self.len);
            self.reach(pc, target, after)?;
        }
        if !info.no_fallthrough() {
            self.reach(pc, insn.next_pc(), after)?;
        }
        Ok(())
    }
}
