#!/usr/bin/env python3
"""vec-sites.py NAMES.wasm [--check BASELINE | --update BASELINE] [--top N]: the inlined
container glue of the module (0161, D2521): calls to `RawVec` growth (`grow_one`),
deallocation, `with_capacity` allocation, `reserve` and the allocation-error path, per
kind and by function. Each call is an inlined `push`, drop, `with_capacity` or `reserve`
at that site.

NAMES.wasm is a build with a name section and the shipped code:
  CARGO_PROFILE_WASM_STRIP=debuginfo scripts/with-wasm-env.sh cargo build -p djs-wasm \\
    --target wasm32-unknown-unknown --profile wasm
--check fails (exit 1) when a kind's total grew by more than 2% over the baseline, and
lists the functions that gained sites; --update writes the current counts as the
baseline (scripts/size/vec-sites.txt)."""
import collections
import re
import subprocess
import sys

KINDS = {
    "grow": "8grow_one",
    "drop": "10deallocate",
    "with_capacity": "15try_allocate_in",
    "reserve": "7reserve",
    "alloc_error": "12handle_error",
}

args = sys.argv[1:]
wasm = args[0]
top = int(args[args.index("--top") + 1]) if "--top" in args else 15
out = subprocess.run(["wasm-objdump", "-d", wasm], capture_output=True, text=True, check=True).stdout
per = collections.defaultdict(collections.Counter)
cur = None
for line in out.split("\n"):
    m = re.match(r"[0-9a-f]+ func\[\d+\] <(.+)>:", line)
    if m:
        cur = m.group(1)
        continue
    if " call " in line and "alloc" in line:
        for k, pat in KINDS.items():
            if pat in line and ("raw_vec" in line or "vec" in line):
                per[cur][k] += 1
names = list(per)
dem = subprocess.run(["rustfilt"], input="\n".join(names), capture_output=True, text=True).stdout.split("\n")
if len(dem) < len(names):
    dem = names
fn = {d: per[n] for n, d in zip(names, dem)}
total = collections.Counter()
for c in fn.values():
    total.update(c)
print("  ".join(f"{k} {total[k]}" for k in KINDS))
rows = sorted(fn.items(), key=lambda kv: -sum(kv[1].values()))[:top]
for d, c in rows:
    print(f"{sum(c.values()):5}  {d[:110]}")


def read(path):
    base, funcs = collections.Counter(), {}
    for line in open(path):
        if line.startswith("#") or not line.strip():
            continue
        k, rest = line.rstrip("\n").split("\t", 1)
        if k == "total":
            kind, n = rest.split("\t")
            base[kind] = int(n)
        else:
            funcs[rest] = int(k)
    return base, funcs


if "--update" in args:
    path = args[args.index("--update") + 1]
    with open(path, "w") as f:
        f.write("# scripts/size/vec-sites.py --update: inlined container glue of the shipped code (D2521)\n")
        for k in KINDS:
            f.write(f"total\t{k}\t{total[k]}\n")
        for d, c in sorted(fn.items()):
            f.write(f"{sum(c.values())}\t{d}\n")
    print(f"wrote {path}")
elif "--check" in args:
    base, funcs = read(args[args.index("--check") + 1])
    grew = [k for k in KINDS if total[k] > base[k] * 1.02 + 1]
    gained = sorted(((sum(c.values()) - funcs.get(d, 0), d) for d, c in fn.items()), reverse=True)
    gained = [(n, d) for n, d in gained if n > 0][:top]
    for k in KINDS:
        print(f"{k:14} {base[k]:5} -> {total[k]:5}")
    if gained:
        print("functions with more sites than the baseline:")
        for n, d in gained:
            print(f"  +{n:3}  {d[:110]}")
    if grew:
        print(f"vec-sites: more inlined container glue than the baseline: {', '.join(grew)}")
        sys.exit(1)
