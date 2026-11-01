#!/usr/bin/env python3
"""Records measured status numbers in tests/scores/status.tsv (D2101).

README.md and the guide show them through `djs-docs` generated regions (`gen:status`),
which only read the file: numbers that need a build, a benchmark or another repository
are measured here, on request, and checked in with the commit they were measured at.
Counts that follow from the repository (test262 groups, ratchets, corpus cases, the base
snapshot) are computed by djs-docs itself and need no recording.

usage:
  scripts/status.py size              build djs-wasm (scripts/build-wasm.sh) and record its
                                      size: bytes, gzip -9, brotli -q 11, code and data
  scripts/status.py speed [-n N]      scripts/perf/wmeasure.py on the README workloads:
                                      instructions of djs-wasm and quickjs-wasi (needs
                                      scripts/perf/setup.sh; takes minutes)
  scripts/status.py set [--at WHEN] KEY VALUE...
                                      record values measured elsewhere, for example
                                      `set pi/passed 221 pi/total 226` after
                                      `node js/pi/pi-tests.ts`; --at names an older
                                      measurement (an issue or commit) instead of HEAD
"""
import datetime
import gzip
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
FILE = os.path.join(ROOT, "tests/scores/status.tsv")
WASM = os.path.join(ROOT, "target/wasm32-unknown-unknown/wasm/djs_wasm.wasm")
HEADER = [
    "# Measured status numbers (D2101): `scripts/status.py` writes this file, and the",
    "# `gen:status` regions of README.md and docs/guide read it. Do not edit by hand.",
    "key\tvalue\tcommit\tdate",
]
# The workloads of README.md's speed table (scripts/perf/w/), in table order.
SPEED = ["tight", "prop1", "strings", "arrays", "json", "json-date", "forof", "calls",
         "push-pop", "pi-like", "foreach", "higher-order"]


def load():
    rows = {}
    if os.path.exists(FILE):
        for line in open(FILE, encoding="utf-8"):
            line = line.rstrip("\n")
            if not line or line.startswith("#") or line.startswith("key\t"):
                continue
            k, v, c, d = line.split("\t")
            rows[k] = (v, c, d)
    return rows


def save(rows, new, at=None):
    commit = at or subprocess.run(["git", "rev-parse", "--short=10", "HEAD"], cwd=ROOT,
                            capture_output=True, text=True, check=True).stdout.strip()
    dirty = subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"], cwd=ROOT,
                           capture_output=True, text=True, check=True).stdout.strip()
    if dirty and not at:
        commit += "-dirty"
    date = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
    for k, v in new.items():
        rows[k] = (str(v), commit, date)
        print(f"{k}\t{v}")
    with open(FILE, "w", encoding="utf-8") as f:
        f.write("\n".join(HEADER) + "\n")
        for k in sorted(rows):
            f.write("\t".join([k, *rows[k]]) + "\n")


def leb(b, i):
    r = shift = 0
    while True:
        x = b[i]
        i += 1
        r |= (x & 0x7F) << shift
        shift += 7
        if x < 0x80:
            return r, i


def section_sizes(b):
    """Payload bytes of the code (10) and data (11) sections."""
    out = {}
    i = 8
    while i < len(b):
        sid = b[i]
        n, j = leb(b, i + 1)
        out[sid] = out.get(sid, 0) + n
        i = j + n
    return out.get(10, 0), out.get(11, 0)


def size():
    subprocess.run(["nice", "-n", "10", os.path.join(ROOT, "scripts/build-wasm.sh")], check=True)
    b = open(WASM, "rb").read()
    code, data = section_sizes(b)
    new = {
        "wasm/bytes": len(b),
        "wasm/gzip": len(gzip.compress(b, compresslevel=9, mtime=0)),
        "wasm/code": code,
        "wasm/data": data,
    }
    if shutil.which("brotli"):
        r = subprocess.run(["brotli", "-c", "-q", "11"], input=b, capture_output=True, check=True)
        new["wasm/brotli"] = len(r.stdout)
    return new


def speed(n):
    out = subprocess.run(
        ["python3", os.path.join(ROOT, "scripts/perf/wmeasure.py"), "-n", str(n), "--wasm", WASM, *SPEED],
        cwd=ROOT, capture_output=True, text=True, check=True).stdout
    new = {}
    for line in out.splitlines():
        m = re.match(r"(\S+)\s+(djs-wasm|qjs-wasi)\s+([\d.]+)\s", line)
        if m:
            new[f"speed/{m.group(1)}/{m.group(2)}"] = m.group(3)
    missing = [w for w in SPEED if f"speed/{w}/djs-wasm" not in new or f"speed/{w}/qjs-wasi" not in new]
    if missing:
        sys.exit(f"status.py: wmeasure gave no numbers for {', '.join(missing)}:\n{out}")
    return new


def main():
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    rows = load()
    if args[0] == "size":
        save(rows, size())
    elif args[0] == "speed":
        n = int(args[2]) if len(args) > 2 and args[1] == "-n" else 3
        subprocess.run(["nice", "-n", "10", os.path.join(ROOT, "scripts/build-wasm.sh")], check=True)
        save(rows, speed(n))
    elif args[0] == "set":
        at = None
        if len(args) > 2 and args[1] == "--at":
            at, args = args[2], args[2:]
        kv = args[1:]
        if not kv or len(kv) % 2:
            sys.exit(__doc__)
        save(rows, dict(zip(kv[0::2], kv[1::2])), at)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
