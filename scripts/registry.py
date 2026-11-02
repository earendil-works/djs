#!/usr/bin/env python3
"""The builtin registry's history checks (D2371), tests/builtins/registry.tsv:

  scripts/registry.py check [REF]   fail if the file renumbers, drops or rewrites a line of
                                    REF's version (default main): every line there must be
                                    here unchanged, or tombstoned (`removed: ...` added)
  scripts/registry.py merge [REF]   after a merge or rebase that unioned two sides' appended
                                    lines: keep REF's lines (a tombstone wins), then the
                                    lines appended after them, deduplicated by kind and path
                                    and renumbered in order

Whether the registry is *current* (every builtin has an entry) is the djs-prelude test's
job; this only guards the append-only property, which needs the history.
"""
import subprocess
import sys

FILE = "tests/builtins/registry.tsv"


def parse(text):
    """[(id, kind, path, removed or None)] and the header lines."""
    header, rows = [], []
    for line in text.splitlines():
        if not line or line.startswith("#"):
            header.append(line)
            continue
        cols = line.split("\t")
        removed = cols[3] if len(cols) > 3 else None
        rows.append((int(cols[0]), cols[1], cols[2], removed))
    return header, rows


def base_text(ref):
    r = subprocess.run(["git", "show", f"{ref}:{FILE}"], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def check(ref):
    old = base_text(ref)
    if old is None:
        print(f"registry: {ref} has no {FILE} yet; nothing to compare")
        return 0
    _, old_rows = parse(old)
    _, new_rows = parse(open(FILE).read())
    errors = []
    for i, (oid, kind, path, removed) in enumerate(old_rows):
        if i >= len(new_rows):
            errors.append(f"id {oid} ({kind} {path}) was dropped")
            continue
        nid, nkind, npath, nremoved = new_rows[i]
        if (nid, nkind, npath) != (oid, kind, path):
            errors.append(f"id {oid} ({kind} {path}) became {nid} ({nkind} {npath})")
        elif removed is not None and nremoved != removed:
            errors.append(f"id {oid} ({kind} {path}) lost or changed its tombstone")
    for i, row in enumerate(new_rows):
        if row[0] != i:
            errors.append(f"line for id {row[0]} is at position {i}: ids must be 0, 1, 2, ...")
            break
    if errors:
        print("registry: the builtin registry is append-only (D2371):")
        for e in errors:
            print(f"  {e}")
        print("  ids are never renumbered or dropped; a removed builtin keeps its line as a "
              "tombstone (scripts/registry-update.sh does both)")
        return 1
    print(f"registry: append-only against {ref}: ok ({len(old_rows)} -> {len(new_rows)} entries)")
    return 0


def merge(ref):
    old = base_text(ref)
    header, rows = parse(open(FILE).read())
    if old is None:
        return 0
    _, old_rows = parse(old)
    out = []
    for oid, kind, path, removed in old_rows:
        here = [r for r in rows if r[0] == oid and r[1] == kind and r[2] == path]
        tomb = next((r[3] for r in here if r[3] is not None), removed)
        out.append((oid, kind, path, tomb))
    seen = {(k, p) for _, k, p, r in out if r is None}
    old_keys = {(i, k, p) for i, k, p, _ in old_rows}
    for i, kind, path, removed in rows:
        if (i, kind, path) in old_keys or (removed is None and (kind, path) in seen):
            continue
        if removed is None:
            seen.add((kind, path))
        out.append((len(out), kind, path, removed))
    text = "\n".join(h for h in header if h) + "\n"
    for i, kind, path, removed in out:
        text += f"{i}\t{kind}\t{path}" + (f"\t{removed}" if removed else "") + "\n"
    open(FILE, "w").write(text)
    return 0


def main():
    args = sys.argv[1:]
    cmd = args[0] if args else "check"
    ref = args[1] if len(args) > 1 else "main"
    sys.exit({"check": check, "merge": merge}[cmd](ref))


main()
