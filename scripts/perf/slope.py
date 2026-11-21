#!/usr/bin/env python3
"""slope.py [-n N] [-r R] [--no-sync] [--flag F]... --wasm A.wasm [--wasm B.wasm]... [names|files]:
steady-state machine instructions per loop iteration of the wasm build in node. Each case
(scripts/perf/s/NAME.js, or a file) is a codemode body whose loop bound is written
N_ITER; it runs with N and with 2N iterations, and the difference divided by N is the
cost of one iteration. Whatever happens once cancels: node's start, compiling the module,
Liftoff code until run_hot tiers up. Runs with `--wasm-sync-tier-up` (the tier-up is then
at the same point in every run; the code is the one default tiering makes, unlike
`--no-liftoff`'s). Min of R runs each. With N = 10M two identical builds agree to about
0.05%; the last column is the last build against the first."""
import subprocess, re, os, argparse, tempfile
ap = argparse.ArgumentParser()
ap.add_argument("-n", type=int, default=10_000_000)
ap.add_argument("-r", type=int, default=2)
ap.add_argument("--wasm", action="append")
ap.add_argument("--flag", action="append", default=[], help="another node flag")
ap.add_argument("--no-sync", action="store_true", help="without --wasm-sync-tier-up")
ap.add_argument("files", nargs="*")
a = ap.parse_args()
tools = os.path.dirname(os.path.abspath(__file__))
run = os.path.join(tools, "wasm-run.ts")
cases = os.path.join(tools, "s")
if not a.wasm:
    raise SystemExit("slope.py: give at least one --wasm build")
files = [f if f.endswith(".js") else os.path.join(cases, f + ".js") for f in a.files] or \
    sorted(os.path.join(cases, f) for f in os.listdir(cases))
flags = ([] if a.no_sync else ["--wasm-sync-tier-up"]) + a.flag
tmp = tempfile.mkdtemp()
def ins(wasm, f):
    best = None
    for _ in range(a.r):
        r = subprocess.run(["/usr/bin/time", "-l", "node"] + flags + [run, f], capture_output=True, text=True, timeout=900,
                           env=dict(os.environ, ONCE="1", DJS_WASM=os.path.abspath(wasm)))
        if r.returncode != 0 or "Error" in r.stdout: print("FAILED", f, r.stdout[-300:], r.stderr[-300:])
        v = int(re.search(r"(\d+)\s+instructions retired", r.stderr).group(1))
        best = v if best is None else min(best, v)
    return best
print(f"{'case':28} " + " ".join(f"{os.path.basename(w)[:14]:>14}" for w in a.wasm) + "   delta")
for f in files:
    src = open(f).read()
    per = []
    for w in a.wasm:
        v = []
        for k in (1, 2):
            p = os.path.join(tmp, f"{os.path.basename(f)}.{k}.js")
            open(p, "w").write(src.replace("N_ITER", str(a.n * k)))
            v.append(ins(w, p))
        per.append((v[1] - v[0]) / a.n)
    d = "" if len(per) < 2 else f"  {100 * (per[-1] / per[0] - 1):+6.2f}%"
    print(f"{os.path.basename(f):28} " + " ".join(f"{p:14.1f}" for p in per) + d, flush=True)
