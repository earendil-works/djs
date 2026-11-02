#!/usr/bin/env python3
"""instr.py [--self-test]: an instrumented copy of the engine (0081, 0155).

Copies the checkout to $DJS_PERF_DIR/instr/src (never the worktree itself), patches the
copy and builds its djs CLI. `DJS_PROF=1 djs run --body x.js` then prints to stderr:
- bytecodes executed, by kind (user code / bytecode builtins) and path (run_hot, step),
  per opcode and per function;
- a time split from a sampling thread: user bytecode, bytecode builtins (with the leaf
  intrinsics they call), native (`@native`) builtins, collections;
- intrinsic calls by id and caller, native state arms, frames, allocations by cell kind,
  collections.

Patches are anchored on exact source text that must occur a given number of times. A
refactor that moves an anchor fails the run with every unmatched anchor listed (and the
lines that look like where it went), never a silently missing counter. `--self-test`
builds, runs a script that exercises each counter and fails if any reads zero."""
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

failures = []

def patch(rel, anchor, text, where="after", count=1):
    """Inserts `text` after or before each occurrence of `anchor` in `rel`, or replaces
    it (`where`). `anchor` must occur exactly `count` times (`None`: at least once)."""
    p = os.path.join(dst, rel)
    s = open(p).read()
    n = s.count(anchor)
    if n != count and not (count is None and n > 0):
        # Where it may have gone: the lines sharing most of the anchor's identifiers.
        words = set(re.findall(r"[A-Za-z_][A-Za-z0-9_]{2,}", anchor))
        scored = sorted(((sum(w in l for w in words), i) for i, l in enumerate(s.splitlines())), reverse=True)
        lines = s.splitlines()
        near = [f"    {i + 1}: {lines[i].strip()}" for n, i in scored[:5] if n * 2 >= len(words) > 0]
        failures.append(f"{rel}: anchor found {n} times, expected {count}:\n  {anchor.strip()!r}\n"
                        + ("\n".join(near) or "    (no similar lines)"))
        return
    new = {"after": anchor + text, "before": text + anchor, "replace": text}[where]
    open(p, "w").write(s.replace(anchor, new))

PROF = r'''//! 0081/0155 instrumentation (scratch copy only, scripts/perf/instr.py).
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};
use crate::heap::cells::FuncTemplate;
use crate::bytecode::template::func_flags;

/// Bytecodes by [kind: 0 user, 1 builtin][path: 0 run_hot, 1 step][opcode].
pub static OPS: [[[AtomicU64; 256]; 2]; 2] = [const { [const { [const { AtomicU64::new(0) }; 256] }; 2] }; 2];
/// call_intrinsic by [0 from bytecode, 1 from native code][id].
pub static INTR: [[AtomicU64; 2048]; 2] = [const { [const { AtomicU64::new(0) }; 2048] }; 2];
pub static NSTEP: [AtomicU64; 256] = [const { AtomicU64::new(0) }; 256];
pub static NARM: [AtomicU64; 256] = [const { AtomicU64::new(0) }; 256];
pub static ALLOC_N: [AtomicU64; 64] = [const { AtomicU64::new(0) }; 64];
pub static ALLOC_B: [AtomicU64; 64] = [const { AtomicU64::new(0) }; 64];
/// Frames pushed by `call` ([0 user, 1 builtin]) and native frames.
pub static FRAMES: [AtomicU64; 2] = [const { AtomicU64::new(0) }; 2];
pub static NFRAMES: AtomicU64 = AtomicU64::new(0);
pub static GCS: AtomicU64 = AtomicU64::new(0);
pub static GC_BYTES: AtomicU64 = AtomicU64::new(0);
/// Per function: bytecodes, entries (pc 0) and the key, in a table hashed by the key: a user
/// function's name atom (its template moves), a builtin's template offset (in the
/// builtin prefix, which never moves; builtins keep no names, D2253). Rare collisions
/// merge two rows.
pub static FN_OPS: [AtomicU64; 65536] = [const { AtomicU64::new(0) }; 65536];
pub static FN_ENTRIES: [AtomicU64; 65536] = [const { AtomicU64::new(0) }; 65536];
pub static FN_KEY: [AtomicU32; 65536] = [const { AtomicU32::new(0) }; 65536];
pub static FN_KIND: [AtomicU32; 65536] = [const { AtomicU32::new(0) }; 65536];
/// What runs now, for the sampler: 0 user bytecode, 1 builtin bytecode, 2 native builtin,
/// 3 collection, 4 outside `run`.
pub static CUR: AtomicU32 = AtomicU32::new(4);
pub static SAMPLES: [AtomicU64; 5] = [const { AtomicU64::new(0) }; 5];
pub static ENABLED: AtomicBool = AtomicBool::new(false);
#[inline(always)]
pub fn on() -> bool { ENABLED.load(Relaxed) }
#[inline(always)]
pub fn inc(a: &AtomicU64, n: u64) { if on() { a.fetch_add(n, Relaxed); } }
pub fn reset() { ENABLED.store(true, Relaxed); }

fn kind_flags(flags: u32) -> usize { (flags & func_flags::PRIVILEGED != 0) as usize }

#[inline(always)]
fn count(kind: usize, path: usize, op: u8, template: u32, name: u32, pc: u32) {
    if !on() { return; }
    CUR.store(kind as u32, Relaxed);
    OPS[kind][path][op as usize].fetch_add(1, Relaxed);
    let key = if kind == 1 { template } else { name };
    let i = ((key.wrapping_mul(0x9E37_79B1) >> 16) ^ kind as u32) as usize & 0xffff;
    FN_OPS[i].fetch_add(1, Relaxed);
    if pc == 0 { FN_ENTRIES[i].fetch_add(1, Relaxed); }
    FN_KEY[i].store(key, Relaxed);
    FN_KIND[i].store(kind as u32, Relaxed);
}

/// A bytecode `run_hot` executes, read from its byte slice.
#[inline(always)]
pub(crate) fn hot(b: &[u8], template: u32, op: u8, pc: u32) {
    if !on() { return; }
    let rd = |o: u32| { let o = o as usize; u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) };
    let flags = rd(template + FuncTemplate::FLAGS.offset);
    count(kind_flags(flags), 0, op, template, rd(template + FuncTemplate::NAME.offset), pc);
}

/// An instruction `step_once` executes (a native step when `op` is None).
pub(crate) fn step(heap: &crate::heap::Heap, template: u32, op: Option<u8>, pc: u32) {
    if !on() { return; }
    let Some(op) = op else { CUR.store(2, Relaxed); return; };
    let flags: u32 = heap.arena().get(template, FuncTemplate::FLAGS);
    let name: u32 = heap.arena().get(template, FuncTemplate::NAME);
    count(kind_flags(flags), 1, op, template, name, pc);
}

pub(crate) fn frame(heap: &crate::heap::Heap, template: u32) {
    if !on() { return; }
    let flags: u32 = heap.arena().get(template, FuncTemplate::FLAGS);
    FRAMES[kind_flags(flags)].fetch_add(1, Relaxed);
}

/// Marks a collection for the sampler while it lives.
pub struct Gc(u32);
impl Gc {
    pub fn start() -> Gc { Gc(CUR.swap(3, Relaxed)) }
}
impl Drop for Gc {
    fn drop(&mut self) { CUR.store(self.0, Relaxed); }
}

pub fn dump(heap: &crate::heap::Heap) -> alloc::string::String {
    use alloc::format;
    use alloc::vec::Vec;
    let mut out = alloc::string::String::new();
    let g = |k: usize, p: usize| (0..256).map(|b| OPS[k][p][b].load(Relaxed)).sum::<u64>();
    let (uh, us, bh, bs) = (g(0, 0), g(0, 1), g(1, 0), g(1, 1));
    let all = (uh + us + bh + bs).max(1);
    let pct = |n: u64, d: u64| 100.0 * n as f64 / d.max(1) as f64;
    out += &format!("bytecodes {all}: user {} ({:.1}%, step {:.1}%), builtins {} ({:.1}%, step {:.1}%)\n",
        uh + us, pct(uh + us, all), pct(us, uh + us), bh + bs, pct(bh + bs, all), pct(bs, bh + bs));
    let s: Vec<u64> = SAMPLES.iter().map(|a| a.load(Relaxed)).collect();
    let st = s.iter().sum::<u64>();
    out += &format!("time ({st} samples): user bytecode {:.1}%, bytecode builtins {:.1}%, native builtins {:.1}%, collections {:.1}%, outside run {:.1}%\n",
        pct(s[0], st), pct(s[1], st), pct(s[2], st), pct(s[3], st), pct(s[4], st));
    out += &format!("frames: user {} builtin {} native {}; native_step entries {}\n",
        FRAMES[0].load(Relaxed), FRAMES[1].load(Relaxed), NFRAMES.load(Relaxed),
        NSTEP.iter().map(|a| a.load(Relaxed)).sum::<u64>());
    let mut rows = Vec::new();
    for k in 0..2 { for b in 0..256 {
        let (h, st) = (OPS[k][0][b].load(Relaxed), OPS[k][1][b].load(Relaxed));
        if h + st > 0 { rows.push((h + st, st, k, b)); }
    } }
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    out += "opcodes (share, count, through step, kind):\n";
    for (n, st, k, b) in rows.iter().take(40) {
        let name = crate::bytecode::Op::from_u8(*b as u8).map(|o| crate::bytecode::OPS[o as usize].name).unwrap_or("?");
        out += &format!("  {:5.1}% {:>11} step {:5.1}% {} {}\n", pct(*n, all), n, pct(*st, *n), if *k == 1 { "builtin" } else { "user   " }, name);
    }
    let mut ir = Vec::new();
    for k in 0..2 { for i in 0..2048 { let n = INTR[k][i].load(Relaxed); if n > 0 { ir.push((n, k, i)); } } }
    ir.sort_by(|a, b| b.0.cmp(&a.0));
    out += &format!("call_intrinsic: from bytecode {} from native {}\n",
        ir.iter().filter(|r| r.1 == 0).map(|r| r.0).sum::<u64>(), ir.iter().filter(|r| r.1 == 1).map(|r| r.0).sum::<u64>());
    for (n, k, i) in ir.iter().take(25) {
        let name = crate::bytecode::intrinsics::INTRINSICS.get(*i).map(|x| x.name).unwrap_or("?");
        out += &format!("  {:>11} {} {}\n", n, if *k == 1 { "native  " } else { "bytecode" }, name);
    }
    let mut nr = Vec::new();
    for i in 0..256 { let a = NARM[i].load(Relaxed); let s = NSTEP[i].load(Relaxed); if a + s > 0 { nr.push((a, s, i)); } }
    nr.sort_by(|a, b| b.0.cmp(&a.0));
    out += "natives (state arms executed, native_step entries):\n";
    for (a, s, i) in nr.iter().take(15) {
        let name = crate::builtins::r#gen::natives::NATIVES.get(*i).map(|x| x.name).unwrap_or("?");
        out += &format!("  {:>11} {:>11} {}\n", a, s, name);
    }
    // A builtin's name: the registry path (D2371) of a function object with its template.
    let mut builtin_names = alloc::collections::BTreeMap::new();
    for (cell, path) in crate::vm::registry::canonical_names(heap) {
        let o = crate::value::Obj(cell);
        if heap.cell_kind(cell) != crate::heap::CellKind::Object
            || heap.object_class(o) != crate::heap::ObjectClass::Function { continue; }
        if let Some(t) = heap.object_slot(o, 0).as_internal() {
            builtin_names.entry(t).or_insert(path);
        }
    }
    let mut fr = Vec::new();
    for i in 0..65536 {
        let o = FN_OPS[i].load(Relaxed);
        if o > 0 {
            let (key, kind) = (FN_KEY[i].load(Relaxed), FN_KIND[i].load(Relaxed));
            let name = if kind == 1 {
                builtin_names.get(&key).cloned().unwrap_or_else(|| format!("template@{key}"))
            } else {
                heap.atom_name(crate::heap::Atom::from_id(key))
            };
            fr.push((o, FN_ENTRIES[i].load(Relaxed), kind, name));
        }
    }
    fr.sort_by(|a, b| b.0.cmp(&a.0));
    out += "functions (bytecodes, entries at pc 0, bytecodes per entry, kind):\n";
    for (o, c, k, n) in fr.iter().take(25) {
        out += &format!("  {:>11} {:>10} {:>8.1} {} {}\n", o, c, *o as f64 / (*c).max(1) as f64, if *k == 1 { "builtin" } else { "user   " }, n);
    }
    let mut al = Vec::new();
    let (mut an, mut ab) = (0, 0);
    for i in 0..64 { let n = ALLOC_N[i].load(Relaxed); let b = ALLOC_B[i].load(Relaxed); an += n; ab += b; if n > 0 { al.push((b, n, i)); } }
    al.sort_by(|a, b| b.0.cmp(&a.0));
    out += &format!("allocations: {} cells, {} bytes; collections {} ({} bytes live after, summed)\n", an, ab, GCS.load(Relaxed), GC_BYTES.load(Relaxed));
    for (b, n, i) in al.iter().take(12) {
        let name = crate::heap::CellKind::from_u8(*i as u8).map(|k| format!("{k:?}")).unwrap_or_default();
        out += &format!("  {:>11} cells {:>12} bytes {}\n", n, b, name);
    }
    out
}
'''
open(os.path.join(dst, "crates/djs-core/src/prof.rs"), "w").write(PROF)
P = "crate::prof"
patch("crates/djs-core/src/lib.rs", "pub mod heap;\n", "pub mod prof;\n")

R = "crates/djs-core/src/interp/run.rs"
# Instructions through `step` (everything run_hot declines), and native steps.
patch(R, "    pub(super) fn step_once(&mut self) -> Option<Status> {\n",
      f"        {P}::step(self.heap, self.template, (self.pc < self.code_len).then(|| self.byte(self.pc)), self.pc);\n")
patch(R, "        let (params, locals) = (th.params, th.locals);\n", f"        {P}::frame(self.heap, t);\n")
# Every op of the tier-0 loop, with its frame's template.
patch("crates/djs-core/src/interp/run/hot.rs", "                    let op = w as u8;\n",
      f"                    {P}::hot(b, self.template, op, ip - code);\n")
N = "crates/djs-core/src/interp/run/native.rs"
patch(N, "        let idx = self.heap.arena().get(t, FuncTemplate::NATIVE);\n",
      f"        {P}::inc(&{P}::NFRAMES, 1);\n")
patch(N, "        let run = self.native_fn().run;\n",
      f"        {P}::inc(&{P}::NSTEP[((self.native - 1) & 255) as usize], 1);\n")
patch(N, "    pub(crate) fn n_state(&self) -> u32 {\n",
      f"        {P}::inc(&{P}::NARM[(self.native.wrapping_sub(1) & 255) as usize], 1);\n"
      f"        if {P}::on() {{ {P}::CUR.store(2, core::sync::atomic::Ordering::Relaxed); }}\n")
patch("crates/djs-core/src/interp/run/intrinsics.rs",
      "    pub(super) fn call_intrinsic(&mut self, id: u16, argc: u32, next: u32) -> Step {\n",
      f"        {P}::inc(&{P}::INTR[(self.native != 0) as usize][(id as usize).min(2047)], 1);\n")
H = "crates/djs-core/src/heap/mod.rs"
patch(H, "        let size = align8(size.max(kind.layout().min_size));\n",
      f"        {P}::inc(&{P}::ALLOC_N[kind as usize & 63], 1);\n"
      f"        {P}::inc(&{P}::ALLOC_B[kind as usize & 63], size as u64);\n", count=None)
# Every collection ends in after_collect; the collectors themselves are marked for the
# sampler (mark-compact: full, minor, seal; Cheney: canonical).
G = "crates/djs-core/src/heap/gc.rs"
patch(G, "    fn after_collect(&mut self, after: u32) {\n",
      f"        {P}::inc(&{P}::GCS, 1);\n        {P}::inc(&{P}::GC_BYTES, after as u64);\n")
patch("crates/djs-core/src/heap/compactor.rs",
      "    pub(super) fn mark_compact(&mut self, old: u32) -> Result<(u32, u32, u32), HeapError> {\n",
      f"        let _gc = {P}::Gc::start();\n")
patch(G, "    pub fn collect_canonical(&mut self) -> Result<GcStats, HeapError> {\n",
      f"        let _gc = {P}::Gc::start();\n")
C = "crates/djs-cli/src/run.rs"
patch(C, "    vm.call(main, this, args).expect(\"the VM is idle\");\n",
      "    let prof = std::env::var_os(\"DJS_PROF\").is_some();\n"
      "    if prof {\n"
      "        djs_core::prof::reset();\n"
      "        // The sampler: what runs every 50 µs.\n"
      "        std::thread::spawn(|| loop {\n"
      "            std::thread::sleep(std::time::Duration::from_micros(50));\n"
      "            let c = djs_core::prof::CUR.load(std::sync::atomic::Ordering::Relaxed) as usize;\n"
      "            djs_core::prof::SAMPLES[c.min(4)].fetch_add(1, std::sync::atomic::Ordering::Relaxed);\n"
      "        });\n"
      "    }\n")
patch(C, "        match vm.run(fuel) {\n",
      "        let st = vm.run(fuel);\n"
      "        djs_core::prof::CUR.store(4, std::sync::atomic::Ordering::Relaxed);\n"
      "        if prof && matches!(st, Status::Done(_) | Status::Idle) { eprint!(\"{}\", djs_core::prof::dump(vm.heap())); }\n"
      "        match st {\n", where="replace")
if failures:
    sys.exit("instr.py: patches do not apply to this checkout (fix the anchors in scripts/perf/instr.py):\n"
             + "\n".join(failures))

env = dict(os.environ, CARGO_TARGET_DIR=os.path.join(here, "instr/target"), CARGO_PROFILE_RELEASE_CODEGEN_UNITS="4")
subprocess.run(["cargo", "build", "--release", "-q", "-p", "djs-cli"], cwd=dst, env=env, check=True)
djs = os.path.join(here, "instr/target/release/djs")
print("built", djs, "(DJS_PROF=1 djs run --body x.js prints the counts to stderr)")

if "--self-test" in sys.argv:
    # Every counter must move: user and builtin bytecodes on both paths, a native builtin,
    # intrinsic calls, frames, allocations, collections and the sampler.
    test = os.path.join(here, "instr/self-test.js")
    open(test, "w").write("""
function work(n) { const a = []; for (let i = 0; i < n; i++) a.push({ i, s: "x" + i }); return a; }
let t = 0;
for (let r = 0; r < 60; r++) {
  const a = work(5000);
  a.sort((x, y) => y.i - x.i);
  t += a.map((o) => o.i).filter((i) => i % 3 === 0).reduce((s, i) => s + i, 0);
  t += JSON.stringify(a.slice(0, 50)).length + [...a.keys()].length;
  t += Object.entries(a[0]).length + a.slice(0, 200).flat().length + "ab".padStart(40, "xy").length;
}
return t;
""")
    r = subprocess.run([djs, "run", "--body", test], env=dict(os.environ, DJS_PROF="1"), capture_output=True, text=True)
    rep = r.stderr
    print(rep)
    m = re.search(r"bytecodes (\d+): user (\d+) \([\d.]+%, step ([\d.]+)%\), builtins (\d+) \([\d.]+%, step ([\d.]+)%\)", rep)
    tm = re.search(r"time \((\d+) samples\): user bytecode ([\d.]+)%, bytecode builtins ([\d.]+)%, native builtins ([\d.]+)%, collections ([\d.]+)%", rep)
    fr = re.search(r"frames: user (\d+) builtin (\d+) native (\d+); native_step entries (\d+)", rep)
    ci = re.search(r"call_intrinsic: from bytecode (\d+) from native (\d+)", rep)
    al = re.search(r"allocations: (\d+) cells, (\d+) bytes; collections (\d+)", rep)
    bad = []
    if r.returncode != 0: bad.append(f"djs exited {r.returncode}")
    if not m or int(m[2]) == 0 or int(m[4]) == 0: bad.append("bytecodes: user or builtin count is 0")
    elif float(m[3]) >= 100 or float(m[5]) >= 100: bad.append("run_hot counted nothing (100% through step)")
    if not tm or int(tm[1]) == 0 or float(tm[2]) == 0 or float(tm[3]) == 0 or float(tm[4]) == 0 or float(tm[5]) == 0:
        bad.append("time split: a bucket is empty")
    if not fr or int(fr[3]) == 0 or int(fr[4]) == 0: bad.append("native frames or steps are 0")
    if not ci or int(ci[1]) == 0 or int(ci[2]) == 0: bad.append("intrinsic calls are 0")
    if not al or int(al[1]) == 0 or int(al[3]) == 0: bad.append("allocations or collections are 0")
    if "natives (state arms" not in rep or "functions (bytecodes" not in rep: bad.append("report sections missing")
    if bad:
        sys.exit("instr.py self-test FAILED: " + "; ".join(bad))
    print("instr.py self-test: ok")
