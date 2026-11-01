#!/usr/bin/env python3
"""sections.py MODULE.wasm [BASE.img] [--qjs QUICKJS.wasm]: gzip -9 of the whole module and
of its code and data sections, and of the data without the base image (found by content
when the image is given), against quickjs-wasi's module (0122)."""
import sys, zlib, os

def leb(b, i):
    r = s = 0
    while True:
        x = b[i]; i += 1; r |= (x & 0x7F) << s; s += 7
        if x < 0x80:
            return r, i

def sections(b):
    i, out = 8, {}
    while i < len(b):
        sid = b[i]; size, j = leb(b, i + 1); out[sid] = b[j:j + size]; i = j + size
    return out

gz = lambda d: len(zlib.compress(d, 9))
args = [a for a in sys.argv[1:] if not a.startswith("--")]
qjs = os.path.expanduser("~/Development/pi-private/node_modules/quickjs-wasi/quickjs.wasm")
if "--qjs" in sys.argv:
    qjs = sys.argv[sys.argv.index("--qjs") + 1]
    args.remove(qjs)
m = open(args[0], "rb").read()
s = sections(m)
print(f"module   raw {len(m):9,}  gzip {gz(m):9,}")
print(f"code     raw {len(s[10]):9,}  gzip {gz(s[10]):9,}")
print(f"data     raw {len(s[11]):9,}  gzip {gz(s[11]):9,}")
if len(args) > 1:
    img = open(args[1], "rb").read()
    k = s[11].find(img[:256])
    if k >= 0:
        rest = s[11][:k] + s[11][k + len(img):]
        print(f"  image  raw {len(img):9,}  gzip {gz(img):9,}")
        print(f"  rest   raw {len(rest):9,}  gzip {gz(rest):9,}")
if os.path.exists(qjs):
    q = open(qjs, "rb").read()
    print(f"quickjs-wasi gzip {gz(q):,}: ratio {gz(m) / gz(q):.3f} (budget 1.5, D020)")
