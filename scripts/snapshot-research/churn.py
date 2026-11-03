# How often did each input of a snapshot change (0140)? Groups main's commits by issue (the
# "NNNN:" prefix: one merge each) and checks which paths each issue touched.
import re, subprocess, collections, sys
log = subprocess.run(["git", "log", "--reverse", "--format=@@%h %ad %s", "--date=short", "--name-only", "main"],
                     capture_output=True, text=True).stdout
issues = collections.OrderedDict()
cur = None
for line in log.splitlines():
    if line.startswith("@@"):
        h, date, subj = line[2:].split(" ", 2)
        m = re.match(r"(\d{4}):", subj)
        key = m.group(1) if m else None
        cur = key
        if key and key not in issues:
            issues[key] = {"date": date, "files": set(), "commits": 0}
        if key:
            issues[key]["commits"] += 1
    elif line.strip() and cur:
        issues[cur]["files"].add(line.strip())

def core_src(f):
    return f.startswith("crates/djs-core/src/") and f.endswith(".rs") and not f.endswith("tests.rs") and "/tests/" not in f
inputs = collections.OrderedDict([
    ("build hash (djs-core sources, prelude, features, libm)", lambda f: core_src(f) or f == "builtins/gen/prelude.js" or f == "crates/djs-core/Cargo.toml"),
    ("cell layouts (heap/cells.rs: layout hash)", lambda f: f == "crates/djs-core/src/heap/cells.rs"),
    ("well-known atoms (heap/atom.rs)", lambda f: f == "crates/djs-core/src/heap/atom.rs"),
    ("opcode table (bytecode/ops.rs)", lambda f: f == "crates/djs-core/src/bytecode/ops.rs"),
    ("intrinsic table (bytecode/intrinsics.rs)", lambda f: f == "crates/djs-core/src/bytecode/intrinsics.rs"),
    ("realm slots (interp/realm.rs)", lambda f: f == "crates/djs-core/src/interp/realm.rs"),
    ("builtins in JS (builtins/gen/prelude.js)", lambda f: f == "builtins/gen/prelude.js"),
    ("compiler (compiler/)", lambda f: f.startswith("crates/djs-core/src/compiler/") and core_src(f)),
    ("object/shape code (heap/object.rs, shape.rs)", lambda f: f in ("crates/djs-core/src/heap/object.rs", "crates/djs-core/src/heap/shape.rs")),
    ("base image making (heap/compact.rs, vm/image.rs, djs-prelude)", lambda f: f in ("crates/djs-core/src/heap/compact.rs", "crates/djs-core/src/vm/image.rs") or f.startswith("crates/djs-prelude/")),
    ("collector (heap/gc.rs, compactor.rs)", lambda f: f in ("crates/djs-core/src/heap/gc.rs", "crates/djs-core/src/heap/compactor.rs")),
    ("pi adapter (js/pi/vm.ts)", lambda f: f == "js/pi/vm.ts"),
    ("pi's prelude copy (tests/pi/prelude-source.ts)", lambda f: f == "tests/pi/prelude-source.ts"),
    ("snapshot container (js/src/snapshot.ts)", lambda f: f == "js/src/snapshot.ts"),
    ("ABI (crates/djs-wasm/src/lib.rs)", lambda f: f.startswith("crates/djs-wasm/src/")),
])
keys = list(issues)
def table(sel, title):
    n = len(sel)
    print(f"\n{title}: {n} issues ({issues[sel[0]]['date']} .. {issues[sel[-1]]['date']})")
    for name, pred in inputs.items():
        hit = [k for k in sel if any(pred(f) for f in issues[k]["files"])]
        print(f"  {name}: {len(hit)} of {n} ({100*len(hit)/n:.0f}%)")
if not sys.argv[1:2] or not sys.argv[1].startswith("--"):
    table(keys, "all issues on main")
    table(keys[-30:], "the last 30 issues")
    print("\nissue count", len(keys), "first", keys[0], "last", keys[-1])

# With two commits: which intrinsics keep their name but change their numeric id (the
# id is what an image's bytecode and builtin closures store).
if len(sys.argv) == 3 and sys.argv[1] != "--deps":
    def names(rev):
        src = subprocess.run(["git", "show", f"{rev}:crates/djs-core/src/bytecode/intrinsics.rs"], capture_output=True, text=True).stdout
        return re.findall(r'\bi\("(\w+)"', src[src.index("pub const INTRINSICS"):])
    a, b = names(sys.argv[1]), names(sys.argv[2])
    moved = [n for n in a if n in b and a.index(n) != b.index(n)]
    print(f"\nintrinsics {sys.argv[1]} -> {sys.argv[2]}: {len(a)} -> {len(b)}; {len(moved)} keep their name and change their id")
    for n in ["FunctionPrototypeBehavior", "PromiseResolveFunction", "PromiseRejectFunction", "PromiseAllResolveElement"]:
        if n in a and n in b:
            print(f"  {n}: {a.index(n)} -> {b.index(n)} (id {a.index(n)} is {b[a.index(n)]} on the newer build)")

# ---- D026 (0164): which merges change an image's dependency descriptor --------------------
#
# `churn.py --deps [BASE_TSV...]`: for every merge on main (a run of commits with one
# "NNNN:" prefix), whether each descriptor component changed, from the sources at the run's
# last
# commit: the cell layout (heap/cells.rs `records!`/`cells!`, heap/atom.rs `well_known!`),
# the opcode table (`ops!`), the intrinsic table (`INTRINSICS`), the internal id tables (the
# regions of crates/djs-core/compat_inputs.rs, cut the same way as build.rs does) and the
# image features (djs-core's [features]). The base image prefix comes from BASE_TSV
# (scripts/snapshot-research/base-churn.sh: the base image built at each merge, hashed
# without its roots cell) where it has the merge; elsewhere it is unknown. The text hashes
# approximate the build's: layout and ops count a changed doc-free macro body, features the
# default build's features. `churn.py --base-revs` lists the merges for base-churn.sh.

def _strip(src):
    out, i, n = [], 0, len(src)
    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif src.startswith("/*", i):
            depth = 0
            while i < n:
                if src.startswith("/*", i):
                    depth += 1; i += 2
                elif src.startswith("*/", i):
                    depth -= 1; i += 2
                    if depth == 0:
                        break
                else:
                    i += 1
        elif c == '"' or _raw_end(src, i) is not None:
            e = _raw_end(src, i) or _lit_end(src, i)
            out.append(src[i:e]); i = e
        elif c == "'" and _char_end(src, i) is not None:
            e = _char_end(src, i)
            out.append(src[i:e]); i = e
        else:
            if not c.isspace():
                out.append(c)
            i += 1
    return "".join(out)

def _ident(c):
    return c.isalnum() or c == "_"

def _lit_end(s, i):
    i += 1
    while i < len(s):
        if s[i] == "\\":
            i += 2
        elif s[i] == '"':
            return i + 1
        else:
            i += 1
    return len(s)

def _raw_end(s, i):
    if s[i] != "r" or (i > 0 and _ident(s[i - 1])):
        return None
    j = i + 1
    while j < len(s) and s[j] == "#":
        j += 1
    if j >= len(s) or s[j] != '"':
        return None
    close = '"' + "#" * (j - i - 1)
    e = s.find(close, j + 1)
    return len(s) if e < 0 else e + len(close)

def _char_end(s, i):
    j = i + 1
    if j < len(s) and s[j] == "\\":
        j += 2
        while j < len(s) and s[j] != "'" and j < i + 12:
            j += 1
    else:
        j += 1
    return j + 1 if j < len(s) and s[j] == "'" else None

def _item_end(s, i):
    depth = 0
    while i < len(s):
        e = _raw_end(s, i)
        if e is not None:
            i = e; continue
        c = s[i]
        if c == '"':
            i = _lit_end(s, i); continue
        if c == "'":
            e = _char_end(s, i)
            if e is not None:
                i = e; continue
        if c in "([{":
            depth += 1
        elif c in ")]":
            depth -= 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        elif c == ";" and depth == 0:
            return i + 1
        i += 1
    return None

def _cut(src, kind, start=None):
    if kind == "all":
        return _strip(src)
    if kind == "item":
        s, pat = _strip(src), _strip(start)
        hits = [m.start() for m in re.finditer(re.escape(pat), s)
                if not (_ident(pat[-1]) and m.end() < len(s) and _ident(s[m.end()]))
                and not (_ident(pat[0]) and m.start() > 0 and _ident(s[m.start() - 1]))]
        if len(hits) != 1:
            return ""
        e = _item_end(s, hits[0])
        return s[hits[0]:e] if e else ""
    out, at = [], 0
    for line in src.splitlines(keepends=True):
        for p in ("pub(crate) ", "pub(super) ", "pub ", ""):
            if line.startswith(p + "const "):
                rest = line[len(p) + 6:]
                if not rest.startswith("fn ") and not rest.startswith("_:"):
                    s = _strip(src[at:at + 20000])
                    e = _item_end(s, 0)
                    item = s[:e] if e else ""
                    # Intrinsic ids by name (`= id("Name")`): the intrinsic table's.
                    if not re.fullmatch(r'[^=]*=((\w+::)*i?)id\("[^"(]*"\);', item):
                        out.append(item)
                break
        at += len(line)
    return "".join(out)

def _regions():
    """compat_inputs.rs REGIONS, parsed from this tree's file."""
    import os
    src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../crates/djs-core/compat_inputs.rs")).read()
    body = src[src.index("pub const REGIONS"):]
    body = body[:body.index("];")]
    regs = []
    for m in re.finditer(r'\("([^"]+)",\s*Cut::(Item\("((?:[^"\\]|\\.)*)"\)|TopConsts|All)\)', body):
        path, kind, start = m.group(1), m.group(2), m.group(3)
        regs.append((path, "item" if kind.startswith("Item") else ("all" if kind == "All" else "consts"), start))
    return regs

def _default_features(section):
    """The features a default build enables: `default` and what it names, recursively."""
    table = {m.group(1): re.findall(r'"([a-z0-9-]+)"', m.group(2))
             for m in re.finditer(r'^([a-z][a-z0-9-]*)\s*=\s*\[([^\]]*)\]', section, re.M)}
    on, todo = set(), list(table.get("default", []))
    while todo:
        f = todo.pop()
        if f not in on and "/" not in f:
            on.add(f)
            todo += table.get(f, [])
    return on

# `--until=REV`: main's history up to REV (the numbers in docs/snapshot-versioning.md).
UNTIL = next((a.split("=", 1)[1] for a in sys.argv if a.startswith("--until=")), "main")

def deps_churn(base_tsv):
    import hashlib
    root = "crates/djs-core/"
    regs = _regions()
    log = subprocess.run(["git", "log", "--reverse", "--format=@@%H %s", "--name-only", UNTIL], capture_output=True, text=True).stdout
    runs = []
    for line in log.splitlines():
        if line.startswith("@@"):
            h, subj = (line[2:].split(" ", 1) + [""])[:2]
            m = re.match(r"(\d{4}):", subj)
            k = m.group(1) if m else "----"
            if runs and runs[-1][0] == k:
                runs[-1][1] = h
            else:
                runs.append([k, h, set()])
        elif line.strip():
            runs[-1][2].add(line.strip())
    base = {}
    for path in base_tsv or []:
        for line in open(path):
            f = line.split()
            if len(f) >= 2 and f[1] != "FAIL":
                base[f[0]] = f[1]
    cat = subprocess.Popen(["git", "cat-file", "--batch"], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    blobs = {}
    def blob(sha):
        if sha not in blobs:
            cat.stdin.write((sha + "\n").encode()); cat.stdin.flush()
            hdr = cat.stdout.readline().split()
            data = cat.stdout.read(int(hdr[2])); cat.stdout.read(1)
            blobs[sha] = data.decode("utf-8", "replace")
        return blobs[sha]
    cuts = {}
    def cut(sha, kind, start):
        k = (sha, kind, start)
        if k not in cuts:
            cuts[k] = _cut(blob(sha), kind, start)
        return cuts[k]
    def tree(commit):
        out = subprocess.run(["git", "ls-tree", "-r", commit, "--", "crates/djs-core/src", "crates/djs-core/Cargo.toml"], capture_output=True, text=True).stdout
        t = {}
        for line in out.splitlines():
            meta, path = line.split("\t", 1)
            t[path] = meta.split()[2]
        return t
    def H(parts):
        return hashlib.sha256("\0".join(parts).encode()).hexdigest()[:12]
    def comps(commit):
        t = tree(commit)
        g = lambda p, kind, start=None: cut(t[root + p], kind, start) if root + p in t else ""
        rec, detail = [], {}
        for path, kind, start in regs:
            files = sorted(p[len(root):] for p in t if p.startswith(root + path) and p.endswith(".rs") and not p.endswith("tests.rs")) if path.endswith("/") else [path]
            part = []
            for f in files:
                part.append(f + ":" + g(f, kind, start))
            rec += part
            detail[f"{path} {start or kind}"] = H(part)
        cargo = blob(t[root + "Cargo.toml"]) if root + "Cargo.toml" in t else ""
        feats = cargo[cargo.find("[features]"):]
        feats = feats[:feats.find("\n[", 1)] if "\n[" in feats[1:] else feats
        return {
            "layout": H([g("src/heap/cells.rs", "item", "records! {"), g("src/heap/cells.rs", "item", "cells! {"), g("src/heap/atom.rs", "item", "well_known! {")]),
            "features": H(sorted(_default_features(feats))),
            "ops": H([g("src/bytecode/ops.rs", "item", "ops! {")]),
            "intrinsics": H([g("src/bytecode/intrinsics.rs", "item", "pub const INTRINSICS: &[IntrinsicInfo]")]),
            "records": H(rec),
            "detail": detail,
        }
    def core_src(f):
        return f.startswith("crates/djs-core/src/") and f.endswith(".rs") and not f.endswith("tests.rs") and "/tests/" not in f
    build_pred = lambda f: core_src(f) or f == "builtins/gen/prelude.js" or f == "crates/djs-core/Cargo.toml"
    base_pred = lambda f: build_pred(f) or f.startswith("builtins/gen/") or f.startswith("crates/djs-prelude/") or f == "crates/djs-core/build.rs"
    rows, prev, prev_base = [], None, None
    for k, h, files in runs:
        touched = any(build_pred(f) for f in files)
        if prev is None:
            prev, prev_base = comps(h), base.get(h[:8])
            continue
        if touched or any(base_pred(f) for f in files):
            cur = comps(h)
        else:
            cur = prev
        b = base.get(h[:8]) if any(base_pred(f) for f in files) else prev_base
        row = {"issue": k, "commit": h[:8], "build": touched}
        for c in cur:
            row[c] = cur[c] != prev[c]
        row["detail"] = [r for r in cur["detail"] if cur["detail"][r] != prev["detail"].get(r)]
        row["prefix"] = None if (b is None or prev_base is None) else b != prev_base
        rows.append(row)
        prev, prev_base = cur, b
    cat.stdin.close()
    return rows

def deps_report(rows):
    comps = ["layout", "features", "ops", "intrinsics", "records"]
    # Per issue (as the table above): an issue changes a component if one of its merges does.
    issues_d = collections.OrderedDict()
    for r in rows:
        if r["issue"] == "----":
            continue
        d = issues_d.setdefault(r["issue"], {**{c: False for c in comps + ["build", "prefix"]}, "known": True})
        for c in comps + ["build"]:
            d[c] = d[c] or r[c]
        d["prefix"] = d["prefix"] or bool(r["prefix"])
    for r in rows:
        # Unknown: a merge that may have changed the base image, which was not built.
        if r["issue"] in issues_d and r["prefix"] is None and r["build"]:
            issues_d[r["issue"]]["known"] = False
    keys = list(issues_d)
    def show(sel, title):
        n = len(sel)
        built = sum(issues_d[k]["build"] for k in sel)
        print(f"\n{title}: {n} issues ({sel[0]}..{sel[-1]})")
        print(f"  engine build hash changed: {built}")
        for c in comps:
            print(f"  {c} changed: {sum(issues_d[k][c] for k in sel)}")
        desc = [k for k in sel if any(issues_d[k][c] for c in comps)]
        print(f"  survive (no component changed): under the build hash {n - built} ({100*(n-built)/n:.0f}%); "
              f"a raw image under the descriptor {n - len(desc)} ({100*(n-len(desc))/n:.0f}%)", end="")
        if all(issues_d[k]["known"] for k in sel):
            pre = [k for k in sel if issues_d[k]["prefix"]]
            both = [k for k in sel if issues_d[k]["prefix"] or any(issues_d[k][c] for c in comps)]
            print(f"; a container (+ base prefix, changed in {len(pre)}) {n - len(both)} ({100*(n-len(both))/n:.0f}%)")
        else:
            print("; a container: base images not built for all")
    show(keys, "all issues on main")
    # The longest run of issues up to now whose base images were all built.
    i = len(keys)
    while i > 0 and issues_d[keys[i - 1]]["known"]:
        i -= 1
    if 0 < i < len(keys):
        show(keys[i:], "issues with every base image built")
    show(keys[-30:], "the last 30 issues")
    print("\nrecords: issues that changed each region")
    by = collections.Counter()
    for k in keys:
        regions = set(x for r in rows if r["issue"] == k for x in r["detail"])
        by.update(regions)
    for region, n in by.most_common():
        print(f"  {n:3} {region}")
    print("\nper merge (issue commit: changed components)")
    for r in rows[-80:]:
        ch = [c for c in comps if r[c]] + (["prefix"] if r["prefix"] else []) + (["prefix?"] if r["prefix"] is None and r["build"] else [])
        if r["build"] or ch:
            print(f"  {r['issue']} {r['commit']}: build{'*' if r['build'] else ' '} {' '.join(ch) or '-'}")

if len(sys.argv) >= 2 and sys.argv[1] == "--deps":
    deps_report(deps_churn([a for a in sys.argv[2:] if not a.startswith("--")]))
if len(sys.argv) >= 2 and sys.argv[1] == "--base-revs":
    # The merges that may change the base image: for base-churn.sh.
    log = subprocess.run(["git", "log", "--reverse", "--format=@@%h %s", "--name-only", "main"], capture_output=True, text=True).stdout
    runs = []
    for line in log.splitlines():
        if line.startswith("@@"):
            h, subj = (line[2:].split(" ", 1) + [""])[:2]
            m = re.match(r"(\d{4}):", subj)
            k = m.group(1) if m else "----"
            if runs and runs[-1][0] == k:
                runs[-1][1] = h
            else:
                runs.append([k, h, False])
        elif line.strip():
            f = line.strip()
            runs[-1][2] |= ((f.startswith("crates/djs-core/") and f.endswith(".rs") and not f.endswith("tests.rs"))
                            or f.startswith("builtins/gen/") or f.startswith("crates/djs-prelude/") or f == "crates/djs-core/Cargo.toml")
    print(" ".join(h for _, h, rel in runs if rel))
