//! The emitter: [`CodeBuf`], labels and jump fixups (docs/compiler.md 4.3).
//!
//! Two modes, chosen by type:
//!
//! - [`CodeBuf<Phase1>`]: what the parser emits. Labels stay symbolic: `place_label`
//!   emits the `label` pseudo-op and jump targets hold label ids.
//! - [`CodeBuf<Final>`] (the default): final code. `place_label` records the position;
//!   jumps are patched to relative offsets by `finish`. The resolution
//!   re-emit uses this mode with the phase-1 label ids
//!   ([`CodeBuf::with_labels`]), and so do hand-assembled tests.
//!
//! Every opcode has a typed emitter method generated from the table (`buf.get_loc(3)`,
//! `buf.call(2)`, `buf.jump(label)`).

use alloc::vec::Vec;
use core::marker::PhantomData;

use super::decode::{Insn, decode};
use super::imm::ImmKind;
use super::ops::Op;

/// A jump target, created by [`CodeBuf::new_label`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct Label(pub u32);

/// Label handling of a [`CodeBuf`]: [`Final`] or [`Phase1`].
pub trait LabelMode: sealed::Sealed {
    const SYMBOLIC: bool;
}

/// Final code: labels are resolved to offsets.
pub struct Final;
/// Phase-1 code: labels stay symbolic.
pub struct Phase1;

impl LabelMode for Final {
    const SYMBOLIC: bool = false;
}
impl LabelMode for Phase1 {
    const SYMBOLIC: bool = true;
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Final {}
    impl Sealed for super::Phase1 {}
}

const UNPLACED: u32 = u32::MAX;

/// Errors found by [`CodeBuf::finish`]. They are compiler bugs, not user errors.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CodeError {
    /// A jump refers to a label that was never placed.
    UnplacedLabel(Label),
    /// A label was placed twice.
    LabelPlacedTwice(Label),
    /// A label id that this buffer did not create.
    UnknownLabel(Label),
    /// The code does not fit `i32` offsets.
    TooLarge,
}

/// Finished final code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code {
    pub code: Vec<u8>,
    /// Position of every label (`u32::MAX` if never placed and never used).
    pub labels: Vec<u32>,
}

impl Code {
    /// The position of a placed label.
    pub fn label_pos(&self, l: Label) -> Option<u32> {
        self.labels
            .get(l.0 as usize)
            .copied()
            .filter(|&p| p != UNPLACED)
    }
}

/// Finished phase-1 code: the instructions, decoded (0096: phase-1 code is never
/// stored as bytes, the re-emit reads the instructions directly). An instruction's `pc`
/// is its index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phase1Code {
    pub insns: Vec<Insn>,
    /// Number of labels; pass it to [`CodeBuf::with_labels`] for the re-emit.
    pub label_count: u32,
}

impl Phase1Code {
    /// The instructions encoded as bytes (label ids in the jumps), for listings and
    /// tests.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for insn in &self.insns {
            out.push(insn.op as u8);
            for (i, &k) in insn.op.info().imms.iter().enumerate() {
                out.extend_from_slice(&insn.imm(i).to_le_bytes()[..k.width() as usize]);
            }
        }
        out
    }
}

/// A bytecode buffer with typed emitters.
pub struct CodeBuf<M: LabelMode = Final> {
    /// Final mode: the code.
    code: Vec<u8>,
    /// Phase-1 mode: the instructions (`pc` = index).
    insns: Vec<Insn>,
    /// Final: label positions (`UNPLACED` until placed). Phase 1: 0/1 placed flags.
    labels: Vec<u32>,
    /// Positions of `Target` immediates, which hold the label id (until `finish` patches
    /// them, in final mode).
    fixups: Vec<u32>,
    /// Start of the last instruction, if nothing (a label) came after it.
    last: Option<u32>,
    error: Option<CodeError>,
    _mode: PhantomData<M>,
}

impl<M: LabelMode> Default for CodeBuf<M> {
    fn default() -> Self {
        Self::make(0)
    }
}

impl CodeBuf<Final> {
    /// An empty buffer for final code.
    pub fn new() -> Self {
        Self::make(0)
    }

    /// A final-code buffer with labels `0..count` already created: the re-emit reuses
    /// the label ids of the phase-1 code.
    pub fn with_labels(count: u32) -> Self {
        Self::make(count)
    }
}

impl CodeBuf<Phase1> {
    /// An empty buffer for phase-1 code.
    pub fn phase1() -> Self {
        let mut b = Self::make(0);
        // Most functions are small: start where doubling from 4 would take three steps.
        b.insns.reserve(32);
        b
    }
}

impl<M: LabelMode> CodeBuf<M> {
    fn make(count: u32) -> Self {
        let init = if M::SYMBOLIC { 0 } else { UNPLACED };
        CodeBuf {
            code: Vec::new(),
            insns: Vec::new(),
            labels: alloc::vec![init; count as usize],
            fixups: Vec::new(),
            last: None,
            error: None,
            _mode: PhantomData,
        }
    }

    pub fn new_label(&mut self) -> Label {
        let l = Label(self.labels.len() as u32);
        self.labels.push(if M::SYMBOLIC { 0 } else { UNPLACED });
        l
    }

    /// Room for `bytes` more code (0092: the re-emit knows its size roughly, so it
    /// does not grow the buffer step by step).
    pub fn reserve(&mut self, bytes: usize) {
        if M::SYMBOLIC {
            self.insns.reserve(bytes / 3);
        } else {
            self.code.reserve(bytes);
        }
    }

    pub fn label_count(&self) -> u32 {
        self.labels.len() as u32
    }

    /// Places `l` at the current position. Nothing emitted before a label can be removed
    /// with [`CodeBuf::truncate_last`] afterwards (a jump may land here).
    pub fn place_label(&mut self, l: Label) {
        let Some(&state) = self.labels.get(l.0 as usize) else {
            self.fail(CodeError::UnknownLabel(l));
            return;
        };
        let unplaced = if M::SYMBOLIC { 0 } else { UNPLACED };
        if state != unplaced {
            self.fail(CodeError::LabelPlacedTwice(l));
        }
        if M::SYMBOLIC {
            self.labels[l.0 as usize] = 1;
            self.label(l);
        } else {
            self.labels[l.0 as usize] = self.code.len() as u32;
        }
        self.last = None;
    }

    /// The current position (where the next instruction starts).
    /// The current position (where the next instruction starts): a byte offset in
    /// final code, an instruction index in phase-1 code.
    pub fn pos(&self) -> u32 {
        if M::SYMBOLIC {
            self.insns.len() as u32
        } else {
            self.code.len() as u32
        }
    }

    /// The final code so far (empty in phase-1 mode).
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    pub fn is_empty(&self) -> bool {
        self.pos() == 0
    }

    /// The last instruction, unless a label was placed after it.
    pub fn last_insn(&self) -> Option<Insn> {
        let start = self.last?;
        if M::SYMBOLIC {
            return self.insns.get(start as usize).copied();
        }
        decode(&self.code, start).ok()
    }

    /// Removes and returns the last instruction (lvalue conversion, docs/compiler.md 3.7).
    /// `None` if there is none or a label was placed after it.
    pub fn truncate_last(&mut self) -> Option<Insn> {
        let insn = self.last_insn()?;
        let start = insn.pc;
        if M::SYMBOLIC {
            self.insns.truncate(start as usize);
            self.last = None;
            return Some(insn);
        }
        self.code.truncate(start as usize);
        while self.fixups.last().is_some_and(|&f| f >= start) {
            self.fixups.pop();
        }
        self.last = None;
        Some(insn)
    }

    /// Re-emits a decoded instruction. `Target` immediates are taken as label ids, so
    /// this copies phase-1 instructions (it is how the re-emit passes through everything
    /// that needs no rewriting).
    pub fn emit_insn(&mut self, insn: &Insn) {
        self.encode(insn.op, insn.imm(0), insn.imm(1));
    }

    /// Emits `op` with the raw immediates `a`, `b` (as many as it has: the low bytes of
    /// each, a label id for a `Target`). Every generated emitter calls this.
    ///
    /// It stays out of line: the per-opcode emitters are inlined into hundreds of
    /// compiler sites, and an inlined `Vec::push` (length check, grow call, store) at
    /// each of them was ~35 KB of wasm (D1571); one call with the opcode and the raw
    /// values is the smallest call site (0092).
    #[inline(never)]
    pub(super) fn encode(&mut self, op: Op, a: u32, b: u32) {
        if M::SYMBOLIC {
            // Phase 1: the instruction itself, and its label id checked.
            let insn = Insn::raw(op, self.insns.len() as u32, [a, b]);
            if let Some(l) = insn.label()
                && (l.0 as usize) >= self.labels.len()
            {
                self.fail(CodeError::UnknownLabel(l));
            }
            self.last = Some(insn.pc);
            self.insns.push(insn);
            return;
        }
        let start = self.code.len() as u32;
        self.last = Some(start);
        // The instruction in a local buffer, then one append.
        let mut bytes = [0u8; 9];
        bytes[0] = op as u8;
        let mut p = 1;
        for (i, &kind) in op.info().imms.iter().enumerate() {
            let v = if i == 0 { a } else { b };
            if kind == ImmKind::Target {
                if (v as usize) >= self.labels.len() {
                    self.fail(CodeError::UnknownLabel(Label(v)));
                }
                self.fixups.push(start + p as u32);
            }
            bytes[p..p + 4].copy_from_slice(&v.to_le_bytes());
            p += kind.width() as usize;
        }
        self.code.extend_from_slice(&bytes[..p]);
    }

    fn fail(&mut self, e: CodeError) {
        self.error.get_or_insert(e);
    }
}

impl CodeBuf<Final> {
    /// Resolves the jump targets and returns the code.
    pub fn finish(mut self) -> Result<Code, CodeError> {
        if let Some(e) = self.error {
            return Err(e);
        }
        if self.code.len() > i32::MAX as usize {
            return Err(CodeError::TooLarge);
        }
        for &at in &self.fixups {
            let at = at as usize;
            let id = u32::from_le_bytes(self.code[at..at + 4].try_into().unwrap_or([0; 4]));
            let target = self.labels[id as usize];
            if target == UNPLACED {
                return Err(CodeError::UnplacedLabel(Label(id)));
            }
            // Relative to the end of the instruction: the target is its last immediate.
            let rel = target as i64 - (at as i64 + 4);
            self.code[at..at + 4].copy_from_slice(&(rel as i32).to_le_bytes());
        }
        Ok(Code {
            code: self.code,
            labels: self.labels,
        })
    }
}

impl CodeBuf<Phase1> {
    /// Moves the code in `mid..end` in front of `start..mid` (loop rotation, 0086).
    /// Phase-1 code refers to labels, not positions, so only the jump fixups move along.
    /// Nothing before `end` can be truncated afterwards, so the fixups need not stay
    /// sorted: `truncate_last` only pops the ones of instructions emitted later.
    pub fn rotate(&mut self, start: u32, mid: u32, end: u32) {
        debug_assert!(start <= mid && mid <= end && end as usize <= self.insns.len());
        let (start, mid, end) = (start as usize, mid as usize, end as usize);
        // (A copy, not `rotate_left`: less code in the wasm build.)
        let moved = self.insns[mid..end].to_vec();
        self.insns.copy_within(start..mid, start + moved.len());
        self.insns[start..start + moved.len()].copy_from_slice(&moved);
        for (i, insn) in self.insns[start..end].iter_mut().enumerate() {
            insn.pc = (start + i) as u32;
        }
        self.last = None;
    }

    /// Checks that every label used by a jump was placed and returns the code.
    pub fn finish(self) -> Result<Phase1Code, CodeError> {
        if let Some(e) = self.error {
            return Err(e);
        }
        for l in self.insns.iter().filter_map(|i| i.label()) {
            if self.labels[l.0 as usize] == 0 {
                return Err(CodeError::UnplacedLabel(l));
            }
        }
        Ok(Phase1Code {
            insns: self.insns,
            label_count: self.labels.len() as u32,
        })
    }
}
