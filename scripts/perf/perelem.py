#!/usr/bin/env python3
"""perelem.py [--djs BIN]... [--wasm X.wasm]... [--noqjs] [names...]: machine instructions
per array element of the iteration builtins: (instr(len L) - instr(len 0)) / (R * L), R
repetitions of the operation. Engines: native djs builds (`djs run --body`), djs-wasm
builds in node (wasm-run.ts, pi's 10k fuel slices), and qjs (native) unless --noqjs."""
import subprocess, re, os, argparse
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(os.path.join(here, "pe"), exist_ok=True)
ap = argparse.ArgumentParser()
ap.add_argument("--djs", action="append", default=[])
ap.add_argument("--wasm", action="append", default=[])
ap.add_argument("--noqjs", action="store_true")
ap.add_argument("-r", type=int, default=2000)
ap.add_argument("-l", type=int, default=500)
ap.add_argument("names", nargs="*")
a = ap.parse_args()
QJS = os.path.expanduser("~/Development/dquickts/qjs")
R, L = a.r, a.l
CASES = [
    ("for (const x of a) t += x", "for (const x of a) t += x;"),
    ("for (let i...) t += a[i]", "for (let i = 0; i < a.length; i++) t += a[i];"),
    ("a.forEach(x => { t += x })", "a.forEach((x) => { t += x; });"),
    ("a.map(x => x)", "a.map((x) => x);"),
    ("a.filter(x => true)", "a.filter((x) => true);"),
    ("a.reduce((s, x) => s + x, 0)", "t += a.reduce((s, x) => s + x, 0);"),
    ("a.some(x => x < 0)", "t += a.some((x) => x < 0) ? 1 : 0;"),
    ("a.find(x => x < 0)", "t += a.find((x) => x < 0) === undefined ? 1 : 0;"),
    ("a.indexOf(-1)", "t += a.indexOf(-1);"),
    ("a.includes(-1)", "t += a.includes(-1) ? 1 : 0;"),
    ("a.join(',')", "t += a.join(',').length;"),
    ("a.slice()", "t += a.slice().length;"),
    ("[...a]", "t += [...a].length;"),
    ("a.sort((x, y) => x - y) (sorted)", "a.sort((x, y) => x - y);"),
    ("s.split('\\n') per piece", None),
]
def run(cmd, env=None):
    r = subprocess.run(["/usr/bin/time", "-l"] + cmd, capture_output=True, text=True, timeout=900,
                       env=dict(os.environ, **(env or {})), cwd=here)
    return int(re.search(r"(\d+)\s+instructions retired", r.stderr).group(1))
def prog(body, n):
    return f"const a = []; for (let i = 0; i < {n}; i++) a.push(i); let t = 0;\nfor (let r = 0; r < {R}; r++) {{ {body} }}\nreturn t;"
def prog_split(n):
    return f"const s = 'ab\\n'.repeat({n}); let t = 0;\nfor (let r = 0; r < {R}; r++) {{ t += s.split('\\n').length; }}\nreturn t;"
engines = [(f"djs{i}", lambda f, d=d: run([d, "run", "--body", f])) for i, d in enumerate(a.djs)]
engines += [(f"wasm{i}", lambda f, w=w: run(["node", os.path.join(tools, "wasm-run.ts"), f], {"ONCE": "1", "DJS_WASM": os.path.abspath(w)}))
            for i, w in enumerate(a.wasm)]
if not a.noqjs:
    def q(f):
        qf = f[:-3] + ".q.js"
        open(qf, "w").write('"use strict";(async () => {' + open(f).read() + '\n})().then(() => {}, (e) => print("ERR", e));\n')
        return run([QJS, "--std", qf])
    engines.append(("qjs", q))
print(f"{'per element':36} " + " ".join(f"{n:>8}" for n, _ in engines))
for i, (name, body) in enumerate(CASES):
    if a.names and not any(x in name for x in a.names):
        continue
    files = []
    for n in (L, 0):
        f = os.path.join(here, "pe", f"c{i}_{n}.js")
        open(f, "w").write(prog_split(n) if body is None else prog(body, n))
        files.append(f)
    per = []
    for _, e in engines:
        hi = min(e(files[0]) for _ in range(2))
        lo = min(e(files[1]) for _ in range(2))
        per.append((hi - lo) / (R * L))
    print(f"{name:36} " + " ".join(f"{p:8.1f}" for p in per), flush=True)
