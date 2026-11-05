import re

by_ns = {}
for t in ALL_TOOLS:
    ns = t["name"].split("__")[0] if "__" in t["name"] else "(core)"
    by_ns.setdefault(ns, []).append(t["name"])
text("\n".join(f"{ns}: {len(names)}" for ns, names in by_ns.items()))
matches = [t["name"] for t in ALL_TOOLS if re.search(r"report|metric", t["name"] + " " + t["description"], re.I)]
sizes = [f"{px * 4 / 1024 / 1024:.1f} MiB" for px in [1280 * 720, 1920 * 1080, 3840 * 2160]]
__result = {"matches": matches, "sizes": sizes, "total": len(ALL_TOOLS)}
