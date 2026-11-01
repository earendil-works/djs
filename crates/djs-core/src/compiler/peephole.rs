//! Peepholes on phase-1 code (0086), before the resolution re-emit.
//!
//! The single-pass emitter produces the general form of every construct; an expression
//! statement drops the value its expression keeps (`x = v;` is `name_set x; drop`,
//! `o.c = v;` is `insert2; put_field; drop`, `i++;` is `to_numeric; dup; inc; name_put;
//! drop`). The rules here remove what the drop makes useless. They match consecutive
//! instructions only: phase-1 labels are `label` instructions, so a jump target never
//! sits inside a match. Resolution-dependent fusions (`inc_loc`) are in the re-emit
//! (resolve.rs).
//!
//! Each kept instruction keeps its phase-1 `pc`, so the line table entries of removed
//! instructions apply to the next kept one, as they would have.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::bytecode::{Insn, Label, Op};

/// Applies the rules until none matches.
pub(super) fn simplify(mut code: Vec<Insn>) -> Vec<Insn> {
    // Cycles of jumps could retarget each other forever: a few passes are enough for
    // everything else.
    for _ in 0..16 {
        let mut changed = thread_jumps(&mut code);
        let mut out = Vec::with_capacity(code.len());
        let mut i = 0;
        while i < code.len() {
            match rewrite(&code[i..], &mut out) {
                Some(n) => {
                    i += n;
                    changed = true;
                }
                None => {
                    out.push(code[i]);
                    i += 1;
                }
            }
        }
        code = out;
        if !changed {
            break;
        }
    }
    code
}

/// Jumps to a `jump` go to its target instead (`if (x) break;` in a loop, `continue`
/// to the update of a loop).
fn thread_jumps(code: &mut [Insn]) -> bool {
    let mut changed = false;
    // The first instruction after each label, if it is a `jump`: its target.
    let mut next_jump: BTreeMap<u32, Label> = BTreeMap::new();
    let mut pending: Vec<u32> = Vec::new();
    for insn in code.iter() {
        if insn.op == Op::Label {
            pending.push(insn.imm(0));
            continue;
        }
        if insn.op == Op::Jump
            && let Some(t) = insn.label()
        {
            for &l in &pending {
                next_jump.insert(l, t);
            }
        }
        pending.clear();
    }
    for insn in code.iter_mut() {
        if !matches!(insn.op, Op::Jump | Op::JumpIfFalse | Op::JumpIfTrue) {
            continue;
        }
        let Some(mut t) = insn.label() else { continue };
        // A cycle of jumps (`for (;;) {}` after rotation) stops after a few steps.
        for _ in 0..8 {
            match next_jump.get(&t.0) {
                Some(&n) if n != t => t = n,
                _ => break,
            }
        }
        if insn.label() != Some(t) {
            *insn = insn.with_label(t);
            changed = true;
        }
    }
    changed
}

/// The store of a reference with `refs` values under the stored value, as `emit_store`
/// emits it (compiler/expr.rs), with the `insert` that `emit_keep` puts before it.
fn keep_insert(store: Op) -> Option<Op> {
    Some(match store {
        Op::PutField => Op::Insert2,
        Op::PutElem | Op::PutPrivateField | Op::PutSuperField => Op::Insert3,
        Op::PutSuperElem => Op::Insert4,
        _ => return None,
    })
}

/// Whether `code[0]`, a jump, targets one of the labels right after it.
fn jumps_to_next(code: &[Insn]) -> bool {
    let Some(t) = code[0].label() else {
        return false;
    };
    code[1..]
        .iter()
        .take_while(|i| i.op == Op::Label)
        .any(|i| i.imm(0) == t.0)
}

/// If a rule matches at the start of `code`, pushes its replacement and returns the
/// number of instructions it consumed.
fn rewrite(code: &[Insn], out: &mut Vec<Insn>) -> Option<usize> {
    let op = |k: usize| code.get(k).map(|i| i.op);
    let incdec = |o: Option<Op>| matches!(o, Some(Op::Inc | Op::Dec));
    match (op(0)?, op(1)) {
        // `x = v;`: store without keeping.
        (Op::NameSet, Some(Op::Drop)) => {
            out.push(code[0].with_op(Op::NamePut));
            Some(2)
        }
        // `o.c = v;`, `o.c += v;`, `o.c++;` (with the update between): the copy under
        // the reference is what the drop removes.
        (ins, _) if matches!(ins, Op::Insert2 | Op::Insert3 | Op::Insert4) => {
            let update = usize::from(matches!(op(1), Some(Op::Inc | Op::Dec | Op::ToNumeric)));
            let store = op(1 + update)?;
            if keep_insert(store) != Some(ins) || op(2 + update) != Some(Op::Drop) {
                return None;
            }
            out.extend_from_slice(&code[1..2 + update]);
            Some(3 + update)
        }
        // `x++;`: the old value under the new one is what the drop removes.
        (Op::Dup, o) if incdec(o) && op(2) == Some(Op::NamePut) && op(3) == Some(Op::Drop) => {
            out.extend_from_slice(&code[1..3]);
            Some(4)
        }
        // `inc` and `dec` take ToNumeric themselves.
        (Op::ToNumeric, o) if incdec(o) => {
            out.push(code[1]);
            Some(2)
        }
        // `if (c) break;`: `jump_if_false L; jump M; L:` is `jump_if_true M; L:`.
        (Op::JumpIfFalse | Op::JumpIfTrue, Some(Op::Jump))
            if op(2) == Some(Op::Label) && code[2].imm(0) == code[0].label()?.0 =>
        {
            let inv = if code[0].op == Op::JumpIfFalse {
                Op::JumpIfTrue
            } else {
                Op::JumpIfFalse
            };
            out.push(code[0].with_op(inv).with_label(code[1].label()?));
            Some(2)
        }
        // A jump to the next instruction.
        (Op::Jump, _) if jumps_to_next(code) => Some(1),
        // `if (!x)`
        (Op::Not, Some(Op::JumpIfFalse)) => {
            out.push(code[1].with_op(Op::JumpIfTrue));
            Some(2)
        }
        (Op::Not, Some(Op::JumpIfTrue)) => {
            out.push(code[1].with_op(Op::JumpIfFalse));
            Some(2)
        }
        // `while (true)`, `for (;;)` after loop rotation.
        (Op::PushTrue, Some(Op::JumpIfTrue)) | (Op::PushFalse, Some(Op::JumpIfFalse)) => {
            out.push(code[1].with_op(Op::Jump));
            Some(2)
        }
        (Op::PushTrue, Some(Op::JumpIfFalse)) | (Op::PushFalse, Some(Op::JumpIfTrue)) => Some(2),
        _ => None,
    }
}
