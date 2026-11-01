#!/usr/bin/env python3
"""wcompile.py [-n N] [--reps R] [--qjs] --wasm A [--wasm B ...] files: compile-only cost in
the wasm build (0092). Instructions retired by the node process (/usr/bin/time -l) for R+1
compiles of each file (wasm-compile.ts, a fresh Sandbox per compile), minus the same for
an empty file, min of N interleaved runs: per compile and per source byte. --qjs adds
quickjs-wasi's compile (evalCode with COMPILE_ONLY). A file ending in .body.js compiles
as a codemode body. Small files need a large R: the first compiles run in Liftoff."""
import subprocess, re, os, argparse, tempfile
here = os.path.dirname(os.path.abspath(__file__))
ap = argparse.ArgumentParser()
ap.add_argument("-n", type=int, default=3)
ap.add_argument("--reps", type=int, default=3)
ap.add_argument("--wasm", action="append", default=[])
ap.add_argument("--qjs", action="store_true")
ap.add_argument("files", nargs="+")
a = ap.parse_args()
def run(args):
    r = subprocess.run(["/usr/bin/time", "-l", "node", os.path.join(here, "wasm-compile.ts"), "--reps", str(a.reps)] + args,
                       capture_output=True, text=True, cwd=here, timeout=900)
    m = re.search(r"(\d+)\s+instructions retired", r.stderr)
    if r.returncode != 0 or not m:
        raise SystemExit(r.stdout + r.stderr)
    return int(m.group(1))
engines = [(os.path.basename(w), ["--wasm", os.path.abspath(w)]) for w in a.wasm]
if a.qjs:
    engines.append(("qjs-wasi", ["--qjs"]))
res = {}
empty = os.path.join(tempfile.mkdtemp(), "empty.js")
open(empty, "w").close()
a.files = [os.path.abspath(f) for f in a.files]
for f in [empty] + a.files:
    for _ in range(a.n):
        for label, args in engines:
            res.setdefault((f, label), []).append(run(args + [f]))
print(f"{'file':14} {'engine':10} {'Minstr/compile':>14} {'instr/byte':>10}")
for f in a.files:
    size = os.path.getsize(f)
    for label, _ in engines:
        d = (min(res[(f, label)]) - min(res[(empty, label)])) / (a.reps + 1)
        print(f"{os.path.basename(f):14} {label:10} {d / 1e6:14.2f} {d / size:10.1f}", flush=True)
