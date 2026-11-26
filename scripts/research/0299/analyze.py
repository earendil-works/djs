#!/usr/bin/env python3
"""Bytecode stability of the codemode corpus over merge points (issue 0299).

Reads work/scratch/0299/disasm/<sha>/<case>.txt (disasm_all.py) for the points in
work/scratch/0299/points.txt ("pos sha issue", oldest first) and hashes every function
at three levels:
  exact  the disassembly without source positions (`; line:col`): pcs, opcodes, operands
         by name (atoms, intrinsics, constants by value), IC slot numbers, header
         (params, locals, stack, captures), handler table;
  pc     what a saved frame relies on: the same minus IC slot numbers, constant pool
         indexes, the max stack size and the debug names of locals;
  iface  what a closure of a function that holds no frame relies on: params, captures
         count and the captured names it reads.
A script survives a span when every function's hash is equal at both ends; "frames"
survival requires `pc` equality only for functions that contain an await or yield (the
ones that can hold a frame at an idle point) and `iface` equality for the rest.
Writes a report to stdout. Usage: analyze.py [FIRST-ISSUE]
"""
import os, re, sys, hashlib, json
from collections import defaultdict

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
S = os.path.join(ROOT, "work/scratch/0299")
POS = re.compile(r"  ; \d+:\d+$")
ADDR = re.compile(r"internal@\d+")  # a heap offset (a regexp literal's cell), not meaning
IC = re.compile(r" ic\d+\b")
CONST = re.compile(r" #\d+ ")
DEBUGNAME = re.compile(r" \((?!\")[^()]*\)")
STACK = re.compile(r" stack=\d+")
SUSPEND = re.compile(r"^\s+\d+\s+(await|yield|async_yield|initial_yield|generator_yield|yield_star|async_generator_yield)\b")

def h(lines):
    return hashlib.sha1("\n".join(lines).encode()).hexdigest()[:16]

def parse(path):
    funcs = {}
    cur = None
    for line in open(path, encoding="utf-8", errors="replace"):
        line = ADDR.sub("internal@", POS.sub("", line.rstrip("\n")))
        if line.startswith("function "):
            key = line.split(":")[0]  # "function 0.1 name"
            cur = [line]
            funcs[key] = cur
        elif cur is not None:
            cur.append(line)
    out = {}
    for key, lines in funcs.items():
        pcl = [DEBUGNAME.sub("", CONST.sub(" ", IC.sub("", STACK.sub("", l)))) for l in lines]
        suspends = any(SUSPEND.match(l) for l in lines[1:])
        hdr = lines[0]
        m = re.search(r"params=(\d+).*captures=(\d+)", hdr)
        caps = sorted(set(re.findall(r"get_cap\w* cap(\d+) \(([^)]*)\)", "\n".join(lines))))
        iface = h([key, m.group(1) if m else "", m.group(2) if m else "", repr(caps)])
        out[key] = (h(lines), h(pcl), iface, suspends)
    return out

def load(points):
    data = {}
    cases = None
    for pos, sha, issue in points:
        d = os.path.join(S, "disasm", sha)
        if not os.path.isdir(d):
            continue
        names = sorted(n.rsplit(".", 1)[0] for n in os.listdir(d))
        cases = names if cases is None else cases
        per = {}
        for n in names:
            p = os.path.join(d, n + ".txt")
            per[n] = parse(p) if os.path.exists(p) else None  # None: does not compile
        data[sha] = per
    return data, cases

def same(a, b, level):
    """Whether script versions a and b (function dicts) match at a level."""
    if a is None or b is None or a.keys() != b.keys():
        return False
    for k in a:
        fa, fb = a[k], b[k]
        if level == "exact" and fa[0] != fb[0]:
            return False
        if level == "pc" and fa[1] != fb[1]:
            return False
        if level == "frames":
            if fa[3] or fb[3]:
                if fa[1] != fb[1]:
                    return False
            elif fa[2] != fb[2]:
                return False
    return True

def main():
    points = [l.split() for l in open(os.path.join(S, "points.txt")) if l.strip()]
    points = [p for p in points if os.path.isdir(os.path.join(S, "disasm", p[1]))]
    if len(sys.argv) > 1:  # the first issue of the window
        points = [p for p in points if p[2] >= sys.argv[1]]
    data, cases = load(points)
    levels = ["exact", "pc", "frames"]
    n = len(points)
    print(f"{n} points, {len(cases)} cases, {points[0][2]}..{points[-1][2]}")
    errs = {p[1]: sum(1 for c in cases if data[p[1]].get(c) is None) for p in points}
    print("points where some case does not compile:", {p[2]: errs[p[1]] for p in points if errs[p[1]]})
    print("\n## consecutive pairs with a change")
    print("from to | " + " | ".join(levels) + " | functions changed (exact)")
    breaks = defaultdict(int)
    for i in range(n - 1):
        a, b = points[i], points[i + 1]
        counts = []
        for lv in levels:
            c = sum(1 for cs in cases if not same(data[a[1]][cs], data[b[1]][cs], lv))
            counts.append(c)
            if c:
                breaks[lv] += 1
        if counts[0]:
            fch = sum(1 for cs in cases if data[a[1]][cs] and data[b[1]][cs]
                      for k in data[b[1]][cs] if data[a[1]][cs].get(k, (None,))[0] != data[b[1]][cs][k][0])
            print(f"{a[2]} {b[2]} {b[1]} | " + " | ".join(str(c) for c in counts) + f" | {fch}")
    print(f"\npairs: {n-1}; pairs with any change: " + ", ".join(f"{lv} {breaks[lv]}" for lv in levels))
    print("\n## survival over spans (fraction of scripts; mean over start points; all-scripts = share of start points where every script survives)")
    print("span | " + " | ".join(f"{lv} scripts | {lv} all" for lv in levels) + " | starts")
    for span in (1, 5, 10, 20, 30, 40, 60, 90):
        if span >= n:
            break
        row = []
        for lv in levels:
            fr, allok = [], 0
            for i in range(n - span):
                a, b = points[i][1], points[i + span][1]
                ok = sum(1 for cs in cases if same(data[a][cs], data[b][cs], lv))
                fr.append(ok / len(cases))
                allok += ok == len(cases)
            row.append(f"{100*sum(fr)/len(fr):.1f}% | {100*allok/(n-span):.0f}%")
        print(f"{span} | " + " | ".join(row) + f" | {n-span}")

    # Combined: a script survives when its functions survive at the `frames` level and
    # no codemode-visible schema change lies in the span (schema.py; such a change is
    # counted against every script, which overstates it).
    sys.path.insert(0, os.path.dirname(__file__))
    import schema
    snaps = {p[1]: schema.snapshot(p[1]) for p in points}
    cats = ["cells", "slots", "registry path", "closures", "records", "state consts"]
    print("\n## combined (frames level and no codemode-visible schema change)")
    print("span | scripts | all scripts")
    for span in (1, 5, 10, 20, 30, 40, 60, 90):
        if span >= n:
            break
        fr, allok = [], 0
        for i in range(n - span):
            a, b = points[i][1], points[i + span][1]
            if any(c in schema.breaks(snaps[a], snaps[b]) for c in cats):
                fr.append(0.0)
                continue
            ok = sum(1 for cs in cases if same(data[a][cs], data[b][cs], "frames"))
            fr.append(ok / len(cases))
            allok += ok == len(cases)
        print(f"{span} | {100*sum(fr)/len(fr):.1f}% | {100*allok/(n-span):.0f}%")

main()
