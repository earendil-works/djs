#!/usr/bin/env python3
"""Rust vs C wasm code density on the same kernels (0123): builds kernels.c with clang -Os
and kernels.rs with rustc opt-level s (and z / C -Oz for reference), both linked with
--compress-relocations, and prints each export's code size including the functions it
calls (imports excluded). usage: scripts/size/density/run.py [OUTDIR]"""
import os, re, subprocess, sys
here = os.path.dirname(os.path.abspath(__file__))
out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(here, "../../../work/scratch/density")
os.makedirs(out, exist_ok=True)
clang = os.environ.get("CLANG", "/opt/homebrew/opt/llvm@22/bin/clang")
builds = {
    "C -Os": [clang, "--target=wasm32", "-Os", "-nostdlib", "-Wl,--no-entry", "-Wl,--allow-undefined",
              "-Wl,--compress-relocations", "-Wl,--strip-debug", os.path.join(here, "kernels.c"), "-o"],
    "C -Oz": [clang, "--target=wasm32", "-Oz", "-nostdlib", "-Wl,--no-entry", "-Wl,--allow-undefined",
              "-Wl,--compress-relocations", "-Wl,--strip-debug", os.path.join(here, "kernels.c"), "-o"],
}
for lvl in ("s", "z"):
    builds[f"Rust opt-level {lvl}"] = ["rustc", "--edition", "2024", "--crate-type", "cdylib",
        "--target", "wasm32-unknown-unknown", "-C", f"opt-level={lvl}", "-C", "panic=abort",
        "-C", "lto", "-C", "link-arg=--compress-relocations", "-C", "link-arg=--allow-undefined", "-C", "strip=debuginfo",
        os.path.join(here, "kernels.rs"), "-o"]

def funcs(path):
    """name -> (size, callees) for defined functions, from wasm-objdump."""
    dis = subprocess.run(["wasm-objdump", "-d", path], capture_output=True, text=True).stdout
    hdr = subprocess.run(["wasm-objdump", "-x", "-j", "Code", path], capture_output=True, text=True).stdout
    sizes = {}
    for line in hdr.splitlines():
        m = re.search(r"func\[(\d+)\] size=(\d+)(?: <(.*)>)?", line)
        if m:
            sizes[int(m.group(1))] = (int(m.group(2)), m.group(3) or m.group(1))
    calls, cur = {}, None
    for line in dis.splitlines():
        m = re.match(r"^[0-9a-f]+ func\[(\d+)\]", line)
        if m:
            cur = int(m.group(1)); calls[cur] = set(); continue
        m = re.search(r"\bcall (\d+)", line)
        if m and cur is not None:
            calls[cur].add(int(m.group(1)))
    exports = {}
    for line in subprocess.run(["wasm-objdump", "-x", "-j", "Export", path], capture_output=True, text=True).stdout.splitlines():
        m = re.search(r'func\[(\d+)\].*-> "(k_\w+)"', line)
        if m:
            exports[m.group(2)] = int(m.group(1))
    return sizes, calls, exports

rows = {}
for name, cmd in builds.items():
    wasm = os.path.join(out, re.sub(r"\W+", "_", name) + ".wasm")
    subprocess.run(cmd + [wasm], check=True)
    sizes, calls, exports = funcs(wasm)
    for k, f in sorted(exports.items()):
        seen, todo = set(), [f]
        while todo:
            x = todo.pop()
            if x in seen or x not in sizes: continue
            seen.add(x); todo += calls.get(x, ())
        rows.setdefault(k, {})[name] = sum(sizes[x][0] for x in seen)
names = list(builds)
print(f"{'kernel':13}" + "".join(f"{n:>18}" for n in names) + f"{'Rust s / C -Os':>16}")
tot = {n: 0 for n in names}
for k, r in rows.items():
    if "C -Os" not in r:
        continue
    for n in names: tot[n] += r.get(n, 0)
    print(f"{k:13}" + "".join(f"{r.get(n, 0):>18,}" for n in names) + f"{r['Rust opt-level s'] / r['C -Os']:>16.2f}")
print(f"{'total':13}" + "".join(f"{tot[n]:>18,}" for n in names) + f"{tot['Rust opt-level s'] / tot['C -Os']:>16.2f}")
for k, r in rows.items():
    if "C -Os" not in r:
        print(f"{k:13}" + "".join(f"{r.get(n, 0):>18,}" for n in names) + "  (Rust-only variant)")
