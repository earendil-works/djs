import math


def num(x):
    # JSON.stringify writes 12.0 as 12.
    return int(x) if x == int(x) else x


res = await query_rows(sql="select * from requests where day >= '2026-10-01'", limit=600)
rows = res["structuredContent"]["rows"]
by = {}
for r in rows:
    k = r["service"] + " " + r["route"]
    g = by.get(k) or {"service": r["service"], "route": r["route"], "n": 0, "errors": 0, "total": 0, "max": 0, "users": set()}
    g["n"] += 1
    if r["status"] >= 500 or r["level"] == "error":
        g["errors"] += 1
    g["total"] += r["durationMs"]
    g["max"] = max(g["max"], r["durationMs"])
    g["users"].add(r["userId"])
    by[k] = g
summary = [
    {"service": g["service"], "route": g["route"], "n": g["n"], "errorRate": num(math.floor(g["errors"] * 1000 / g["n"] + 0.5) / 1000), "avgMs": num(math.floor(g["total"] * 10 / g["n"] + 0.5) / 10), "maxMs": num(g["max"]), "users": len(g["users"])}
    for g in by.values()
]
summary = [g for g in summary if g["n"] >= 3]
summary.sort(key=lambda g: (-g["errorRate"], -g["avgMs"]))
pkg_text = (await bash(command="cat packages/server/package.json", timeout=10))["output"]
pkg = json.loads(pkg_text)
scripts = {k: v.replace("shx rm -rf", "node ../../scripts/clean.mjs").replace("shx chmod +x", "node ../../scripts/chmod.mjs") for k, v in pkg["scripts"].items()}
changed = [k for k in scripts if scripts[k] != pkg["scripts"][k]]
if changed:
    await edit(path="packages/server/package.json", edits=[{"oldText": json.dumps(pkg["scripts"], indent=2), "newText": json.dumps(scripts, indent=2)}])
__result = json.dumps({"top": summary[:15], "groups": len(summary), "rows": len(rows), "changed": changed}, indent=2)
