#!/usr/bin/env python3
"""Sizes of pi session images compressed (zstd -3, the reference as a raw-content
dictionary, as the adapter does: D2261) against each reference `goldi-images` wrote.
usage: refs.py OUT   (the goldi-images output directory)"""
import glob, os, re, statistics, subprocess, sys

out = sys.argv[1]


def z(img, ref):
    return len(subprocess.run(["zstd", "-3", "-q", "-c", "-D", ref, img], capture_output=True, check=True).stdout)


for variant in sorted(os.listdir(out)):
    d = os.path.join(out, variant)
    if not os.path.isdir(d):
        continue
    print(f"== {variant} (median / max KB over idle points)")
    print(f"{'case':12} {'points':>6} {'image':>12} {'vs exact':>12} {'vs empty':>12} {'vs base':>12} {'layered vs stage1':>18}")
    cases = sorted({re.sub(r"-(today|layered)-\d+\.img$", "", os.path.basename(f)) for f in glob.glob(d + "/*-today-*.img")})
    for c in cases:
        cols = {"image": [], "exact": [], "empty": [], "base": [], "stage1": []}
        for f in sorted(glob.glob(f"{d}/{c}-today-*.img")):
            k = f.rsplit("-", 1)[1]
            cols["image"].append(os.path.getsize(f))
            for r in ["exact", "empty", "base"]:
                cols[r].append(z(f, f"{d}/{r}.img"))
            cols["stage1"].append(z(f"{d}/{c}-layered-{k}", f"{d}/stage1.img"))
        fmt = lambda v: f"{statistics.median(v)/1024:.1f} / {max(v)/1024:.1f}"
        print(f"{c:12} {len(cols['image']):>6} {fmt(cols['image']):>12} {fmt(cols['exact']):>12} {fmt(cols['empty']):>12} {fmt(cols['base']):>12} {fmt(cols['stage1']):>18}")
