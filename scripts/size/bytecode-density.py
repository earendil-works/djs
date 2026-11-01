#!/usr/bin/env python3
"""Bytecode bytes for the same source in djs and QuickJS (0123).

djs: `djs disasm` (release djs-cli), summing each function's last instruction offset + 1
(the last instruction is a one-byte return in practice). QuickJS: a native build of
work/scratch/0123/qjs-src/bcsize.c (the dquickts sources copied there; see
docs/size-vs-quickjs.md), which compiles without running and reports the bytecode bytes
from JS_ComputeMemoryUsage. Line tables are excluded on both sides.

usage: scripts/size/bytecode-density.py QJS_BCSIZE FILE.js...
"""
import re
import subprocess
import sys

bc, files = sys.argv[1], sys.argv[2:]
djs = "target/release/djs"
print(f"{'file':40} {'djs funcs':>9} {'djs bytes':>10} {'qjs funcs':>9} {'qjs bytes':>10} {'ratio':>6}")
for f in files:
    out = subprocess.run([djs, "disasm", f], capture_output=True, text=True).stdout
    funcs, total, last = 0, 0, None
    for line in out.splitlines():
        if line.startswith("function "):
            if last is not None:
                total += last + 1
            funcs += 1
            last = None
            continue
        m = re.match(r"\s+(\d+)  ", line)
        if m:
            last = int(m.group(1))
    if last is not None:
        total += last + 1
    q = subprocess.run([bc, f], capture_output=True, text=True).stdout
    m = re.search(r"functions (\d+) code (\d+)", q)
    qf, qb = (int(m.group(1)), int(m.group(2))) if m else (0, 0)
    print(f"{f[-40:]:40} {funcs:>9} {total:>10,} {qf:>9} {qb:>10,} {total / qb if qb else 0:>6.2f}")
