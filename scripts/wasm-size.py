#!/usr/bin/env python3
"""Size attribution of a djs wasm module (0043): sections, then code bytes per crate,
per module path and per function, from the code and name sections. Demangles with
`rustfilt` when it is on PATH (names stay mangled otherwise).

usage: scripts/wasm-size.py [module.wasm] [--top N] [--depth D]
  default module: target/wasm32-unknown-unknown/wasm/djs_wasm.wasm
  --depth D: module path components to group by (default 3: crate::a::b)
"""
import re
import shutil
import subprocess
import sys
from collections import defaultdict


def leb(b, i):
    r = shift = 0
    while True:
        x = b[i]
        i += 1
        r |= (x & 0x7F) << shift
        shift += 7
        if x < 0x80:
            return r, i


def sections(b):
    i = 8
    while i < len(b):
        sid = b[i]
        size, j = leb(b, i + 1)
        name = None
        if sid == 0:
            n, k = leb(b, j)
            name = b[k:k + n].decode()
        yield sid, name, j, j + size
        i = j + size


SECTION_NAMES = {1: "type", 2: "import", 3: "function", 4: "table", 5: "memory", 6: "global",
                 7: "export", 8: "start", 9: "elem", 10: "code", 11: "data", 12: "datacount"}


def main():
    args = sys.argv[1:]
    top, depth = 40, 3
    path = "target/wasm32-unknown-unknown/wasm/djs_wasm.wasm"
    while args:
        a = args.pop(0)
        if a == "--top":
            top = int(args.pop(0))
        elif a == "--depth":
            depth = int(args.pop(0))
        else:
            path = a
    b = open(path, "rb").read()
    print(f"{path}: {len(b):,} bytes\n")
    print("## sections\n")
    imports = 0
    bodies, names, data = [], {}, []
    for sid, name, start, end in sections(b):
        label = name if sid == 0 else SECTION_NAMES.get(sid, str(sid))
        print(f"{label:>16} {end - start:>10,}")
        if sid == 2:
            n, i = leb(b, start)
            for _ in range(n):
                for _ in range(2):
                    l, i = leb(b, i)
                    i += l
                kind = b[i]
                i += 1
                if kind == 0:
                    _, i = leb(b, i)
                    imports += 1
                else:
                    raise SystemExit("unsupported import kind")
        elif sid == 10:
            n, i = leb(b, start)
            for _ in range(n):
                size, j = leb(b, i)
                bodies.append(size + (j - i))
                i = j + size
        elif sid == 11:
            n, i = leb(b, start)
            for _ in range(n):
                flags, i = leb(b, i)
                if flags == 0:
                    # an i32.const offset expression: opcode, sleb, end
                    i += 1
                    while b[i] & 0x80:
                        i += 1
                    i += 2
                size, i = leb(b, i)
                data.append(size)
                i += size
        elif sid == 0 and name == "name":
            n, i = leb(b, start)
            i += n
            while i < end:
                sub = b[i]
                size, j = leb(b, i + 1)
                if sub == 1:
                    count, k = leb(b, j)
                    for _ in range(count):
                        idx, k = leb(b, k)
                        l, k = leb(b, k)
                        names[idx] = b[k:k + l].decode(errors="replace")
                        k += l
                i = j + size
    print(f"\ndata segments: {', '.join(f'{d:,}' for d in data)}")
    mangled = [names.get(imports + k, f"func[{imports + k}]") for k in range(len(bodies))]
    if shutil.which("rustfilt"):
        out = subprocess.run(["rustfilt"], input="\n".join(mangled), capture_output=True,
                             text=True).stdout.split("\n")
        demangled = out[:len(mangled)]
    else:
        demangled = mangled
    funcs = list(zip(demangled, bodies))
    total = sum(bodies)

    def path_of(name, d):
        # The path of the item the function belongs to: drop generic arguments and
        # impl brackets, keep the first `d` components.
        s = name
        m = re.match(r"^<(.*?)( as .*)?>::(.*)$", s)
        if m:
            s = m.group(1) + "::" + m.group(3)
        s = re.sub(r"<.*", "", s)
        parts = [p for p in s.split("::") if p]
        return "::".join(parts[:d]) or name

    def table(title, groups, n):
        print(f"\n## {title}\n")
        rows = sorted(groups.items(), key=lambda kv: -kv[1][0])
        for key, (size, count) in rows[:n]:
            print(f"{size:>9,} {100 * size / total:5.1f}% {count:>5}  {key}")

    for d, title in [(1, "code by crate"), (depth, f"code by path (depth {depth})")]:
        g = defaultdict(lambda: [0, 0])
        for name, size in funcs:
            g[path_of(name, d)][0] += size
            g[path_of(name, d)][1] += 1
        table(title, g, top)
    print(f"\n## largest functions ({len(funcs)} total, {total:,} bytes)\n")
    for name, size in sorted(funcs, key=lambda f: -f[1])[:top]:
        short = name if len(name) < 140 else name[:137] + "..."
        print(f"{size:>9,} {100 * size / total:5.1f}%  {short}")


main()
