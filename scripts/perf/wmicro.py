#!/usr/bin/env python3
"""wmicro.py [--wasm X.wasm]... [--noqjs] [-n N] [-r R] [filters]: machine instructions per
JS operation in the wasm builds (djs-wasm with pi's 10k fuel slices; quickjs-wasi unless
--noqjs), for the cases of cases.py: setup; for (i < N) { body x 10 }, minus the same
with the body x 0, divided by N * 10. Min of R runs per file. Several --wasm builds are
compared side by side (A/B); NODE_FLAGS passes node flags (e.g. --no-liftoff)."""
import subprocess, re, os, argparse
from cases import CASES
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(os.path.join(here, "wm"), exist_ok=True)
ap = argparse.ArgumentParser()
ap.add_argument("-n", type=int, default=100_000)
ap.add_argument("-r", type=int, default=2)
ap.add_argument("--wasm", action="append")
ap.add_argument("--noqjs", action="store_true")
ap.add_argument("names", nargs="*")
a = ap.parse_args()
wasms = a.wasm or [os.path.join(here, "target/wasm32-unknown-unknown/wasm/djs_wasm.wasm")]
flags = os.environ.get("NODE_FLAGS", "").split()
K = 10

def ins(runner, f, env):
    best = None
    for _ in range(a.r):
        r = subprocess.run(["/usr/bin/time", "-l", "node"] + flags + [runner, f], capture_output=True, text=True,
                           timeout=900, env=dict(os.environ, ONCE="1", **env), cwd=here)
        if r.returncode != 0 or "Error" in r.stdout:
            print("FAILED", f, r.stdout[-300:], r.stderr[-300:])
        v = int(re.search(r"(\d+)\s+instructions retired", r.stderr).group(1))
        best = v if best is None else min(best, v)
    return best

engines = [(f"djs{i}" if len(wasms) > 1 else "djs-wasm", os.path.join(tools, "wasm-run.ts"), {"DJS_WASM": os.path.abspath(w)})
           for i, w in enumerate(wasms)]
if not a.noqjs:
    engines.append(("qjs-wasi", os.path.join(tools, "qjs-wasm-run.mjs"), {}))
print(f"{'operation':34} " + " ".join(f"{e[0]:>9}" for e in engines) + ("" if len(engines) < 2 else f" {'ratio':>6}"))
for name, setup, body in CASES:
    if a.names and not any(x in name for x in a.names):
        continue
    tag = re.sub(r"\W+", "_", name)
    files = []
    for k in (K, 0):
        p = os.path.join(here, "wm", f"{tag}_{k}.js")
        open(p, "w").write(setup + f"\nfor (let i = 0; i < {a.n}; i++) {{ " + " ".join([body] * k) + " }\nreturn 0;")
        files.append(p)
    per = [(ins(r, files[0], env) - ins(r, files[1], env)) / (a.n * K) for _, r, env in engines]
    ratio = f" {per[0] / per[-1]:6.2f}" if len(per) > 1 and per[-1] > 0 else ""
    print(f"{name:34} " + " ".join(f"{p:9.1f}" for p in per) + ratio, flush=True)
