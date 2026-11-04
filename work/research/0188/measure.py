import os, subprocess, glob, sys, statistics, zlib
rows = []
for d in ["today", "htoday"]:
    for f in sorted(glob.glob(f"{d}/*.dump.js")):
        r = f"resid/{d}/" + os.path.basename(f)
        if not os.path.exists(r): continue
        a = open(f, "rb").read(); b = open(r, "rb").read()
        za = len(subprocess.run(["zstd", "-3", "-c"], input=a, capture_output=True).stdout)
        zb = len(subprocess.run(["zstd", "-3", "-c"], input=b, capture_output=True).stdout)
        def t(p):
            o = subprocess.run(["./target/release/djs", "resume", p], env=dict(os.environ, DJS_TIME_LOAD="10"), capture_output=True, text=True).stdout
            return int(o.split()[2]) if o.startswith("load_ns") else None
        ta, tb = t(f), t(r)
        rows.append((d, os.path.basename(f), len(a), len(b), za, zb, ta, tb, a.count(b"$dump.resume(")))
with open("measure.tsv", "w") as out:
    for r in rows: out.write("\t".join(map(str, r)) + "\n")
for d in ["today", "htoday"]:
    rs = [r for r in rows if r[0] == d and r[6] and r[7]]
    sa = sum(r[2] for r in rs); sb = sum(r[3] for r in rs); za = sum(r[4] for r in rs); zb = sum(r[5] for r in rs)
    print(d, len(rs), "dumps; raw", sa, "->", sb, f"({sb/sa-1:+.1%})", "zstd", za, "->", zb, f"({zb/za-1:+.1%})")
    ratio = [r[3]/r[2] for r in rs]; zr = [r[5]/r[4] for r in rs]
    print("  per dump raw ratio median", round(statistics.median(ratio),3), "p90", round(sorted(ratio)[int(len(ratio)*.9)],3), "max", round(max(ratio),3), "; zstd median", round(statistics.median(zr),3), "max", round(max(zr),3))
    ta = [r[6] for r in rs]; tb = [r[7] for r in rs]
    print("  load best-of-10 us: median", statistics.median(ta)/1e3, "->", statistics.median(tb)/1e3, "; sum", sum(ta)/1e6, "ms ->", sum(tb)/1e6, "ms")
    lr = [r[7]/r[6] for r in rs]
    print("  load ratio median", round(statistics.median(lr),3), "p10", round(sorted(lr)[int(len(lr)*.1)],3), "p90", round(sorted(lr)[int(len(lr)*.9)],3))
