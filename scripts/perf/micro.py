#!/usr/bin/env python3
"""micro.py [names...]: machine instructions (and cycles) per JS-level operation, djs vs
qjs. Each case is `setup; for (i < N) { body x K }`; the per-op cost is
(instr(case) - instr(empty loop)) / (N * K), from /usr/bin/time -l "instructions retired"
(exact to ~0.1%; cycles are noisy on a loaded machine). Min of R runs."""
import subprocess, re, os, sys, argparse
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(here, exist_ok=True)
ap = argparse.ArgumentParser()
ap.add_argument("-r", type=int, default=3)
ap.add_argument("-n", type=int, default=1_000_000)
ap.add_argument("--djs", default=os.path.join(here, "target/release/djs"))
ap.add_argument("--noqjs", action="store_true")
ap.add_argument("names", nargs="*")
a = ap.parse_args()
QJS = os.path.expanduser("~/Development/dquickts/qjs")
K = 10
from cases import CASES  # noqa: E402
def src(setup, body, k):
    return setup + f"\nfor (let i = 0; i < {a.n}; i++) {{ " + " ".join([body] * k) + " }\nreturn 0;"
def run(cmd):
    r = subprocess.run(["/usr/bin/time", "-l"] + cmd, capture_output=True, text=True, timeout=600)
    if r.returncode != 0 or "ERR" in r.stdout:
        print("FAILED", cmd, r.stdout[-300:], r.stderr[-500:]); return (float("nan"),) * 2
    e = r.stderr
    return (int(re.search(r"(\d+)\s+instructions retired", e).group(1)),
            int(re.search(r"(\d+)\s+cycles elapsed", e).group(1)))
os.makedirs(os.path.join(here, "m"), exist_ok=True)
def measure(name, setup, body, k):
    tag = re.sub(r"\W+", "_", name) + f"_{k}"
    dj = os.path.join(here, "m", tag + ".js")
    open(dj, "w").write(src(setup, body, k))
    qj = os.path.join(here, "m", tag + ".q.js")
    open(qj, "w").write('"use strict";(async () => {' + src(setup, body, k) + '\n})().then(() => {}, (e) => print("ERR", e));\n')
    res = {}
    for _ in range(a.r):
        for eng, cmd in [("djs", [a.djs, "run", "--body", dj])] + ([] if a.noqjs else [("qjs", [QJS, "--std", qj])]):
            ins, cyc = run(cmd)
            o = res.setdefault(eng, [ins, cyc])
            o[0] = min(o[0], ins); o[1] = min(o[1], cyc)
    return res
cases = [c for c in CASES if not a.names or any(n in c[0] for n in a.names)]
base = measure("empty", "", "", 0)
print(f"empty loop per iteration: djs {base['djs'][0] / a.n:.0f} instr" + ("" if a.noqjs else f", qjs {base['qjs'][0] / a.n:.0f}"))
print(f"{'operation':34} {'djs ins':>8} {'qjs ins':>8} {'ratio':>6} {'djs cyc':>8} {'qjs cyc':>8}")
for name, setup, body in cases:
    m = measure(name, setup, body, K)
    m0 = measure(name, setup, body, 0)  # the setup alone
    per = {e: ((m[e][0] - m0[e][0]) / (a.n * K), (m[e][1] - m0[e][1]) / (a.n * K)) for e in m}
    q = per.get("qjs", (float("nan"), float("nan")))
    print(f"{name:34} {per['djs'][0]:8.1f} {q[0]:8.1f} {per['djs'][0] / q[0] if q[0] > 0 else float('nan'):6.1f} {per['djs'][1]:8.1f} {q[1]:8.1f}", flush=True)
