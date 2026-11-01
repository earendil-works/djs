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
CASES = [
    # name, setup, body (repeated K times), bytecodes it should add (comment)
    ("local read (x;)", "let x = 1;", "x;"),
    ("local write (x = i)", "let x = 1;", "x = i;"),
    ("int add (x = x + 1 | 0)", "let x = 1;", "x = (x + 1) | 0;"),
    ("float add (y = y + 0.5)", "let y = 0.5;", "y = y + 0.5;"),
    ("compare+branch (if (i < 0) x++)", "let x = 0;", "if (i < 0) x++;"),
    ("call+return f(i)", "function f(a) { return a; }", "f(i);"),
    ("call 0-arg closure g()", "let c = 0; const g = () => c;", "g();"),
    ("method call o.m(i)", "const o = { m(a) { return a; } };", "o.m(i);"),
    ("own field read o.c", "const o = { a: 1, b: 2, c: 3, d: 4 };", "o.c;"),
    ("own field write o.c = i", "const o = { a: 1, b: 2, c: 3, d: 4 };", "o.c = i;"),
    ("proto field read p.m", "class P { m() {} }; const p = new P();", "p.m;"),
    ("global read Math", "", "Math;"),
    ("Math.PI", "", "Math.PI;"),
    ("array read a[i & 7]", "const a = [1,2,3,4,5,6,7,8];", "a[i & 7];"),
    ("array write a[i & 7] = i", "const a = [1,2,3,4,5,6,7,8];", "a[i & 7] = i;"),
    ("arr.length", "const a = [1,2,3,4,5,6,7,8];", "a.length;"),
    ("push+pop", "const a = [];", "a.push(i); a.pop();"),
    ("str.charCodeAt(k)", "const s = 'hello world';", "s.charCodeAt(3);"),
    ("str.length", "const s = 'hello world';", "s.length;"),
    ("empty object {}", "", "({});"),
    ("object {a, b}", "", "({ a: i, b: i });"),
    ("array [i, i]", "", "[i, i];"),
    ("closure () => i", "", "(() => i);"),
    ("string concat 'a' + i", "", "'a' + i;"),
    ("template `a${i}`", "", "`a${i}`;"),
    ("for-of over [1,2,3]", "const a = [1,2,3]; let t = 0;", "for (const x of a) t = x;"),
    ("a.indexOf(3) (len 8)", "const a = [1,2,3,4,5,6,7,8];", "a.indexOf(3);"),
    ("a.includes(3) (len 8)", "const a = [1,2,3,4,5,6,7,8];", "a.includes(3);"),
    ("arr.map(x=>x) len 4", "const a = [1,2,3,4];", "a.map((x) => x);"),
    ("s.slice(1, 4)", "const s = 'hello world';", "s.slice(1, 4);"),
    ("s.indexOf('o')", "const s = 'hello world';", "s.indexOf('o');"),
    ("JSON.stringify small", "const o = { a: 1, b: 'x' };", "JSON.stringify(o);"),
    ("Object.keys small", "const o = { a: 1, b: 'x' };", "Object.keys(o);"),
    ("destructure const {a,b} = o", "const o = { a: 1, b: 2 };", "{ const { a, b } = o; }"),
    ("spread [...a] len 4", "const a = [1,2,3,4];", "[...a];"),
    ("try/catch no throw", "let x = 0;", "try { x = i; } catch (e) {}"),
    ("line.includes('throw')", "const l = '  const value12 = compute(12, 7);';", "l.includes('throw');"),
    ("'a,b,c'.split(',')", "const t = 'a,b,c';", "t.split(',');"),
    ("s.padEnd(20)", "const s = 'hello';", "s.padEnd(20);"),
    ("String(i)", "", "String(i);"),
    ("s.startsWith('he')", "const s = 'hello';", "s.startsWith('he');"),
    ("s.trim()", "const s = ' hello ';", "s.trim();"),
    ("o.hasOwnProperty('a')", "const o = { a: 1 };", "o.hasOwnProperty('a');"),
    ("Math.max(i, 3)", "", "Math.max(i, 3);"),
    ("Math.floor(i / 3)", "", "Math.floor(i / 3);"),
]
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
