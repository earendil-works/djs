#!/usr/bin/env python3
"""perelem.py: machine instructions per array element for iteration builtins, djs vs qjs:
(instr(len L) - instr(len 0)) / (R * L), R repetitions of the operation."""
import subprocess, re, os
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(here, exist_ok=True)
QJS = os.path.expanduser("~/Development/dquickts/qjs"); DJS = os.path.join(here, "target/release/djs")
R, L = 2000, 500
CASES = [
    ("for (const x of a) t += x", "for (const x of a) t += x;"),
    ("for (let i...) t += a[i]", "for (let i = 0; i < a.length; i++) t += a[i];"),
    ("a.forEach(x => { t += x })", "a.forEach((x) => { t += x; });"),
    ("a.map(x => x)", "a.map((x) => x);"),
    ("a.filter(x => true)", "a.filter((x) => true);"),
    ("a.reduce((s, x) => s + x, 0)", "t += a.reduce((s, x) => s + x, 0);"),
    ("a.indexOf(-1)", "t += a.indexOf(-1);"),
    ("a.includes(-1)", "t += a.includes(-1) ? 1 : 0;"),
    ("a.join(',')", "t += a.join(',').length;"),
    ("a.slice()", "t += a.slice().length;"),
    ("[...a]", "t += [...a].length;"),
    ("a.sort((x, y) => x - y) (sorted)", "a.sort((x, y) => x - y);"),
    ("s.split('\\n') per piece", None),
]
def run(cmd):
    r = subprocess.run(["/usr/bin/time", "-l"] + cmd, capture_output=True, text=True, timeout=600)
    return int(re.search(r"(\d+)\s+instructions retired", r.stderr).group(1))
def prog(body, n):
    return f"const a = []; for (let i = 0; i < {n}; i++) a.push(i); let t = 0;\nfor (let r = 0; r < {R}; r++) {{ {body} }}\nreturn t;"
def prog_split(n):
    return f"const s = 'ab\\n'.repeat({n}); let t = 0;\nfor (let r = 0; r < {R}; r++) {{ t += s.split('\\n').length; }}\nreturn t;"
os.makedirs(os.path.join(here, "pe"), exist_ok=True)
def both(src, tag):
    d = os.path.join(here, "pe", tag + ".js"); open(d, "w").write(src)
    q = os.path.join(here, "pe", tag + ".q.js"); open(q, "w").write('"use strict";(async () => {' + src + '\n})().then(() => {}, (e) => print("ERR", e));\n')
    return min(run([DJS, "run", "--body", d]) for _ in range(2)), min(run([QJS, "--std", q]) for _ in range(2))
print(f"{'per element':36} {'djs':>8} {'qjs':>8} {'ratio':>6}")
for i, (name, body) in enumerate(CASES):
    if body is None:
        d1, q1 = both(prog_split(L), f"c{i}_L"); d0, q0 = both(prog_split(0), f"c{i}_0")
    else:
        d1, q1 = both(prog(body, L), f"c{i}_L"); d0, q0 = both(prog(body, 0), f"c{i}_0")
    d = (d1 - d0) / (R * L); q = (q1 - q0) / (R * L)
    print(f"{name:36} {d:8.1f} {q:8.1f} {d / q:6.1f}", flush=True)
