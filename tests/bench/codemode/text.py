import re

hits = (await bash(command="rg -n resolveToken src lib test", timeout=30))["output"]
by_file = {}
for line in hits.split("\n"):
    m = re.match(r"^([^:]+):(\d+):(.*)$", line)
    if not m:
        continue
    by_file.setdefault(m.group(1), []).append({"line": int(m.group(2)), "code": m.group(3).strip()})
top = sorted(by_file.items(), key=lambda kv: -len(kv[1]))[:4]
report = []
for file, lst in top:
    src = await read(path=file)
    lines = src.split("\n")
    markers = [(i + 1, l) for i, l in enumerate(lines) if re.search(r"\b(TODO|FIXME)\b", l)]
    defs = [f"{m.group(1)}({m.group(2)})" for m in re.finditer(r"^export (?:async )?function (\w+)\((\w+)", src, re.M)]
    renamed = len(src.replace("Error(", "SessionError(")) - len(src)
    report.append(f"{file}: {len(lst)} hits, {len(lines)} lines, {len(defs)} functions, +{renamed} chars renamed")
    for n, l in markers[:5]:
        report.append(f"  {str(n).rjust(4)}: {l.strip()[:100]}")
    report.append("  defs: " + ", ".join(defs[:8]))
text(f"{len(by_file)} files, {len(hits.split(chr(10))) - 1} hits\n" + "\n".join(report))
