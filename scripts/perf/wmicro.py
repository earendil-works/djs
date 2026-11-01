#!/usr/bin/env python3
"""wmicro.py [filters]: machine instructions per JS operation in the wasm builds (djs-wasm
with pi's 10k fuel slices, quickjs-wasi), from micro.py's m/<case>_{0,10}.js files:
(instr(K=10) - instr(K=0)) / (N * 10). Run micro.py first (it writes the files, N=200000)."""
import subprocess, re, os, sys, glob
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(here, exist_ok=True)
N = int(os.environ.get("N", "200000")); R = int(os.environ.get("R", "2"))
flags = os.environ.get("NODE_FLAGS", "").split()
def ins(cmd, f):
    best = None
    for _ in range(R):
        r = subprocess.run(["/usr/bin/time", "-l", "node"] + flags + [cmd, f], capture_output=True, text=True, timeout=900, env=dict(os.environ, ONCE="1"), cwd=here)
        v = int(re.search(r"(\d+)\s+instructions retired", r.stderr).group(1))
        best = v if best is None else min(best, v)
    return best
cases = sorted({os.path.basename(p)[:-len("_10.js")] for p in glob.glob(os.path.join(here, "m", "*_10.js"))})
filt = sys.argv[1:]
print(f"{'case':36} {'djs-wasm':>9} {'qjs-wasi':>9} {'ratio':>6}")
for c in cases:
    if filt and not any(x in c for x in filt): continue
    N = int(re.search(r"i < (\d+)", open(os.path.join(here, "m", f"{c}_10.js")).read()).group(1))
    d = (ins(os.path.join(tools, "wasm-run.ts"), f"m/{c}_10.js") - ins(os.path.join(tools, "wasm-run.ts"), f"m/{c}_0.js")) / (N * 10)
    qsrc = open(os.path.join(here, "m", f"{c}_10.js")).read()  # the same body, run by qjs-wasm-run (it wraps it)
    q = (ins(os.path.join(tools, "qjs-wasm-run.mjs"), f"m/{c}_10.js") - ins(os.path.join(tools, "qjs-wasm-run.mjs"), f"m/{c}_0.js")) / (N * 10)
    print(f"{c:36} {d:9.1f} {q:9.1f} {d / q if q > 0 else float('nan'):6.1f}", flush=True)
