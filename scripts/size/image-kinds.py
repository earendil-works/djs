#!/usr/bin/env python3
"""image-kinds.py BASE.img: the base image's cells by kind (count, raw bytes, gzip -9 of the
kind's cells alone) and the gzip of the whole image in its own order and regrouped by kind
(0122, D2251: the build orders it by kind now, so the two agree). Cell headers: kind byte
at +0, size u32 at +4, cells back to back from offset 0."""
import sys, zlib, collections
b = open(sys.argv[1], "rb").read()
gz = lambda d: len(zlib.compress(d, 9))
cells, off = [], 0
while off + 8 <= len(b):
    k = b[off]; size = int.from_bytes(b[off + 4:off + 8], "little")
    cells.append((off, k, size)); off += size
by = collections.defaultdict(list)
for o, k, s in cells:
    by[k].append(b[o:o + s])
print(f"image {len(b):,} bytes, {len(cells)} cells, gzip {gz(b):,}; regrouped by kind "
      f"{gz(b''.join(b''.join(v) for _, v in sorted(by.items()))):,}")
for k, v in sorted(by.items(), key=lambda kv: -sum(map(len, kv[1]))):
    d = b"".join(v)
    print(f"kind {k:3d} {len(v):6d} cells {len(d):8,} raw {gz(d):8,} gzip")
