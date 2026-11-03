#!/usr/bin/env python3
"""measure.py [-n N] [--djs BIN]... workloads...: instructions retired, cycles and user time
per run (from /usr/bin/time -l), for djs (--body) and qjs (wrapped in a strict async
arrow), interleaved. Prints min/median per engine and the empty.js-subtracted numbers."""
import subprocess, sys, re, os, statistics, argparse
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(here, exist_ok=True)
ap = argparse.ArgumentParser()
ap.add_argument("-n", type=int, default=5)
ap.add_argument("--djs", action="append")
ap.add_argument("--noqjs", action="store_true")
ap.add_argument("--backend", default=None)
ap.add_argument("names", nargs="*")
a = ap.parse_args()
djs = a.djs or [os.path.join(here, "target/release/djs")]
QJS = os.path.expanduser("~/Development/dquickts/qjs")
os.makedirs(os.path.join(here, "q"), exist_ok=True)
def qfile(name):
    src = open(os.path.join(tools, "w", name + ".js")).read()
    p = os.path.join(here, "q", name + ".js")
    open(p, "w").write('"use strict";(async () => {' + src + '\n})().then((r) => print(r), (e) => print("ERR", e));\n')
    return p
def run(cmd):
    r = subprocess.run(["/usr/bin/time", "-l"] + cmd, capture_output=True, text=True, timeout=600)
    e = r.stderr
    if r.returncode != 0:
        raise SystemExit(f"measure.py: run failed (exit {r.returncode}): {' '.join(cmd)}\n{e[-1500:]}")
    ins = int(re.search(r"(\d+)\s+instructions retired", e).group(1))
    cyc = int(re.search(r"(\d+)\s+cycles elapsed", e).group(1))
    user = float(re.search(r"([\d.]+) user", e).group(1))
    rss = int(re.search(r"(\d+)\s+peak memory footprint", e).group(1))
    return ins, cyc, user, rss, r.stdout.strip()[-40:]
names = a.names or sorted(f[:-3] for f in os.listdir(os.path.join(tools, "w")))
if "empty" not in names: names = ["empty"] + names
engines = [(f"djs{i}" if len(djs) > 1 else "djs", d) for i, d in enumerate(djs)]
res = {}
for name in names:
    cmds = []
    for label, d in engines:
        c = [d, "run", "--body"] + (["--backend", a.backend] if a.backend else []) + [os.path.join(tools, "w", name + ".js")]
        cmds.append((label, c))
    if not a.noqjs:
        cmds.append(("qjs", [QJS, "--std", qfile(name)]))
    for i in range(a.n):
        for label, c in cmds:
            res.setdefault((name, label), []).append(run(c))
def summ(rs):
    ins = sorted(r[0] for r in rs); cyc = sorted(r[1] for r in rs); us = sorted(r[2] for r in rs)
    return ins[0], statistics.median(ins), cyc[0], statistics.median(cyc), us[0], max(r[3] for r in rs), rs[0][4]
labels = [l for l, _ in engines] + ([] if a.noqjs else ["qjs"])
print(f"{'workload':14} {'engine':8} {'Minstr':>9} {'spread%':>7} {'Mcyc':>9} {'spread%':>7} {'IPC':>5} {'user s':>7} {'MB':>6}  out")
base = {l: summ(res[("empty", l)]) for l in labels}
for name in names:
    for l in labels:
        rs = res[(name, l)]
        s = summ(rs)
        ins = [r[0] for r in rs]; cyc = [r[1] for r in rs]
        b = base[l] if name != "empty" else (0,0,0,0,0,0,0)
        di = s[0] - b[0]; dc = s[2] - b[2]
        print(f"{name:14} {l:8} {di/1e6:9.1f} {100*(max(ins)-min(ins))/min(ins):7.1f} {dc/1e6:9.1f} {100*(max(cyc)-min(cyc))/min(cyc):7.1f} {di/max(dc,1):5.2f} {s[4]:7.3f} {s[5]/1e6:6.1f}  {s[6]}")
