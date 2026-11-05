# Assigns each extracted body (extract.py's output) to one cluster by rules and prints
# aggregates only: counts and size quantiles, never a body.
#   python3 work/research/0229/cluster.py
import json, re, collections, os
HERE = os.path.dirname(os.path.abspath(__file__))
rows = [json.loads(l) for l in open(os.path.join(HERE, "../../scratch/0229/private/codes.jsonl"))]
has = lambda p, c: re.search(p, c) is not None
TEXTP = r"\.match(All)?\(|\.replace(All)?\(|\.split\(|new RegExp|[=(,]\s*/[^/\s*][^/\n]*/[gimsuy]*|\.test\(|\.includes\(|\.trim\(|\.indexOf\("
JSONP = r"JSON\.parse|JSON\.stringify|\.sort\(|\.reduce\(|Object\.(entries|keys|values|fromEntries)|new Map|\.filter\("
TSP = r":\s*(string|number|boolean|any|unknown|Record<|Array<|\w+\[\])\s*[,)=;]|\binterface\s+\w+|\btype\s+\w+\s*=|\bas\s+(const|any|string|number)|<\w+>\(|\):\s*\w+\s*(=>|{)"
def cluster(r):
    c = r["code"]; n = len(r.get("calls") or [])
    stat = len(re.findall(r"tools\.\w+\s*\(", c))
    if has(TSP, c): return "ts"
    if n == 0 and stat == 0: return "compute"
    if has(r"\bimage\(", c): return "media"
    if len(re.findall(TEXTP, c)) >= 2: return "text"
    if len(re.findall(JSONP, c)) >= 2: return "json"
    if has(r"Promise\.all", c): return "fanout"
    if has(r"\bfor\s*\(|\bwhile\s*\(|\.forEach\(", c): return "loop"
    return "sequential"
def q(xs, ps=(25, 50, 90)):
    xs = sorted(xs); n = len(xs)
    return "/".join(str(xs[min(n-1, int(p/100*n))]) for p in ps)
groups = collections.defaultdict(list)
for r in rows: groups[cluster(r)].append(r)
print(f"{'cluster':11} {'n':>5} {'%':>4} {'chars p25/50/90':>18} {'calls p25/50/90':>16} {'result p25/50/90':>20}")
for k, g in sorted(groups.items(), key=lambda x: -len(x[1])):
    print(f"{k:11} {len(g):5} {round(100*len(g)/len(rows)):4} {q([len(r['code']) for r in g]):>18} {q([len(r.get('calls') or []) for r in g]):>16} {q([r.get('resultLen',0) for r in g]):>20}")
