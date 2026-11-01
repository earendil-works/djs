#!/usr/bin/env python3
"""prof.py <workload> [--bin djs] [--rate 4000] [--top 30]: samply profile of
`djs run --body w/<workload>.js`; self samples by innermost inlined function, by the
real (outlined) function, by source line, and the hottest addresses. Also buckets by
whole stack (gc / alloc / native code / intrinsic / call-return / dispatch)."""
import json, gzip, subprocess, sys, collections, os, argparse, re
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(here, exist_ok=True)
ap = argparse.ArgumentParser()
ap.add_argument("workload")
ap.add_argument("--bin", default=os.path.join(here, "target/release/djs"))
ap.add_argument("--dsym", default=os.path.join(here, "djs.dSYM/Contents/Resources/DWARF/djs"))
ap.add_argument("--rate", default="4000")
ap.add_argument("--top", type=int, default=30)
ap.add_argument("--reuse", action="store_true")
a = ap.parse_args()
out = os.path.join(here, "prof", a.workload + ".json.gz")
os.makedirs(os.path.dirname(out), exist_ok=True)
if not a.reuse:
    subprocess.run(["samply", "record", "-s", "-r", a.rate, "-o", out, "--", a.bin, "run", "--body",
                    os.path.join(tools, "w", a.workload + ".js")], check=True, capture_output=True)
p = json.load(gzip.open(out))
libs = p["libs"]
t = max(p["threads"], key=lambda t: t["samples"]["length"])
ft, st, fn, rt = t["frameTable"], t["stackTable"], t["funcTable"], t["resourceTable"]
def lib_of(f):
    r = fn["resource"][ft["func"][f]]
    return libs[rt["lib"][r]]["name"] if r is not None and r >= 0 else "?"
djs_frames = sorted({f for f in range(ft["length"]) if lib_of(f) == "djs"})
addrs = sorted({ft["address"][f] for f in djs_frames})
info = {}  # addr -> list of (func, file:line) innermost first
for i in range(0, len(addrs), 400):
    chunk = addrs[i:i + 400]
    o = subprocess.run(["atos", "-o", a.dsym, "-l", "0x100000000", "-i"] + [hex(0x100000000 + x) for x in chunk],
                       capture_output=True, text=True).stdout
    blocks = o.strip("\n").split("\n\n")
    for x, b in zip(chunk, blocks):
        fr = []
        for l in b.split("\n"):
            if not l.strip(): continue
            m = re.match(r"(.*?) \(in djs\) \((.*?)\)", l)
            if m: fr.append((m.group(1), m.group(2)))
            else: fr.append((l.split(" (in ")[0], "?"))
        info[x] = fr
def short(n):
    n = re.sub(r"_\$LT\$|\$GT\$|<.*?>::", "", n)
    n = re.sub(r"\(.*$", "", n)
    return n[:90]
self_inner = collections.Counter(); self_outer = collections.Counter(); self_line = collections.Counter()
self_addr = collections.Counter(); bucket = collections.Counter(); incl = collections.Counter()
lib_callers = collections.Counter()
total = 0
cache = {}
def chain(s):
    if s in cache: return cache[s]
    res = []; s0 = s
    while s is not None:
        f = st["frame"][s]
        if lib_of(f) == "djs":
            res.extend(x[0] for x in info.get(ft["address"][f], []))
        else:
            res.append(lib_of(f))
        s = st["prefix"][s]
    cache[s0] = res
    return res
BUCKETS = [
    ("gc", ["collect", "safepoint_collect", "forward"]),
    ("native builtin code", ["native_"]),
    ("call_intrinsic", ["call_intrinsic", "call_installed"]),
    ("alloc", ["alloc_cell", "alloc_object", "new_object", "new_array", "alloc_value_array"]),
    ("property lookup", ["lookup", "find_own", "table_find", "get_property", "set_property", "lookup_ic", "create_data_property"]),
]
for s in t["samples"]["stack"]:
    if s is None: continue
    total += 1
    f = st["frame"][s]
    addr = ft["address"][f] if lib_of(f) == "djs" else None
    if addr is None:
        self_inner[lib_of(f)] += 1; self_outer[lib_of(f)] += 1; self_line["(" + lib_of(f) + ")"] += 1
    else:
        fr = info.get(addr, [("?", "?")])
        self_inner[short(fr[0][0])] += 1
        self_outer[short(fr[-1][0])] += 1
        self_line[fr[0][1] + "  " + short(fr[0][0])] += 1
        self_addr[addr] += 1
    c = chain(s)
    if addr is None:
        callers = [x for x in c if not x.endswith(".dylib") and x != "?"]
        lib_callers[lib_of(f) + " <- " + " <- ".join(short(x) for x in callers[:8] if "core3ptr" not in x and "alloc3vec" not in x)] += 1
    joined = " ".join(c)
    for name, pats in BUCKETS:
        if any(pat in joined for pat in pats):
            bucket[name] += 1; break
    else:
        bucket["interpreter (run_loop and inlined)"] += 1
    for n in set(short(x) for x in c):
        incl[n] += 1
def show(title, c, n):
    print(f"--- {title}")
    for k, v in c.most_common(n):
        print(f"{100 * v / total:6.1f}%  {k}")
print(f"{a.workload}: {total} samples")
show("buckets (whole stack, first match)", bucket, 10)
show("self, innermost inlined function", self_inner, a.top)
show("self, real function", self_outer, 15)
show("inclusive", incl, 25)
show("self, source line", self_line, a.top)
show("system library self time by caller", lib_callers, 8)
print("--- hottest addresses")
for k, v in self_addr.most_common(15):
    fr = info.get(k, [("?", "?")])
    print(f"{100 * v / total:6.1f}%  0x{0x100000000 + k:x}  {fr[0][1]}  {short(fr[0][0])}")
