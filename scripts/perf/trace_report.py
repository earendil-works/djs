#!/usr/bin/env python3
"""trace_report.py <trace.txt> [--show OPNAME] [--show-n K]: per-bytecode machine
instruction counts from a trace.py trace. Segments the trace at the run loop's fuel check;
a segment is one fast_step/step dispatch (bytecode op name) or one native transition
(native). For each op: occurrences, instructions, loads, stores, branches per occurrence,
and where the instructions come from (categories from the inlined source location)."""
import sys, re, subprocess, collections, os, argparse
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(here, exist_ok=True)
ap = argparse.ArgumentParser()
ap.add_argument("trace")
ap.add_argument("--show", default=None)
ap.add_argument("--show-n", type=int, default=1)
ap.add_argument("--dsym", default=os.path.join(here, "djs.dSYM/Contents/Resources/DWARF/djs"))
a = ap.parse_args()

OPS = {}
src = open(os.path.join(root, "crates/djs-core/src/bytecode/ops.rs")).read()
for i, m in enumerate(re.finditer(r"^    [A-Z][A-Za-z0-9]* ([a-z_0-9]+)\(", src, re.M)):
    OPS[i] = m.group(1).rstrip("_")
asm = {}
for l in open(os.path.join(here, "all.s")):
    m = re.match(r"\s*([0-9a-f]+):\s+(\S+)\s*(.*)", l)
    if m: asm[int(m.group(1), 16)] = (m.group(2), m.group(3))
rows = []
for l in open(a.trace):
    p = l.split()
    pc = int(p[0], 16)
    if len(p) > 1 and p[1] == "B":
        rows.append((pc, (int(p[2]), int(p[3]))))
    else:
        rows.append((pc, None))
pcs = sorted({pc for pc, _ in rows if pc in asm})
info = {}
for i in range(0, len(pcs), 400):
    chunk = pcs[i:i + 400]
    o = subprocess.run(["atos", "-o", a.dsym, "-l", "0x100000000", "-i"] + [hex(x) for x in chunk], capture_output=True, text=True).stdout
    for x, b in zip(chunk, o.strip("\n").split("\n\n")):
        fr = []
        for ln in b.split("\n"):
            m = re.match(r"(.*?) \(in djs\) \((.*?)\)", ln)
            if m: fr.append((m.group(1), m.group(2)))
        info[x] = fr

def idents(n):
    out = []; i = 0
    while i < len(n):
        m = re.match(r"(\d+)", n[i:])
        if m:
            k = int(m.group(1)); j = i + len(m.group(1))
            if n[j:j+1] == "_": j += 1
            name = n[j:j + k]
            if re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", name): out.append(name); i = j + k; continue
            i = j
        else:
            i += 1
    return [x for x in out if not re.match(r"^Cs|^Nt|^[A-Z][a-z]?$", x)]
def short(n):
    ids = idents(n)
    return ids[-1] if ids else n[:40]

RULES = [
    # (category, test on (mnemonic, frames)) first match wins; frames innermost first
    ("system lib (memcpy/memset/malloc)", lambda m, f, pc: pc not in asm),
    ("bounds checks", lambda m, f, pc: f and (re.search(r"uint_macros|checked_sub|index\.rs|slice/mod|first_chunk|out_of_bounds", f[0][1] + f[0][0]) ) and not m.startswith(("ld", "st"))),
    ("arena/stack load", lambda m, f, pc: m.startswith("ld") and f and re.search(r"layout\.rs|read_array|from_le_bytes", f[0][1] + f[0][0])),
    ("arena/stack store", lambda m, f, pc: m.startswith("st") and f and re.search(r"layout\.rs|write_array", f[0][1] + f[0][0])),
    ("Vec ptr/len reload", lambda m, f, pc: f and re.search(r"as_slice|as_mut_slice|vec/mod\.rs|deref", f[0][0] + f[0][1])),
    ("address arithmetic (cell+field+index)", lambda m, f, pc: f and re.search(r"layout\.rs:3[56]\d|Tail.*at|arena\.rs:26[0-9]|Field", f[0][1] + f[0][0])),
    ("NaN-box tag tests/encode", lambda m, f, pc: f and re.search(r"value\.rs", f[0][1])),
    ("fuel", lambda m, f, pc: f and re.search(r"run\.rs:(1127|1137|947)\b|fuel", f[0][1] + f[0][0])),
    ("dispatch (pc/code/opcode/jump table)", lambda m, f, pc: f and re.search(r"run\.rs:(97[2-9]|98\d|99\d|100[0-9]|101[0-2]|1128)\b|fast_step$", f[0][1] + short(f[0][0]))),
    ("Exec state load/store (sp/pc/lp/stack...)", lambda m, f, pc: m.startswith(("ld", "st")) and f and re.search(r"run\.rs|native\.rs|ic\.rs", f[0][1])),
    ("spills/stack frame", lambda m, f, pc: m.startswith(("ld", "st")) and "sp" in asm.get(pc, ("", ""))[1].split(",")[1:2].__str__()),
]
def cat(pc):
    m = asm.get(pc, ("?", ""))[0]
    f = info.get(pc, [])
    for name, t in RULES:
        try:
            if t(m, f, pc): return name
        except Exception:
            pass
    return "other: " + (short(f[0][0]) if f else "?")

segs = []
cur = None
for pc, b in rows:
    if b is not None:
        if cur: segs.append(cur)
        bpc, op = b
        cur = [OPS.get(op, "native" if op < 0 else f"op{op}"), []]
    if cur: cur[1].append(pc)
if cur: segs.append(cur)
segs = segs[1:-1]  # partial ends
per = collections.defaultdict(lambda: [0, 0, 0, 0, 0, collections.Counter()])
tot_cat = collections.Counter()
total = 0
for name, ps in segs:
    s = per[name]
    s[0] += 1; s[1] += len(ps)
    for pc in ps:
        m = asm.get(pc, ("?", ""))[0]
        if m.startswith("ld"): s[2] += 1
        if m.startswith("st"): s[3] += 1
        if m.startswith(("b", "cb", "tb", "ret")): s[4] += 1
        c = cat(pc); s[5][c] += 1; tot_cat[c] += 1
        total += 1
print(f"{len(segs)} dispatches, {total} instructions ({total / max(len(segs), 1):.1f} per dispatch)")
print(f"{'op':22} {'n':>5} {'instr':>7} {'loads':>6} {'stores':>6} {'branch':>6}  top categories")
for name, s in sorted(per.items(), key=lambda kv: -kv[1][1]):
    n = s[0]
    cats = ", ".join(f"{k} {v / n:.0f}" for k, v in s[5].most_common(4))
    print(f"{name:22} {n:5} {s[1] / n:7.1f} {s[2] / n:6.1f} {s[3] / n:6.1f} {s[4] / n:6.1f}  {cats}")
print("--- all instructions by category")
for k, v in tot_cat.most_common(25):
    print(f"{100 * v / total:6.1f}%  {k}")
if a.show:
    k = 0
    for name, ps in segs:
        if name != a.show: continue
        print(f"--- one {name} ({len(ps)} instructions)")
        for pc in ps:
            m, ops = asm.get(pc, ("(lib)", ""))
            f = info.get(pc, [])
            loc = f"{f[0][1]} {short(f[0][0])}" if f else ""
            print(f"  {pc:x} {m:6} {ops[:44]:44} {cat(pc)[:28]:28} {loc[:60]}")
        k += 1
        if k >= a.show_n: break
if os.environ.get("BYFUNC"):
    want = os.environ["BYFUNC"]
    inner = collections.Counter(); outer = collections.Counter(); n = 0
    for name, ps in segs:
        if name != want: continue
        n += 1
        for pc in ps:
            f = info.get(pc, [])
            inner[short(f[0][0]) if f else "(lib)"] += 1
            # the outermost function below run_loop / native_step that is not a generic helper
            chain = [short(x[0]) for x in f]
            outer[" > ".join(chain[::-1][-3:]) if chain else "(lib)"] += 1
    print(f"--- {want}: by innermost function (per occurrence, {n} occurrences)")
    for k, v in inner.most_common(40): print(f"  {v / n:8.1f}  {k}")
