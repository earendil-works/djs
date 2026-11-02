//! Operand roles at an `await`: what each value below the awaited one *is*, named by the
//! instruction that produced it. This stands in for the stack map a compiler would emit
//! (docs/portable-snapshots.md): both builds derive it from their own bytecode, and a
//! transfer matches operands by role, never by position or pc.

use std::collections::HashMap;

use djs_core::bytecode::decode::decode;
use djs_core::bytecode::flags;
use djs_core::heap::Heap;

use crate::heapx::TemplateInfo;

/// The producer label of every operand below the top at `pc` (bottom first).
pub fn labels_at(heap: &Heap, t: &TemplateInfo, at: u32) -> Option<Vec<String>> {
    let code = &t.code;
    let len = code.len() as u32;
    let mut state: HashMap<u32, Vec<String>> = HashMap::new();
    let mut work = vec![0u32];
    state.insert(0, vec![]);
    let mut handler_seeded = vec![false; t.handlers.len()];
    loop {
        while let Some(pc) = work.pop() {
            if pc >= len {
                continue;
            }
            let insn = decode(code, pc).ok()?;
            let info = insn.info();
            let mut st = state[&pc].clone();
            let name = info.name;
            let pops = insn.pops() as usize;
            let pushes = insn.pushes() as usize;
            if st.len() < pops {
                return None;
            }
            match name {
                "dup" => {
                    let a = st.last().cloned()?;
                    st.push(a);
                }
                "dup2" => {
                    let n = st.len();
                    let (a, b) = (st[n - 2].clone(), st[n - 1].clone());
                    st.push(a);
                    st.push(b);
                }
                "swap" => {
                    let n = st.len();
                    st.swap(n - 1, n - 2);
                }
                "nip" => {
                    let b = st.pop()?;
                    st.pop();
                    st.push(b);
                }
                "insert2" | "insert3" | "insert4" => {
                    let k = pops;
                    let top = st.last().cloned()?;
                    let n = st.len();
                    st.insert(n - k, top);
                }
                "rot3l" => {
                    let n = st.len();
                    let x = st.remove(n - 3);
                    st.push(x);
                }
                "rot3r" => {
                    let x = st.pop()?;
                    let n = st.len();
                    st.insert(n - 2, x);
                }
                "pick" => {
                    let k = insn.imm(0) as usize;
                    let n = st.len();
                    let v = st[n - 1 - k].clone();
                    st.push(v);
                }
                _ => {
                    let below: Vec<String> = st.split_off(st.len() - pops);
                    let label = producer(heap, &insn, info.name, &below);
                    match (name, pushes) {
                        ("get_method", 2) => {
                            st.push(format!("callee:{}", atom_imm(heap, &insn)));
                            st.push("this".into());
                        }
                        _ => {
                            for _ in 0..pushes {
                                st.push(label.clone());
                            }
                        }
                    }
                }
            }
            if pc == at {
                let mut s = state[&pc].clone();
                s.pop();
                return Some(s);
            }
            let mut succ = vec![];
            if info.has(flags::GOSUB) {
                let mut s2 = st.clone();
                s2.push("finally.return-address".into());
                if let Some(tg) = insn.jump_target() {
                    reach(&mut state, &mut work, tg, s2);
                }
                let before = state[&pc].clone();
                reach(&mut state, &mut work, insn.next_pc(), before);
                continue;
            }
            if (info.has(flags::JUMP) || info.has(flags::BRANCH))
                && let Some(tg) = insn.jump_target()
            {
                succ.push(tg);
            }
            if !info.no_fallthrough() {
                succ.push(insn.next_pc());
            }
            for s in succ {
                reach(&mut state, &mut work, s, st.clone());
            }
        }
        // Handlers: entered with the stack of their region's start, cut to the
        // handler's depth, plus the exception.
        let mut seeded = false;
        for (i, h) in t.handlers.iter().enumerate() {
            if handler_seeded[i] {
                continue;
            }
            if let Some(s) = state.get(&h.start) {
                let mut s = s.clone();
                s.truncate(h.depth as usize);
                s.push("exception".into());
                handler_seeded[i] = true;
                reach(&mut state, &mut work, h.target, s);
                seeded = true;
            }
        }
        if !seeded {
            return None;
        }
    }
}

fn reach(state: &mut HashMap<u32, Vec<String>>, work: &mut Vec<u32>, pc: u32, s: Vec<String>) {
    if let Some(old) = state.get_mut(&pc) {
        // A join: keep the labels both paths agree on.
        for (a, b) in old.iter_mut().zip(s.iter()) {
            if a != b {
                *a = "merged".into();
            }
        }
        return;
    }
    state.insert(pc, s);
    work.push(pc);
}

fn atom_imm(heap: &Heap, insn: &djs_core::bytecode::Insn) -> String {
    heap.atom_name(insn.atom(0))
}

fn producer(heap: &Heap, insn: &djs_core::bytecode::Insn, name: &str, below: &[String]) -> String {
    match name {
        "call_intrinsic" => {
            let id = insn.imm(0) as usize;
            let n = djs_core::bytecode::intrinsics::INTRINSICS
                .get(id)
                .map(|i| i.name)
                .unwrap_or("?");
            match n {
                "GetIterator" => "iterator".into(),
                "GetAsyncIterator" => "async-iterator".into(),
                _ => format!("intrinsic:{n}"),
            }
        }
        "get_field" if below.last().is_some_and(|b| b.ends_with("iterator")) => {
            format!("{}.{}", below.last().unwrap(), heap.atom_name(insn.atom(0)))
        }
        "object" => "object-literal".into(),
        "array_from" | "array" => "array-literal".into(),
        "to_string" => "template-part".into(),
        "define_field" | "put_array_el" | "append" => below.first().cloned().unwrap_or_default(),
        _ => "value".into(),
    }
}
