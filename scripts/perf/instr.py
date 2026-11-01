#!/usr/bin/env python3
"""Makes an instrumented copy of the engine in $DJS_PERF_DIR/instr/src (never the
worktree itself) and builds it: counts executed bytecodes (fast_step vs step, user vs prelude), intrinsic
calls by id and caller kind, native state-arm executions and native_step entries per
native, frames pushed, allocations by cell kind, collections. DJS_PROF=1 prints them."""
import os, re, shutil, subprocess, sys
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(here, exist_ok=True)
dst = os.path.join(here, "instr/src")
subprocess.run(["rsync", "-a", "--delete", "--exclude", "target", "--exclude", "work", "--exclude", "target-lto",
                "--exclude", "vendor", "--exclude", "node_modules", "--exclude", ".git",
                root + "/", dst + "/"], check=True)

def patch(rel, old, new, count=1):
    p = os.path.join(dst, rel)
    s = open(p).read()
    n = s.count(old)
    if n != count:
        sys.exit(f"{rel}: expected {count} matches of {old!r}, found {n}")
    open(p, "w").write(s.replace(old, new))

PROF = r'''
pub mod prof {
    //! 0081 instrumentation (scratch copy only).
    use core::sync::atomic::{AtomicU64, Ordering::Relaxed};
    pub static FAST: [[AtomicU64; 256]; 2] = [const { [const { AtomicU64::new(0) }; 256] }; 2];
    pub static SLOW: [[AtomicU64; 256]; 2] = [const { [const { AtomicU64::new(0) }; 256] }; 2];
    // call_intrinsic by id: [0] from bytecode, [1] from native code (n_leaf/n_call)
    pub static INTR: [[AtomicU64; 1024]; 2] = [const { [const { AtomicU64::new(0) }; 1024] }; 2];
    pub static NSTEP: [AtomicU64; 256] = [const { AtomicU64::new(0) }; 256];
    pub static NARM: [AtomicU64; 256] = [const { AtomicU64::new(0) }; 256];
    pub static ALLOC_N: [AtomicU64; 64] = [const { AtomicU64::new(0) }; 64];
    pub static ALLOC_B: [AtomicU64; 64] = [const { AtomicU64::new(0) }; 64];
    pub static FRAMES: AtomicU64 = AtomicU64::new(0);
    pub static NFRAMES: AtomicU64 = AtomicU64::new(0);
    pub static GCS: AtomicU64 = AtomicU64::new(0);
    pub static GC_BYTES: AtomicU64 = AtomicU64::new(0);
    pub static IN_NATIVE: AtomicU64 = AtomicU64::new(0);
    pub static FN_CALLS: [AtomicU64; 65536] = [const { AtomicU64::new(0) }; 65536];
    pub static FN_OPS: [AtomicU64; 65536] = [const { AtomicU64::new(0) }; 65536];
    pub static mut NAMES: alloc::collections::BTreeMap<u32, alloc::string::String> = alloc::collections::BTreeMap::new();
    #[allow(static_mut_refs)]
    pub(crate) fn fn_call(heap: &crate::heap::Heap, t: u32) {
        if !on() { return; }
        let a: u32 = heap.arena().get(t, crate::heap::cells::FuncTemplate::NAME);
        FN_CALLS[(a & 0xffff) as usize].fetch_add(1, Relaxed);
        unsafe { if !NAMES.contains_key(&a) { NAMES.insert(a, heap.atom_name(crate::heap::Atom::from_id(a))); } }
    }
    pub static ENABLED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
    #[inline(always)]
    pub fn on() -> bool { ENABLED.load(Relaxed) }
    #[inline(always)]
    pub fn inc(a: &AtomicU64, n: u64) { if on() { a.fetch_add(n, Relaxed); } }
    pub fn reset() { ENABLED.store(true, Relaxed); }
    #[inline(always)]
    pub(crate) fn kind(ex: &super::Exec<'_>) -> usize {
        if !on() || ex.pc >= ex.code_len { return 2; }
        use crate::heap::cells::FuncTemplate;
        let script: crate::heap::Ref = ex.heap.arena().get(ex.template, FuncTemplate::SCRIPT);
        let prelude = super::intrinsic(ex.heap, super::Intrinsic::PreludeScript).as_internal();
        let a: u32 = ex.heap.arena().get(ex.template, FuncTemplate::NAME);
        FN_OPS[(a & 0xffff) as usize].fetch_add(1, Relaxed);
        (prelude == script.get()) as usize
    }
    #[inline(always)]
    pub(crate) fn count(k: usize, fast: bool, b: u8) {
        if k > 1 { return; }
        if fast { FAST[k][b as usize].fetch_add(1, Relaxed); } else { SLOW[k][b as usize].fetch_add(1, Relaxed); }
    }
    pub fn dump() -> alloc::string::String {
        use alloc::format;
        use alloc::vec::Vec;
        let mut out = alloc::string::String::new();
        let mut rows = Vec::new();
        let mut tot = [0u64; 2];
        let mut slow = [0u64; 2];
        for k in 0..2 { for b in 0..256 {
            let f = FAST[k][b].load(Relaxed); let s = SLOW[k][b].load(Relaxed);
            tot[k] += f + s; slow[k] += s;
            if f + s > 0 { rows.push((f + s, s, k, b)); }
        } }
        rows.sort_by(|a, b| b.0.cmp(&a.0));
        let all = (tot[0] + tot[1]).max(1);
        out += &format!("bytecodes user {} prelude {} (slow path: user {} prelude {})\n", tot[0], tot[1], slow[0], slow[1]);
        out += &format!("frames: bytecode {} native {}; native_step entries {}\n", FRAMES.load(Relaxed), NFRAMES.load(Relaxed), NSTEP.iter().map(|a| a.load(Relaxed)).sum::<u64>());
        for (n, s, k, b) in rows.iter().take(45) {
            let name = crate::bytecode::Op::from_u8(*b as u8).map(|o| crate::bytecode::OPS[o as usize].name).unwrap_or("?");
            out += &format!("  {:5.1}% {:>11} slow {:5.1}% {} {}\n", 100.0 * *n as f64 / all as f64, n, 100.0 * *s as f64 / *n as f64, if *k == 1 { "prelude" } else { "user   " }, name);
        }
        let mut ir = Vec::new();
        for k in 0..2 { for i in 0..1024 { let n = INTR[k][i].load(Relaxed); if n > 0 { ir.push((n, k, i)); } } }
        ir.sort_by(|a, b| b.0.cmp(&a.0));
        out += &format!("call_intrinsic: from bytecode {} from native {}\n", ir.iter().filter(|r| r.1 == 0).map(|r| r.0).sum::<u64>(), ir.iter().filter(|r| r.1 == 1).map(|r| r.0).sum::<u64>());
        for (n, k, i) in ir.iter().take(30) {
            let name = crate::bytecode::intrinsics::INTRINSICS.get(*i).map(|x| x.name).unwrap_or("?");
            out += &format!("  {:>11} {} {}\n", n, if *k == 1 { "native  " } else { "bytecode" }, name);
        }
        let mut nr = Vec::new();
        for i in 0..256 { let a = NARM[i].load(Relaxed); let s = NSTEP[i].load(Relaxed); if a + s > 0 { nr.push((a, s, i)); } }
        nr.sort_by(|a, b| b.0.cmp(&a.0));
        out += "natives (state arms executed, native_step entries):\n";
        for (a, s, i) in nr.iter().take(20) {
            let name = crate::builtins::r#gen::natives::NATIVES.get(*i).map(|x| x.name).unwrap_or("?");
            out += &format!("  {:>11} {:>11} {}\n", a, s, name);
        }
        let mut fr = Vec::new();
        #[allow(static_mut_refs)]
        for (a, name) in unsafe { NAMES.iter() } {
            let i = (*a & 0xffff) as usize;
            fr.push((FN_OPS[i].load(Relaxed), FN_CALLS[i].load(Relaxed), name.clone()));
        }
        fr.sort_by(|a, b| b.0.cmp(&a.0));
        out += "functions (bytecodes executed, calls, bytecodes per call):\n";
        for (o, c, n) in fr.iter().take(25) {
            out += &format!("  {:>11} {:>10} {:>7.1} {}\n", o, c, *o as f64 / (*c).max(1) as f64, n);
        }
        let mut al = Vec::new();
        let (mut an, mut ab) = (0, 0);
        for i in 0..64 { let n = ALLOC_N[i].load(Relaxed); let b = ALLOC_B[i].load(Relaxed); an += n; ab += b; if n > 0 { al.push((b, n, i)); } }
        al.sort_by(|a, b| b.0.cmp(&a.0));
        out += &format!("allocations: {} cells, {} bytes; collections {} ({} bytes live after)\n", an, ab, GCS.load(Relaxed), GC_BYTES.load(Relaxed));
        for (b, n, i) in al.iter().take(14) {
            let name = crate::heap::CellKind::from_u8(*i as u8).map(|k| format!("{k:?}")).unwrap_or_default();
            out += &format!("  {:>11} cells {:>12} bytes {}\n", n, b, name);
        }
        out
    }
}
'''

R = "crates/djs-core/src/interp/run.rs"
patch(R, "pub(crate) struct Exec<'h> {", PROF + "\npub(crate) struct Exec<'h> {")
patch(R, "            if self.fast_step() || self.fast_more() {\n                continue;\n            }",
      "            let prof_k = prof::kind(self);\n"
      "            let prof_b = if prof_k < 2 { self.byte(self.pc) } else { 0 };\n"
      "            if self.fast_step() || self.fast_more() {\n                prof::count(prof_k, true, prof_b);\n                continue;\n            }\n"
      "            prof::count(prof_k, false, prof_b);")
patch(R, "    fn push_frame(&mut self, t: u32, base: u32, argc: u32, cont: Continuation, ret_pc: u32) {\n",
      "    fn push_frame(&mut self, t: u32, base: u32, argc: u32, cont: Continuation, ret_pc: u32) {\n        prof::inc(&prof::FRAMES, 1);\n        prof::fn_call(self.heap, t);\n")
N = "crates/djs-core/src/interp/run/native.rs"
patch(N, "        let idx = self.heap.arena().get(t, FuncTemplate::NATIVE);\n        let Some(nf)",
      "        let idx = self.heap.arena().get(t, FuncTemplate::NATIVE);\n        super::prof::inc(&super::prof::NFRAMES, 1);\n        super::prof::fn_call(self.heap, t);\n        let Some(nf)")
patch(N, "        let run = self.native_fn().run;\n",
      "        let run = self.native_fn().run;\n        super::prof::inc(&super::prof::NSTEP[((self.native - 1) & 255) as usize], 1);\n")
patch(N, "    pub(crate) fn n_state(&self) -> u32 {\n        self.pc\n",
      "    pub(crate) fn n_state(&self) -> u32 {\n        super::prof::inc(&super::prof::NARM[((self.native.wrapping_sub(1)) & 255) as usize], 1);\n        self.pc\n")
I = "crates/djs-core/src/interp/run/intrinsics.rs"
patch(I, "    pub(super) fn call_intrinsic(&mut self, id: u16, argc: u32, next: u32) -> Step {\n",
      "    pub(super) fn call_intrinsic(&mut self, id: u16, argc: u32, next: u32) -> Step {\n        super::prof::inc(&super::prof::INTR[(self.native != 0) as usize][(id as usize).min(1023)], 1);\n")
H = "crates/djs-core/src/heap/mod.rs"
for fn in ["alloc_cell(&mut self, kind: CellKind, size: u32)", "alloc_cell_init(", "alloc_cell_from_iter("]:
    pass
patch(H, "        let size = align8(size.max(kind.layout().min_size));\n",
      "        let size = align8(size.max(kind.layout().min_size));\n"
      "        crate::interp::run::prof::inc(&crate::interp::run::prof::ALLOC_N[kind as usize & 63], 1);\n"
      "        crate::interp::run::prof::inc(&crate::interp::run::prof::ALLOC_B[kind as usize & 63], size as u64);\n", count=3)
G = "crates/djs-core/src/heap/gc.rs"
patch(G, "        Ok(GcStats { before, after })",
      "        crate::interp::run::prof::inc(&crate::interp::run::prof::GCS, 1);\n"
      "        crate::interp::run::prof::inc(&crate::interp::run::prof::GC_BYTES, after as u64);\n        Ok(GcStats { before, after })")
patch("crates/djs-core/src/interp/mod.rs", "pub(crate) mod run;", "pub mod run;")
C = "crates/djs-cli/src/run.rs"
patch(C, "    vm.call(main, this, args).expect(\"the VM is idle\");\n",
      "    vm.call(main, this, args).expect(\"the VM is idle\");\n    if std::env::var_os(\"DJS_PROF\").is_some() { djs_core::interp::run::prof::reset(); }\n")
patch(C, "        match vm.run(fuel) {\n",
      "        let st = vm.run(fuel);\n        if std::env::var_os(\"DJS_PROF\").is_some() && matches!(st, Status::Done(_) | Status::Idle) { eprint!(\"{}\", djs_core::interp::run::prof::dump()); }\n        match st {\n")
env = dict(os.environ, CARGO_TARGET_DIR=os.path.join(here, "instr/target"), CARGO_PROFILE_RELEASE_CODEGEN_UNITS="4")
subprocess.run(["cargo", "build", "--release", "-p", "djs-cli"], cwd=dst, env=env, check=True)
print("built", os.path.join(here, "instr/target/release/djs"), "(DJS_PROF=1 djs run --body x.js prints the counts)")
