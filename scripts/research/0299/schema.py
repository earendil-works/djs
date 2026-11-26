#!/usr/bin/env python3
"""Schema stability proxies over merge points (issue 0299), read with `git show`, no builds.

For each point of work/scratch/0299/points.txt it reads the named schemas a logical
snapshot keyed by names would depend on, and for each consecutive pair counts the changes
such a snapshot would notice: a name that is gone (removed or renamed) or a field whose
declared type changed. Additions, offsets, ids and order do not count.
  cells     js/src/layout.generated.ts: cell kinds, their fields and types, the tail's
            element type, the value tags, flags, property flags, object class names;
  slots     heap/object.rs: each object class's internal slot count (`slots()`) and its doc
            comment, which is where the slots' meanings are written (a changed comment
            is a candidate, inspected by hand; a changed count is a change);
  registry  tests/builtins/registry.tsv: live paths (value entries) and intrinsic names;
            a tombstoned or dropped entry is gone. Split into user-reachable paths and
            the internal `%intrinsic:`/`%prelude:` function paths;
            Cells the snapshot never stores (Roots, atom/handle tables, Interp, Realm; code
            cells, which are recompiled) are listed apart and do not count. A field
            added to a stored cell kind, or a slot added to an object class, is listed
            apart too (`added`): a table-driven codec needs a default for it;
  closures  interp/run/intrinsics.rs BUILTIN_CLOSURES: the builtin closure kinds by name,
            and their order (the kind number is what `NewBuiltinClosure` sites pass);
  consts    named constants that spell out stored state's meaning (frame header, activation
            and promise states and slots, Map/Set layout, store flags): one gone or with
            another value is a change;
  ops       bytecode/ops.rs: the opcode table; an op whose byte moved, or whose
            immediates changed (reported apart: a name-normalized bytecode hash does
            not see op numbers, a hash of the raw bytes does);
  intrinsic ids  bytecode/intrinsics.rs: an intrinsic whose id moved (same);
  layout    the layout hash and field offsets, for contrast: what an image depends on and a
            snapshot by names does not;
  records   builtins/*.ts: per file, the `NewRecord(n)` sizes and the highest
            RecordGet/RecordSet index (a proxy for the records' slot meanings).
Prints per-pair changes and the survival over spans.
"""
import os, re, sys, json, subprocess
from collections import defaultdict

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
S = os.path.join(ROOT, "work/scratch/0299")

def show(sha, path):
    r = subprocess.run(["git", "-C", ROOT, "show", f"{sha}:{path}"], capture_output=True)
    return r.stdout.decode("utf-8", "replace") if r.returncode == 0 else None

def ls(sha, path):
    r = subprocess.run(["git", "-C", ROOT, "ls-tree", "--name-only", f"{sha}:{path}"], capture_output=True)
    return r.stdout.decode().split() if r.returncode == 0 else []

def cells(sha):
    t = show(sha, "js/src/layout.generated.ts")
    if t is None:
        return None
    j = json.loads(t[t.index("{"):t.rindex("}") + 1])
    out = {}
    for kind, c in j.get("cells", {}).items():
        out[f"cell {kind}"] = "cell"
        for f, d in c.get("fields", {}).items():
            out[f"cell {kind}.{f}"] = d.get("type")
        tail = c.get("tail")
        if tail:
            out[f"cell {kind}[{tail.get('name')}]"] = tail.get("elem")
            for f, d in (tail.get("fields") or {}).items():
                if f:
                    out[f"cell {kind}[].{f}"] = d.get("type")
    for k in j.get("value", {}).get("tags", {}):
        out[f"tag {k}"] = "tag"
    for k in j.get("flags", {}):
        out[f"flag {k}"] = "flag"
    for k in j.get("propFlags", {}):
        out[f"propflag {k}"] = "flag"
    for k in j.get("objectClasses", {}):
        out[f"class {k}"] = "class"
    return out

def slots(sha):
    t = show(sha, "crates/djs-core/src/heap/object.rs")
    if t is None:
        return None
    m = re.search(r"pub enum ObjectClass \{(.*?)\n\}", t, re.S)
    docs, doc = {}, []
    for line in m.group(1).splitlines():
        s = line.strip()
        if s.startswith("///"):
            doc.append(s[3:].strip())
        elif re.match(r"\w+ = \d+,", s):
            name = s.split()[0]
            docs[name] = " ".join(doc)
            doc = []
    counts = {}
    m = re.search(r"pub const fn slots\(self\) -> u32 \{\s*match self \{(.*?)\n        \}", t, re.S)
    if m:
        body = re.sub(r"//[^\n]*", "", m.group(1))
        for arm in re.finditer(r"((?:\|?\s*ObjectClass::\w+\s*)+)=>\s*(\d+)", body):
            for c in re.findall(r"ObjectClass::(\w+)", arm.group(1)):
                counts[c] = int(arm.group(2))
    return {"docs": docs, "counts": counts}

def registry(sha):
    t = show(sha, "tests/builtins/registry.tsv")
    if t is None:
        return None
    live = {}
    for line in t.splitlines():
        if line.startswith("#") or not line.strip():
            continue
        cols = line.split("\t")
        if len(cols) == 3:
            live[cols[2]] = (cols[1], int(cols[0]))
    return live

def closures(sha):
    t = show(sha, "crates/djs-core/src/interp/run/intrinsics.rs")
    m = t and re.search(r"BUILTIN_CLOSURES: \[u16; \d+\] = \[(.*?)\];", t, re.S)
    return re.findall(r'id\("(\w+)"\)', m.group(1)) if m else None

def records(sha):
    out = {}
    for f in ls(sha, "builtins"):
        if not f.endswith(".ts") or f.endswith(".d.ts"):
            continue
        t = show(sha, "builtins/" + f)
        news = sorted(int(n) for n in re.findall(r"NewRecord\((\d+)\)", t))
        idx = [int(n) for n in re.findall(r"Record(?:Get|Set)\([^,]+,\s*(\d+)", t)]
        if news or idx:
            out[f] = (tuple(news), max(idx) if idx else -1)
    return out

# Where the meaning of stored state is written as named constants: an activation's frame
# header and flags, generator/async states and slots, promise states, flags and slots,
# Map/Set entry layout, property store flags.
CONST_FILES = ["crates/djs-core/src/interp/frame.rs", "crates/djs-core/src/interp/run/suspend.rs",
               "crates/djs-core/src/interp/run/promise.rs", "crates/djs-core/src/heap/collection.rs",
               "crates/djs-core/src/heap/object.rs"]
CONST = re.compile(r"^\s*(?:pub(?:\([a-z]+\))? )?const ([A-Z][A-Z_0-9]*): (?:[iu]\d+|usize) = ([^;]+);", re.M)

def consts(sha):
    out = {}
    for f in CONST_FILES:
        t = show(sha, f)
        if t is None:
            continue
        for name, val in CONST.findall(t):
            out[f"{os.path.basename(f)}:{name}"] = " ".join(val.split())
    return out

OP = re.compile(r"^\s{4}[A-Z]\w* (\w+) ?\(([^)]*)\)\s+pop", re.M)

def ops(sha):
    """The opcode table in byte order: (disassembler name, immediate kinds)."""
    t = show(sha, "crates/djs-core/src/bytecode/ops.rs")
    if t is None or "ops! {" not in t:
        return None
    body = t[t.index("\nops! {"):]
    return [[m.group(1).rstrip("_"), " ".join(k.split(":")[-1].strip() for k in m.group(2).split(",") if k.strip())]
            for m in OP.finditer(body)]

def intrinsic_table(sha):
    t = show(sha, "crates/djs-core/src/bytecode/intrinsics.rs")
    m = t and re.search(r"pub const INTRINSICS: &\[IntrinsicInfo\] = &\[(.*?)\n\];", t, re.S)
    return re.findall(r'i\("(\w+)"', m.group(1)) if m else None

def layout_offsets(sha):
    t = show(sha, "js/src/layout.generated.ts")
    if t is None:
        return None
    j = json.loads(t[t.index("{"):t.rindex("}") + 1])
    return {"hash": j.get("layoutHash"), "offsets": {f"{k}.{f}": d.get("offset") for k, c in j.get("cells", {}).items()
            for f, d in c.get("fields", {}).items()}}

def snapshot(sha):
    path = os.path.join(S, "schema", sha + ".json")
    if os.path.exists(path):
        return json.load(open(path))
    d = {"cells": cells(sha), "slots": slots(sha), "registry": registry(sha),
         "closures": closures(sha), "records": records(sha), "consts": consts(sha),
         "ops": ops(sha), "intrinsics": intrinsic_table(sha), "layout": layout_offsets(sha)}
    os.makedirs(os.path.dirname(path), exist_ok=True)
    json.dump(d, open(path, "w"))
    return json.load(open(path))

# The VM's own cells: a logical snapshot saves the named parts it needs (the random state,
# the next call id), never these cells as such.
VM_CELLS = re.compile(r"^cell (Roots|AtomTable|HandleTable|Interp|Realm)\b")
# Code cells: a logical snapshot never stores templates; it compiles the source again.
CODE_CELLS = re.compile(r"^cell (FuncTemplate|Script|FuncInfo)\b")
INTERNAL = re.compile(r"^%(intrinsic|prelude):|~\d+$")

def breaks(a, b):
    """The name-visible changes from point a to point b, by category."""
    out = defaultdict(list)
    if a["cells"] and b["cells"]:
        for k, v in a["cells"].items():
            cat = "cells (vm)" if VM_CELLS.match(k) else "cells (code)" if CODE_CELLS.match(k) else "cells"
            if k not in b["cells"]:
                out[cat].append(f"gone {k}")
            elif b["cells"][k] != v:
                out[cat].append(f"type {k}: {v} -> {b['cells'][k]}")
        for k in b["cells"]:
            if k not in a["cells"] and k.split(".")[0].split("[")[0] in a["cells"] and not VM_CELLS.match(k) and not CODE_CELLS.match(k) and "." in k:
                out["cells added"].append(f"new {k}")
    if a["slots"] and b["slots"]:
        for c, n in a["slots"]["counts"].items():
            m = b["slots"]["counts"].get(c)
            if m is not None and m < n:
                out["slots"].append(f"count {c}: {n} -> {m}")
            elif m is not None and m > n:
                out["slots added"].append(f"count {c}: {n} -> {m}")
        for c, d in a["slots"]["docs"].items():
            e = b["slots"]["docs"].get(c)
            if e is not None and e != d:
                out["slot docs"].append(f"doc {c}")
    if a["registry"] and b["registry"]:
        for p, (kind, _) in a["registry"].items():
            if p not in b["registry"]:
                cat = "registry intrinsic" if kind == "intrinsic" else (
                    "registry internal" if INTERNAL.search(p) else "registry path")
                out[cat].append(f"gone {p}")
    if a["closures"] and b["closures"]:
        for i, n in enumerate(a["closures"]):
            if n not in b["closures"]:
                out["closures"].append(f"gone {n}")
        if b["closures"][:len(a["closures"])] != a["closures"]:
            out["closure order"].append("renumbered")
    if a.get("consts") and b.get("consts"):
        for k, v in a["consts"].items():
            w = b["consts"].get(k)
            if w is None:
                out["state consts"].append(f"gone {k}")
            elif w != v:
                out["state consts"].append(f"{k}: {v} -> {w}")
    if a.get("ops") and b.get("ops"):
        na = {n: (i, k) for i, (n, k) in enumerate(a["ops"])}
        nb = {n: (i, k) for i, (n, k) in enumerate(b["ops"])}
        moved = [n for n in na if n in nb and nb[n][0] != na[n][0]]
        enc = [n for n in na if n in nb and nb[n][1] != na[n][1]]
        gone = [n for n in na if n not in nb]
        if moved:
            out["op numbers"].append(f"{len(moved)} moved, e.g. {moved[0]}")
        if enc or gone:
            out["op encoding"].append(f"{len(enc)} other immediates, {len(gone)} gone: {(enc + gone)[:3]}")
    if a.get("intrinsics") and b.get("intrinsics"):
        ia = {n: i for i, n in enumerate(a["intrinsics"])}
        ib = {n: i for i, n in enumerate(b["intrinsics"])}
        moved = [n for n in ia if n in ib and ib[n] != ia[n]]
        if moved:
            out["intrinsic ids"].append(f"{len(moved)} moved")
    if a.get("layout") and b.get("layout"):
        if a["layout"]["hash"] != b["layout"]["hash"]:
            out["layout hash"].append("changed")
        if any(b["layout"]["offsets"].get(k, v) != v for k, v in a["layout"]["offsets"].items()):
            out["field offsets"].append("moved")
    if a["records"] is not None and b["records"] is not None:
        for f, v in a["records"].items():
            w = b["records"].get(f)
            if w is not None and [list(x) if isinstance(x, list) else x for x in w] != [list(x) if isinstance(x, list) else x for x in v]:
                out["records"].append(f"{f}: {v} -> {w}")
    return out

CATS = ["cells", "cells added", "cells (vm)", "cells (code)", "slots", "slots added", "slot docs", "registry path", "registry internal", "registry intrinsic",
        "closures", "closure order", "records", "state consts", "op numbers", "op encoding", "intrinsic ids", "layout hash", "field offsets"]

def main():
    points = [l.split() for l in open(os.path.join(S, "points.txt")) if l.strip()]
    if len(sys.argv) > 1:
        points = [p for p in points if p[2] >= sys.argv[1]]
    snaps = [snapshot(p[1]) for p in points]
    n = len(points)
    print(f"{n} points {points[0][2]}..{points[-1][2]}")
    per = []
    print("\n## pairs with a name-visible change")
    for i in range(n - 1):
        b = breaks(snaps[i], snaps[i + 1])
        per.append(b)
        if b:
            print(f"{points[i][2]} -> {points[i+1][2]} {points[i+1][1]}: " +
                  "; ".join(f"{c} {len(v)}: {', '.join(v[:4])}{' ...' if len(v) > 4 else ''}" for c, v in b.items()))
    print("\n## pairs (of %d) with a change, by category" % (n - 1))
    for c in CATS:
        print(f"{c}: {sum(1 for b in per if c in b)}")
    groups = {"strict (all but slot docs and VM cells)": [c for c in CATS if c not in ("slot docs", "cells (vm)", "cells (code)", "cells added", "slots added", "op numbers", "op encoding", "intrinsic ids", "layout hash", "field offsets")],
              "strict + additions (a table-driven codec needs a default for a new field)":
                  [c for c in CATS if c not in ("slot docs", "cells (vm)", "cells (code)", "op numbers", "op encoding", "intrinsic ids", "layout hash", "field offsets")],
              "raw bytecode ids (what a hash of the encoded bytes would also see)": ["op numbers", "op encoding", "intrinsic ids"],
              "codemode-visible (cells, slots, user paths, closures, records)":
                  ["cells", "slots", "registry path", "closures", "records", "state consts"]}
    built = sum(1 for i in range(n - 1) if subprocess.run(["git", "-C", ROOT, "diff", "--quiet", points[i][1], points[i + 1][1],
                "--", "crates/djs-core/src", "crates/djs-libm/src", "builtins", "crates/djs-prelude"]).returncode)
    print(f"\npairs where the engine sources changed (a build-keyed snapshot breaks): {built} of {n-1}")
    print("\n## survival over spans: share of start points with no change in the group")
    for g, cats in groups.items():
        row = []
        for span in (1, 5, 10, 20, 30):
            if span >= n:
                break
            ok = 0
            for i in range(n - span):
                b = breaks(snaps[i], snaps[i + span])
                ok += not any(c in b for c in cats)
            row.append(f"{span}: {100*ok/(n-span):.0f}%")
        print(f"{g}: " + ", ".join(row))

if __name__ == "__main__":
    main()
