#!/usr/bin/env python3
"""wmeasure.py [-n N] [--wasm X.wasm[@SRCROOT]]... workloads...: the wasm build in node (D016's
primary target). Per workload: machine instructions retired and cycles of the whole node
process (/usr/bin/time -l, all threads, so V8's compilers are included) for ONE run of
w/<name>.js, minus the same for w/empty.js, min of N interleaved runs; and the same for
quickjs-wasi (pi's engine). Instructions are load independent (spread printed); cycles
and ms are not. `X.wasm@SRCROOT` runs that build with SRCROOT's scripts/perf/wasm-run.ts
and JS host (its heap reader), for builds whose heap layout differs from this checkout's.
A run that fails (non-zero exit, no timing line) stops the measurement with its error:
a workload never goes missing silently."""
import subprocess, re, os, argparse
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(here, exist_ok=True)
ap = argparse.ArgumentParser()
ap.add_argument("-n", type=int, default=3)
ap.add_argument("--wasm", action="append")
ap.add_argument("--noqjs", action="store_true")
ap.add_argument("--node-flag", action="append", default=[])
ap.add_argument("names", nargs="*")
a = ap.parse_args()
wasms = a.wasm or [os.path.join(here, "target/wasm32-unknown-unknown/wasm/djs_wasm.wasm")]
def run(cmd, env):
    r = subprocess.run(["/usr/bin/time", "-l"] + cmd, capture_output=True, text=True, timeout=900,
                       env=dict(os.environ, ONCE="1", **env), cwd=here)
    e = r.stderr
    m = re.search(r"\t([\d.]+) ms", r.stdout)
    if r.returncode != 0 or not m:
        # The error, without /usr/bin/time's statistics (the indented lines after it).
        err = [l for l in e.splitlines() if l.strip() and not re.match(r"\s+[\d.]+\s+(real|\w)", l)]
        tail = "\n".join((r.stdout.strip().splitlines() + err)[-20:])
        raise SystemExit(f"wmeasure.py: run failed (exit {r.returncode}): {' '.join(cmd)} with {env}\n{tail}\n"
                         "(a build with another heap layout needs its own reader: --wasm X.wasm@SRCROOT)")
    ins = int(re.search(r"(\d+)\s+instructions retired", e).group(1))
    cyc = int(re.search(r"(\d+)\s+cycles elapsed", e).group(1))
    mem = re.search(r"wasm memory ([\d.]+) MiB", r.stdout)
    return ins, cyc, float(m.group(1)), float(mem.group(1)) if mem else float("nan")
def djs(w):
    """A build, run with its own checkout's wasm-run.ts if given as X.wasm@SRCROOT."""
    wasm, _, src = w.partition("@")
    run = os.path.join(os.path.abspath(src), "scripts/perf/wasm-run.ts") if src else os.path.join(tools, "wasm-run.ts")
    if not os.path.isfile(run) or not os.path.isfile(wasm):
        raise SystemExit(f"wmeasure.py: no such build or reader: {wasm} {run}")
    return run, os.path.abspath(wasm)
engines = [(f"djs{i}" if len(wasms) > 1 else "djs-wasm", ["node"] + a.node_flag + [djs(w)[0]], {"DJS_WASM": djs(w)[1]}) for i, w in enumerate(wasms)]
if not a.noqjs:
    engines.append(("qjs-wasi", ["node"] + a.node_flag + [os.path.join(tools, "qjs-wasm-run.mjs")], {}))
names = ["empty"] + [n for n in a.names if n != "empty"]
res = {}
for name in names:
    for _ in range(a.n):
        for label, cmd, env in engines:
            res.setdefault((name, label), []).append(run(cmd + [name], env))
base = {l: (min(r[0] for r in res[("empty", l)]), min(r[1] for r in res[("empty", l)])) for l, _, _ in engines}
print(f"{'workload':12} {'engine':9} {'Minstr':>9} {'spread%':>7} {'Mcycles':>8} {'spread%':>7} {'ms(min)':>8} {'MiB':>6}")
for name in names[1:]:
    for l, _, _ in engines:
        rs = res[(name, l)]
        ins = [r[0] for r in rs]; cyc = [r[1] for r in rs]
        print(f"{name:12} {l:9} {(min(ins) - base[l][0]) / 1e6:9.1f} {100 * (max(ins) - min(ins)) / min(ins):7.1f} "
              f"{(min(cyc) - base[l][1]) / 1e6:8.1f} {100 * (max(cyc) - min(cyc)) / min(cyc):7.1f} {min(r[2] for r in rs):8.1f} {rs[0][3]:6.1f}", flush=True)
