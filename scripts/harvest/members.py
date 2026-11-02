#!/usr/bin/env python3
"""Builtin usage by global and member (0149, docs/slim-codemode.md).

Joins features.py's per-execution features with the builtin registry
(tests/builtins/registry.tsv, D2371): for every builtin function or accessor that a
global reaches, how many distinct scripts name it. Statics (`Object.keys`) are matched
exactly (`static:Object.keys`); prototype methods and getters only by name (`.map`,
`.size`), since the receiver is unknown statically, so their counts are an upper bound.
Symbol-keyed members (`[Symbol.iterator]`) are reached through protocols and are not
counted here. With --touched, adds the number of corpus cases whose run touched the
member (the 0143 touch build, named by work/scratch's touched-names example).

    scripts/harvest/members.py FEATURES.jsonl [--touched TOUCHED.tsv] [--families]

Prints a markdown table per global (or one line per global with --families). Counts
only: no script text.
"""
import collections
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))


def registry():
    out = []
    for line in open(os.path.join(ROOT, "tests/builtins/registry.tsv")):
        if line.startswith("#") or not line.strip():
            continue
        cols = line.rstrip("\n").split("\t")
        if len(cols) >= 3 and cols[1] == "value" and len(cols) == 3:
            out.append(cols[2])
    return out


def member(path):
    """(global, kind, name) for `X.y`, `X.prototype.y`, `get X.prototype.y`; None for
    globals themselves, symbol keys, captures and hidden intrinsics."""
    acc = ""
    if path.startswith("get ") or path.startswith("set "):
        acc, path = path[:3], path[4:]
    if "[" in path or "~" in path or path.startswith("%"):
        return None
    parts = path.split(".")
    if len(parts) == 2:
        return parts[0], "static" if not acc else acc + " static", parts[1]
    if len(parts) == 3 and parts[1] == "prototype":
        return parts[0], "proto" if not acc else acc + " proto", parts[2]
    return None


def main():
    args = sys.argv[1:]
    touched = {}
    if "--touched" in args:
        i = args.index("--touched")
        for line in open(args[i + 1]):
            if line.startswith("#"):
                continue
            n, _, p = line.rstrip("\n").split("\t")
            touched[p] = max(touched.get(p, 0), int(n))
        del args[i:i + 2]
    families = "--families" in args
    rows = [json.loads(line) for line in open(args[0])]
    scripts = collections.defaultdict(set)
    for r in rows:
        for f in r["features"]:
            scripts[f].add(r["script"])
    total = len({r["script"] for r in rows})
    by_global = collections.defaultdict(list)
    globals_ = set()
    for p in registry():
        if re.fullmatch(r"[A-Za-z_$][\w$]*", p):
            globals_.add(p)
            continue
        m = member(p)
        if not m:
            continue
        g, kind, name = m
        if kind.endswith("static"):
            used = scripts.get(f"static:{g}.{name}", set())
        else:
            used = scripts.get(f"call:.{name}", set()) | scripts.get(f"read:.{name}", set())
        by_global[g].append((p, kind, len(used), touched.get(p, 0)))
    print(f"{total} distinct scripts\n")
    if families:
        print("| global | scripts naming it | members | members named in any script | members touched by any case |")
        print("| --- | ---: | ---: | ---: | ---: |")
        for g in sorted(globals_ | set(by_global)):
            ms = by_global.get(g, [])
            named = sum(1 for m in ms if m[2])
            tch = sum(1 for m in ms if m[3])
            print(f"| {g} | {len(scripts.get('global:' + g, set()))} | {len(ms)} | {named} | {tch} |")
        return
    for g in sorted(by_global):
        print(f"\n### {g} (global named in {len(scripts.get('global:' + g, set()))} scripts)\n")
        print("| member | kind | scripts (by name) | cases touching it |")
        print("| --- | --- | ---: | ---: |")
        for p, kind, n, t in sorted(by_global[g], key=lambda x: (-x[2], -x[3], x[0])):
            print(f"| `{p}` | {kind} | {n} | {t} |")


if __name__ == "__main__":
    main()
