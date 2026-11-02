#!/usr/bin/env python3
"""datamap.py MAP.txt MODULE.wasm [--top N]: the data section by symbol and by category
(0156): the coded base image, Unicode tables, other named statics, and anonymous
constants split into text (messages, names) and binary (lookup tables), raw and gzip -9.

MAP.txt is wasm-ld's link map of the same build (no other changes):
  OUT=work/scratch/x RUSTFLAGS="-Clink-arg=--Map=$PWD/work/scratch/x/map.txt" \\
    CARGO_PROFILE_WASM_STRIP=debuginfo scripts/size/variant.sh mapped
then datamap.py work/scratch/x/map.txt work/scratch/x/mapped.wasm. Needs rustfilt."""
import collections
import re
import subprocess
import sys
import zlib

top = int(sys.argv[sys.argv.index("--top") + 1]) if "--top" in sys.argv else 25
lines = open(sys.argv[1]).read().split("\n")
first = next(i for i, l in enumerate(lines) if re.search(r"\s\.(rodata|data)$", l))
syms = []
for l in lines[first:]:
    m = re.match(r"\s+([0-9a-f]+)\s+([0-9a-f-]+)\s+([0-9a-f]+)\s{17}(\S.*)$", l)
    if m and not m.group(4).startswith(("/", "<")):
        syms.append((int(m.group(1), 16), int(m.group(3), 16), m.group(4)))
names = subprocess.run(["rustfilt"], input="\n".join(s[2] for s in syms),
                       capture_output=True, text=True).stdout.split("\n")


def leb(b, i):
    r = s = 0
    while True:
        x = b[i]
        i += 1
        r |= (x & 0x7F) << s
        s += 7
        if x < 0x80:
            return r, i


# The first active data segment and its address.
b = open(sys.argv[2], "rb").read()
i = 8
while i < len(b):
    sid = b[i]
    size, j = leb(b, i + 1)
    if sid == 11:
        data = b[j:j + size]
    i = j + size
_, k = leb(data, 0)
_, k = leb(data, k)
assert data[k] == 0x41  # i32.const
addr, k = leb(data, k + 1)
assert data[k] == 0x0B
seg_len, k = leb(data, k + 1)
seg = data[k:k + seg_len]

cats = collections.defaultdict(bytearray)
rows = []
for (a, sz, _), d in sorted(zip(syms, names)):
    v = seg[a - addr:a - addr + sz]
    if a < addr or len(v) != sz:
        continue
    rows.append((sz, d))
    if d.startswith(".Lanon"):
        if sz > 32768 and v[:1] != b"\0":
            c = "base image (coded), by size"
        elif sum(32 <= x < 127 for x in v) > 0.85 * sz:
            c = "anonymous text: messages, names"
        else:
            c = "anonymous binary: lookup tables"
    elif "unicode::" in d:
        c = "Unicode tables"
    else:
        c = "other named statics"
    cats[c] += v
gz = lambda d: len(zlib.compress(bytes(d), 9))
print(f"data segment {len(seg):,} bytes, gzip {gz(seg):,}")
for c, v in sorted(cats.items(), key=lambda kv: -len(kv[1])):
    print(f"{len(v):9,} {gz(v):8,} gz  {c}")
print()
for sz, d in sorted(rows, reverse=True)[:top]:
    print(f"{sz:9,}  {d[:100]}")
