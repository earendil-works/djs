//! The disassembler: one line per instruction, driven by the table.
//!
//! ```text
//!     0  push_i32 42
//!     5  get_field "length"
//!    10  jump_if_false -> 25
//!    15  get_loc loc1 (x)
//! ```

use alloc::string::String;
use core::fmt::Write;

use super::decode::{Insn, insns};
use super::imm::ImmKind;
use crate::heap::{Atom, Heap};
use crate::value::Value;

/// What the disassembler may use to print operands symbolically. Everything is
/// optional; without it operands are printed as numbers.
#[derive(Clone, Copy, Default)]
pub struct DisasmContext<'a> {
    /// Resolves atoms and string constants.
    pub heap: Option<&'a Heap>,
    /// The constant pool.
    pub consts: &'a [Value],
    /// Local names, then capture names (`FuncTemplate::NAMES`).
    pub names: &'a [Atom],
    /// How many of `names` are locals.
    pub locals: usize,
    /// Phase-1 code: jump targets are label ids.
    pub phase1: bool,
    /// An encoded line table: instructions where an entry starts get a `; line:col`
    /// comment.
    pub lines: &'a [u8],
}

/// Disassembles `code`. Decoding errors end the listing with an `<invalid ...>` line.
pub fn disasm(code: &[u8], cx: &DisasmContext<'_>) -> String {
    let mut out = String::new();
    let mut positions = super::lines::line_entries(cx.lines).peekable();
    for r in insns(code) {
        match r {
            Ok(insn) => {
                line(&mut out, &insn, cx);
                let mut at = None;
                while let Some(e) = positions.next_if(|e| e.pc <= insn.pc) {
                    if e.pc == insn.pc {
                        at = Some(e);
                    }
                }
                if let Some(e) = at {
                    let _ = write!(out, "  ; {}:{}", e.line, e.col);
                }
                out.push('\n');
            }
            Err(e) => {
                let _ = writeln!(out, "<invalid {e:?}>");
            }
        }
    }
    out
}

/// One instruction, without the newline.
pub fn disasm_insn(insn: &Insn, cx: &DisasmContext<'_>) -> String {
    let mut out = String::new();
    line(&mut out, insn, cx);
    out
}

fn line(out: &mut String, insn: &Insn, cx: &DisasmContext<'_>) {
    let info = insn.info();
    if insn.op == super::Op::Label {
        let _ = write!(out, "{:>5}  L{}:", insn.pc, insn.imm(0));
        return;
    }
    let _ = write!(out, "{:>5}  {}", insn.pc, info.name);
    for (i, &kind) in info.imms.iter().enumerate() {
        out.push(' ');
        operand(out, insn, i, kind, cx);
    }
}

fn operand(out: &mut String, insn: &Insn, i: usize, kind: ImmKind, cx: &DisasmContext<'_>) {
    let raw = insn.imm(i);
    let _ = match kind {
        ImmKind::U8 | ImmKind::Count | ImmKind::Argc => write!(out, "{raw}"),
        ImmKind::I8 | ImmKind::I32 => write!(out, "{}", insn.imm_i32(i)),
        ImmKind::Loc => {
            let _ = write!(out, "loc{raw}");
            name(out, cx, raw as usize, raw < cx.locals as u32)
        }
        ImmKind::Arg => write!(out, "arg{raw}"),
        ImmKind::Cap => {
            let _ = write!(out, "cap{raw}");
            name(out, cx, cx.locals + raw as usize, true)
        }
        ImmKind::Scope => write!(out, "s{raw}"),
        ImmKind::Intrinsic => match super::intrinsics::intrinsic(raw as u16) {
            Some(i) => write!(out, "%{}", i.name),
            None => write!(out, "%{raw}"),
        },
        ImmKind::Const if insn.op == super::Op::GlobalDecls => {
            let _ = write!(out, "#{raw}");
            if let Some(d) = super::global_decls::GlobalDecls::decode(cx.consts, raw as usize) {
                let lists = [
                    ("var", &d.vars),
                    ("function", &d.funcs),
                    ("let", &d.lets),
                    ("const", &d.consts),
                ];
                for (kw, list) in lists {
                    if !list.is_empty() {
                        let _ = write!(out, " {kw}");
                        for &a in list.iter() {
                            out.push(' ');
                            atom(out, a, cx);
                        }
                    }
                }
            }
            Ok(())
        }
        ImmKind::Const => {
            let _ = write!(out, "#{raw}");
            if let Some(&v) = cx.consts.get(raw as usize) {
                out.push_str(" (");
                value(out, v, cx);
                out.push(')');
            }
            Ok(())
        }
        ImmKind::Child => write!(out, "child{raw}"),
        ImmKind::NameRef => write!(out, "r{raw}"),
        ImmKind::LabelId => write!(out, "L{raw}"),
        ImmKind::Atom => {
            atom(out, insn.atom(i), cx);
            Ok(())
        }
        ImmKind::Target if cx.phase1 => write!(out, "L{raw}"),
        ImmKind::Target => match insn.jump_target() {
            Some(t) => write!(out, "-> {t}"),
            None => write!(out, "-> ?"),
        },
    };
}

fn name(out: &mut String, cx: &DisasmContext<'_>, idx: usize, ok: bool) -> core::fmt::Result {
    if let (true, Some(&a)) = (ok, cx.names.get(idx)) {
        out.push_str(" (");
        match cx.heap {
            Some(h) => out.push_str(&h.atom_name(a)),
            None => {
                let _ = write!(out, "atom#{}", a.id());
            }
        }
        out.push(')');
    }
    Ok(())
}

fn atom(out: &mut String, a: Atom, cx: &DisasmContext<'_>) {
    match cx.heap {
        Some(h) if !a.is_null() && (a.is_int() || a.id() & 0x3FFF_FFFF < h.atom_count()) => {
            let _ = write!(out, "{:?}", h.atom_name(a));
        }
        _ => {
            let _ = write!(out, "atom#{}", a.id());
        }
    }
}

fn value(out: &mut String, v: Value, cx: &DisasmContext<'_>) {
    if let Some(n) = v.as_number() {
        out.push_str(crate::num::to_string(n).as_str());
        return;
    }
    if let (Some(s), Some(h)) = (v.as_string(), cx.heap) {
        let _ = write!(out, "{:?}", h.str_to_string(s));
        return;
    }
    let _ = write!(out, "{v:?}");
}
