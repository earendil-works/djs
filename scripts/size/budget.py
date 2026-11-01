#!/usr/bin/env python3
"""budget.py MODULE.wasm [--update]: the gate's size check (D020, 0122).

The budget is gzip -9 of the shipped module (scripts/build-wasm.sh) within 1.5x
quickjs-wasi's, recorded in tests/scores/size.tsv (the gate's machine needs no copy of
quickjs-wasi). Until the module is within the budget, a ratchet holds it there: the gate
fails when the module's gzip exceeds the recorded ceiling. `--update` lowers the ceiling to
this module's gzip plus the slack (never raises it: raising it is a recorded decision and
an edit of the file)."""
import gzip, os, sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
FILE = os.path.join(ROOT, "tests/scores/size.tsv")
SLACK = 1024  # bytes: gzip moves a little with unrelated edits

def read():
    vals, lines = {}, open(FILE).read().splitlines()
    for l in lines:
        if l and not l.startswith("#"):
            k, v = l.split("\t")[:2]
            vals[k] = int(v)
    return vals, lines

# Build-machine paths that must not reach the shipped module (D2331): panic locations
# carry source paths, scripts/wasm-rustflags.sh remaps them.
LEAKS = [b"/Users/", b"/home/", b".cargo/registry", b".rustup/", b"C:\\Users\\"]


def leaks(data):
    found = []
    for pat in LEAKS:
        i = data.find(pat)
        if i >= 0:
            start = max(data.rfind(b"\0", 0, i) + 1, i - 80)
            found.append(data[start:i + 60].split(b"\0")[0].decode("latin1"))
    return found


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    data = open(args[0], "rb").read()
    bad = leaks(data)
    if bad:
        print("size: FAIL: the module contains build-machine paths (D2331; build it with "
              "scripts/wasm-rustflags.sh):")
        for b in bad:
            print(f"  {b}")
        sys.exit(1)
    size = len(gzip.compress(data, 9, mtime=0))
    vals, lines = read()
    qjs, ceiling = vals["quickjs_wasi_gzip"], vals["ceiling_gzip"]
    budget = qjs * 3 // 2
    print(f"size: {size:,} bytes gzip ({len(data):,} raw), {size / qjs:.3f}x quickjs-wasi "
          f"({qjs:,}); budget 1.5x = {budget:,} (D020), ratchet ceiling {ceiling:,}")
    if "--update" in sys.argv:
        new = min(ceiling, max(size + SLACK, 0))
        if new < ceiling:
            out = [l if not l.startswith("ceiling_gzip\t") else f"ceiling_gzip\t{new}" for l in lines]
            open(FILE, "w").write("\n".join(out) + "\n")
            print(f"size: ceiling lowered to {new:,}")
        return
    limit = min(ceiling, budget) if size <= budget else ceiling
    if size > limit:
        print(f"size: FAIL: {size:,} > {limit:,} (D020; see TESTING.md \"Size budget\")")
        sys.exit(1)
    if size + SLACK < ceiling:
        print(f"size: {ceiling - size:,} bytes below the ceiling: lower it with "
              f"`scripts/size/budget.py {args[0]} --update`")

main()
