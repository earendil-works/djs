//! Construct temporaries (D2732): the state a construct keeps while its body runs (a
//! loop's iterator record, a switch's discriminant, a destructuring pattern's source, a
//! `using` list's resource Record) lives in a local of its own, not on the operand
//! stack, so a suspended frame holds it in a named slot (D2720, D2721).
//!
//! Each is a binding of kind `VarKind::Temp` with an internal name (the integer atom of
//! the temporary's number in its function, which no source name can be and which needs
//! no interning), reached with
//! `name_get`/`name_put` like any local. Its slot name is `construct@offset`
//! (`for-of.iterator@18`), kept in `FuncState::temps`. A suspension map lists a
//! temporary at a site only where it holds a value the code still reads: [`live_at`]
//! computes that from the final phase-1 code, so a construct's code order (a for-of's
//! target runs after its right side) cannot make the map wrong.

use super::func::VarKind;
use super::parser::Parser;
use crate::heap::Atom;

impl Parser<'_, '_> {
    /// A new temporary of `construct` (`"for-of.iterator"`) at source offset `at`
    /// (absolute; its name is relative to the function's start), declared in the
    /// current scope.
    #[inline(never)]
    pub(super) fn temp(&mut self, construct: &'static str, at: u32) -> Atom {
        // An integer atom: no source name is one, and it needs no interning.
        let n = self.f.temps.len() as u32;
        let atom = Atom::int(n).unwrap_or(Atom::NULL);
        let v = self.f.add_var(atom, VarKind::Temp, self.scope);
        self.f
            .temps
            .push((v, construct, at.saturating_sub(self.f.src_start)));
        atom
    }

    /// Pushes temporary `t`.
    pub(super) fn temp_get(&mut self, t: Atom) {
        self.temp_op(t, false);
    }

    /// Pops into temporary `t`.
    pub(super) fn temp_put(&mut self, t: Atom) {
        self.temp_op(t, true);
    }

    #[inline(never)]
    fn temp_op(&mut self, t: Atom, put: bool) {
        let r = self.name_ref(t, 0);
        if put {
            self.code().name_put(r);
        } else {
            self.code().name_get(r);
        }
    }
}

#[cfg(feature = "dump")]
pub(super) use live::live_at;

#[cfg(feature = "dump")]
mod live {
    use alloc::vec;
    use alloc::vec::Vec;

    use crate::bytecode::Insn;
    use crate::bytecode::ops::{Op, flags};

    use super::super::func::HandlerLabels;

    /// For each instruction index `k` of `sites` in `all` (phase-1 code after the
    /// peephole pass), which of `n` temporaries hold a value some path from there reads
    /// before writing it (live after the instruction, the exception edges of the
    /// handlers included: from each instruction that can throw in a handler's region). `temp_of(r)` is the temporary name reference `r` reaches.
    /// A `gosub` continues into its subroutine, and each `ret` at the instructions after
    /// the `gosub`s to the subroutines it ends.
    pub fn live_at(
        all: &[Insn],
        handlers: &[HandlerLabels],
        n: usize,
        temp_of: impl Fn(u32) -> Option<usize>,
        sites: &[usize],
    ) -> Vec<Vec<bool>> {
        let words = n.div_ceil(64).max(1);
        let len = all.len();
        // Label id -> instruction index.
        let mut label_at: Vec<usize> = Vec::new();
        for (k, i) in all.iter().enumerate() {
            if i.op == Op::Label {
                let l = i.imm(0) as usize;
                if label_at.len() <= l {
                    label_at.resize(l + 1, usize::MAX);
                }
                label_at[l] = k;
            }
        }
        let at = |l: u32| label_at.get(l as usize).copied().unwrap_or(usize::MAX);
        let regions: Vec<(usize, usize, usize)> = handlers
            .iter()
            .map(|h| (at(h.start.0), at(h.end.0), at(h.target.0)))
            .collect();
        let target = |i: &Insn| {
            let imms = i.info().imms;
            match imms.last() {
                Some(k) if k.is_target() => Some(at(i.imm(imms.len() - 1))),
                _ => None,
            }
        };
        // Each subroutine's `ret`s: the ones reachable from its entry without entering a
        // nested subroutine or a handler. A `ret` returns after every `gosub` to them.
        let mut ret_to: Vec<Vec<usize>> = vec![Vec::new(); len];
        let gosubs: Vec<(usize, usize)> = all
            .iter()
            .enumerate()
            .filter(|(_, i)| i.info().has(flags::GOSUB))
            .filter_map(|(k, i)| target(i).map(|t| (k, t)))
            .collect();
        let mut entries: Vec<usize> = gosubs.iter().map(|g| g.1).collect();
        entries.sort_unstable();
        entries.dedup();
        let mut seen = vec![false; len];
        for &e in &entries {
            seen.iter_mut().for_each(|x| *x = false);
            let mut work = vec![e];
            while let Some(k) = work.pop() {
                if k >= len || seen[k] {
                    continue;
                }
                seen[k] = true;
                let i = &all[k];
                let info = i.info();
                if i.op == Op::Ret {
                    for &(g, t) in &gosubs {
                        if t == e {
                            ret_to[k].push(g + 1);
                        }
                    }
                    continue;
                }
                if info.has(flags::JUMP) {
                    work.extend(target(i));
                } else if info.has(flags::BRANCH) {
                    work.extend(target(i));
                    work.push(k + 1);
                } else if !info.has(flags::TERM) {
                    work.push(k + 1);
                }
            }
        }
        let mut succ: Vec<Vec<usize>> = Vec::with_capacity(len);
        let mut uses = vec![0u64; len * words];
        let mut defs = vec![0u64; len * words];
        for (k, i) in all.iter().enumerate() {
            let info = i.info();
            let mut s = Vec::new();
            if info.has(flags::JUMP) || info.has(flags::GOSUB) {
                s.extend(target(i));
            } else if info.has(flags::BRANCH) {
                s.extend(target(i));
                s.push(k + 1);
            } else if i.op == Op::Ret {
                s.extend_from_slice(&ret_to[k]);
            } else if !info.has(flags::TERM) {
                s.push(k + 1);
            }
            // (Only instructions that can throw reach a handler.)
            if info.has(flags::THROWS) {
                for &(a, b, t) in &regions {
                    if a <= k && k < b {
                        s.push(t);
                    }
                }
            }
            s.retain(|&x| x < len);
            succ.push(s);
            let bit = |r: u32| temp_of(r).map(|t| (k * words + t / 64, 1u64 << (t % 64)));
            match i.op {
                Op::NameGet | Op::NameTypeof => {
                    if let Some((w, b)) = bit(i.imm(0)) {
                        uses[w] |= b;
                    }
                }
                Op::NamePut | Op::NameSet | Op::NameInit => {
                    if let Some((w, b)) = bit(i.imm(0)) {
                        defs[w] |= b;
                    }
                }
                _ => {}
            }
        }
        // live_in[k] = uses[k] | (live_out[k] & !defs[k]); live_out[k] = OR of the
        // successors' live_in. Backward passes until nothing changes.
        let mut live_in = vec![0u64; len * words];
        let mut out = vec![0u64; words];
        loop {
            let mut changed = false;
            for k in (0..len).rev() {
                out.iter_mut().for_each(|w| *w = 0);
                for &x in &succ[k] {
                    for w in 0..words {
                        out[w] |= live_in[x * words + w];
                    }
                }
                for w in 0..words {
                    let v = uses[k * words + w] | (out[w] & !defs[k * words + w]);
                    if v != live_in[k * words + w] {
                        live_in[k * words + w] = v;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        sites
            .iter()
            .map(|&k| {
                let mut o = vec![0u64; words];
                for &x in succ.get(k).map(|s| s.as_slice()).unwrap_or_default() {
                    for w in 0..words {
                        o[w] |= live_in[x * words + w];
                    }
                }
                (0..n).map(|t| o[t / 64] & (1 << (t % 64)) != 0).collect()
            })
            .collect()
    }
}
