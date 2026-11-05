# The typed variant of json.py, with Python annotations (monty does not check them here).
from typing import Callable


def percentile(xs: list[float], p: float) -> float:
    s = sorted(xs)
    return s[min(len(s) - 1, int(p / 100 * len(s)))] if s else 0


def group_by(items: list[dict], key: Callable[[dict], str]) -> dict[str, list[dict]]:
    m: dict[str, list[dict]] = {}
    for it in items:
        m.setdefault(key(it), []).append(it)
    return m


res = await query_rows(sql="select * from requests where level != 'debug'", limit=600)
rows: list[dict] = res["structuredContent"]["rows"]
groups: list[dict] = []
for k, lst in group_by(rows, lambda r: f"{r['service']} {r['route']}").items():
    service, route = k.split(" ")
    groups.append({"service": service, "route": route, "n": len(lst), "errors": len([r for r in lst if r["status"] >= 500]), "durations": [r["durationMs"] for r in lst]})
out: list[dict] = [
    {"key": f"{g['service']}{g['route']}", "n": g["n"], "errorRate": g["errors"] / g["n"], "p50": percentile(g["durations"], 50), "p95": percentile(g["durations"], 95)}
    for g in groups if g["n"] > 2
]
out.sort(key=lambda s: -s["p95"])
levels = {k: len(v) for k, v in group_by(rows, lambda r: r["level"]).items()}
text("\n".join(f"{s['key'].ljust(40)} {str(s['n']).rjust(4)} {format(s['errorRate'] * 100, '.1f').rjust(5)}% p50 {s['p50']:.1f} p95 {s['p95']:.1f}" for s in out[:20]))
__result = {"groups": len(out), "levels": levels}
