#!/usr/bin/env python3
"""families.py MODULE.wasm [--top N]: code bytes per generic family: function names with
their type parameters and closure suffixes removed, summed over instantiations, for
families with 3 or more (monomorphization, 0122). Needs a module with a name section
(CARGO_PROFILE_WASM_STRIP=debuginfo), wasm-objdump and rustfilt."""
import subprocess, re, sys, collections
top = int(sys.argv[sys.argv.index("--top") + 1]) if "--top" in sys.argv else 40
out = subprocess.run(["wasm-objdump", "-x", "-j", "Code", sys.argv[1]], capture_output=True, text=True).stdout
rows = [(int(m.group(1)), m.group(2)) for m in re.finditer(r"size=(\d+) <(.+)>", out)]
names = subprocess.run(["rustfilt"], input="\n".join(n for _, n in rows), capture_output=True, text=True).stdout.split("\n")
fam = collections.defaultdict(lambda: [0, 0])
for (s, _), n in zip(rows, names):
    base = n
    for _ in range(6):
        base = re.sub(r"<[^<>]*>", "", base)
    base = re.sub(r"::\{closure#\d+\}", "", base)
    fam[base][0] += s; fam[base][1] += 1
for s, c, k in sorted(((v[0], v[1], k) for k, v in fam.items() if v[1] >= 3), reverse=True)[:top]:
    print(f"{s:8,} {c:4d} {k[:120]}")
