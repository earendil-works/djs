//! Decoding: [`Insn`], [`decode`], the [`Insns`] iterator, and raw immediate readers for
//! the interpreter.

use super::code::Label;
use super::imm::ImmKind;
use super::ops::{Op, OpInfo};
use crate::heap::Atom;

/// A decoded instruction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Insn {
    pub op: Op,
    /// Position of the opcode byte.
    pub pc: u32,
    /// Raw immediates: unsigned kinds zero-extended, signed kinds as `i32` bits.
    imms: [u32; 2],
}

/// Why bytes are not a valid instruction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DecodeError {
    InvalidOpcode {
        pc: u32,
        byte: u8,
    },
    /// The instruction runs past the end of the code.
    Truncated {
        pc: u32,
    },
}

impl Insn {
    pub fn info(&self) -> &'static OpInfo {
        self.op.info()
    }

    /// Encoded size in bytes.
    pub fn size(&self) -> u32 {
        self.op.size() as u32
    }

    pub fn next_pc(&self) -> u32 {
        self.pc + self.size()
    }

    /// Immediate `i` as a raw `u32`.
    pub fn imm(&self, i: usize) -> u32 {
        self.imms[i]
    }

    /// Immediate `i` as a signed value (for `I8`, `I32` and final-code `Target`s).
    pub fn imm_i32(&self, i: usize) -> i32 {
        self.imms[i] as i32
    }

    /// Immediate `i` as an atom.
    pub fn atom(&self, i: usize) -> Atom {
        Atom::from_id(self.imms[i])
    }

    /// The `Target` of a phase-1 jump, as a label.
    pub fn label(&self) -> Option<Label> {
        self.target_index().map(|i| Label(self.imms[i]))
    }

    /// The absolute target of a final-code jump.
    pub fn jump_target(&self) -> Option<u32> {
        let i = self.target_index()?;
        Some((self.next_pc() as i64 + self.imm_i32(i) as i64) as u32)
    }

    /// The number of values popped (resolving `pop + imm` for calls and counts).
    pub fn pops(&self) -> u32 {
        let info = self.info();
        info.pop as u32 + info.pop_imm.map_or(0, |k| self.imms[k as usize])
    }

    pub fn pushes(&self) -> u32 {
        self.info().push as u32
    }

    fn target_index(&self) -> Option<usize> {
        let imms = self.info().imms;
        (imms.last() == Some(&ImmKind::Target)).then(|| imms.len() - 1)
    }
}

/// Decodes the instruction at `pc`.
pub fn decode(code: &[u8], pc: u32) -> Result<Insn, DecodeError> {
    let at = pc as usize;
    let byte = *code.get(at).ok_or(DecodeError::Truncated { pc })?;
    let op = Op::from_u8(byte).ok_or(DecodeError::InvalidOpcode { pc, byte })?;
    let info = op.info();
    if at + info.size as usize > code.len() {
        return Err(DecodeError::Truncated { pc });
    }
    let mut imms = [0u32; 2];
    let mut p = at + 1;
    for (i, &kind) in info.imms.iter().enumerate() {
        imms[i] = match kind.width() {
            1 if kind.is_signed() => code[p] as i8 as i32 as u32,
            1 => u32::from(code[p]),
            2 => u32::from(read_u16(code, p)),
            _ => read_u32(code, p),
        };
        p += kind.width() as usize;
    }
    Ok(Insn { op, pc, imms })
}

/// Iterates over the instructions of `code` from the start. After an error it yields
/// that error once and stops.
pub fn insns(code: &[u8]) -> Insns<'_> {
    Insns {
        code,
        pc: 0,
        done: false,
    }
}

pub struct Insns<'a> {
    code: &'a [u8],
    pc: u32,
    done: bool,
}

impl Iterator for Insns<'_> {
    type Item = Result<Insn, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.pc as usize >= self.code.len() {
            return None;
        }
        let r = decode(self.code, self.pc);
        match r {
            Ok(insn) => self.pc = insn.next_pc(),
            Err(_) => self.done = true,
        }
        Some(r)
    }
}

// Raw readers for the interpreter's dispatch loop, which knows each opcode's layout
// from the table (`pc + 1` is the first immediate). Out-of-range reads panic (a wasm
// trap on corrupt code), never undefined behavior.

#[inline]
pub fn read_u8(code: &[u8], at: usize) -> u8 {
    code[at]
}

#[inline]
pub fn read_i8(code: &[u8], at: usize) -> i8 {
    code[at] as i8
}

#[inline]
pub fn read_u16(code: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([code[at], code[at + 1]])
}

#[inline]
pub fn read_u32(code: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([code[at], code[at + 1], code[at + 2], code[at + 3]])
}

#[inline]
pub fn read_i32(code: &[u8], at: usize) -> i32 {
    read_u32(code, at) as i32
}
