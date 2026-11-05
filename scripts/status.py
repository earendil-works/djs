#!/usr/bin/env python3
"""Records measured status numbers in tests/scores/status.tsv (D2101).

README.md and the guide show them through `djs-docs` generated regions (`gen:status`),
which only read the file: numbers that need a build, a benchmark or another repository
are measured here, on request, and checked in with the commit they were measured at.
Counts that follow from the repository (test262 groups, ratchets, corpus cases, the base
snapshot) are computed by djs-docs itself and need no recording.

usage:
  scripts/status.py size              build the shipped, slim and dump modules
                                      (scripts/build-wasm.sh) and record their sizes: bytes,
                                      gzip -9, brotli -q 11, code and data of the shipped one,
                                      its coded base section and the decoded base of slim
  scripts/status.py speed [-n N]      scripts/perf/wmeasure.py on the README workloads:
                                      instructions of djs-wasm and quickjs-wasi (needs
                                      scripts/perf/setup.sh; takes minutes)
  scripts/status.py pi                pi's codemode tests on djs (`node js/pi/pi-tests.ts`: the
                                      default mode, the next mode, the next mode with a dump at
                                      every idle point on the dump module; needs pi-private)
                                      and pi's benchmark vs quickjs-wasi (js/bench/pi.bench.ts,
                                      the median of 3 runs)
  scripts/status.py startup           js/bench/sandbox.bench.ts: vm_new, Sandbox.create, the
                                      first run (median of 3 runs)
  scripts/status.py dumps             scripts/dump-costs/dump-costs.ts, packaging B: dump sizes
                                      and times over the harvested corpus, and the log case
  scripts/status.py all               size, speed, pi, startup and dumps
  scripts/status.py set [--at WHEN] KEY VALUE...
                                      record values measured elsewhere, for example
                                      `set pi/passed 221 pi/total 226` after
                                      `node js/pi/pi-tests.ts`; --at names an older
                                      measurement (an issue or commit) instead of HEAD
"""
import datetime
import gzip
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
FILE = os.path.join(ROOT, "tests/scores/status.tsv")
WASM = os.path.join(ROOT, "target/wasm32-unknown-unknown/wasm/djs_wasm.wasm")
SLIM = os.path.join(ROOT, "target/djs-slim.wasm")
DUMP = os.path.join(ROOT, "target/djs-dump.wasm")
HEADER = [
    "# Measured status numbers (D2101): `scripts/status.py` writes this file, and the",
    "# `gen:status` regions of README.md and docs/guide read it. Do not edit by hand.",
    "key\tvalue\tcommit\tdate",
]
# The workloads of README.md's speed table (scripts/perf/w/), in table order.
SPEED = ["tight", "prop1", "strings", "arrays", "json", "json-date", "forof", "calls",
         "push-pop", "pi-like", "foreach", "higher-order"]


def load():
    rows = {}
    if os.path.exists(FILE):
        for line in open(FILE, encoding="utf-8"):
            line = line.rstrip("\n")
            if not line or line.startswith("#") or line.startswith("key\t"):
                continue
            k, v, c, d = line.split("\t")
            rows[k] = (v, c, d)
    return rows


def save(rows, new, at=None):
    commit = at or subprocess.run(["git", "rev-parse", "--short=10", "HEAD"], cwd=ROOT,
                            capture_output=True, text=True, check=True).stdout.strip()
    dirty = subprocess.run(["git", "status", "--porcelain", "--untracked-files=no"], cwd=ROOT,
                           capture_output=True, text=True, check=True).stdout.strip()
    if dirty and not at:
        commit += "-dirty"
    date = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
    for k, v in new.items():
        rows[k] = (str(v), commit, date)
        print(f"{k}\t{v}")
    with open(FILE, "w", encoding="utf-8") as f:
        f.write("\n".join(HEADER) + "\n")
        for k in sorted(rows):
            f.write("\t".join([k, *rows[k]]) + "\n")


def leb(b, i):
    r = shift = 0
    while True:
        x = b[i]
        i += 1
        r |= (x & 0x7F) << shift
        shift += 7
        if x < 0x80:
            return r, i


def section_sizes(b):
    """Payload bytes of the code (10) and data (11) sections."""
    out = {}
    i = 8
    while i < len(b):
        sid = b[i]
        n, j = leb(b, i + 1)
        out[sid] = out.get(sid, 0) + n
        i = j + n
    return out.get(10, 0), out.get(11, 0)


# The base snapshot of a module: decoded (an instance's `djs_base_len`) and coded (the
# `djs_base` custom section), as `decoded coded`.
BASE_JS = """
const fs = require("fs");
const m = new WebAssembly.Module(fs.readFileSync(process.argv[1]));
const x = new WebAssembly.Instance(m, {}).exports;
x.djs_base_ptr();
console.log(x.djs_base_len(), WebAssembly.Module.customSections(m, "djs_base")[0].byteLength);
"""


def base(path):
    out = subprocess.run(["node", "-e", BASE_JS, path], capture_output=True, text=True, check=True)
    decoded, coded = out.stdout.split()
    return int(decoded), int(coded)


def build(*flags):
    subprocess.run(["nice", "-n", "10", os.path.join(ROOT, "scripts/build-wasm.sh"), *flags], check=True)


def size():
    build()
    build("--slim")
    build("--dump")
    b = open(WASM, "rb").read()
    code, data = section_sizes(b)
    _, coded = base(WASM)
    new = {
        "wasm/bytes": len(b),
        "wasm/gzip": len(gzip.compress(b, compresslevel=9, mtime=0)),
        "wasm/code": code,
        "wasm/data": data,
        "wasm/base-coded": coded,
    }
    if shutil.which("brotli"):
        r = subprocess.run(["brotli", "-c", "-q", "11"], input=b, capture_output=True, check=True)
        new["wasm/brotli"] = len(r.stdout)
    for name, path in (("slim", SLIM), ("dump", DUMP)):
        m = open(path, "rb").read()
        new[f"{name}/bytes"] = len(m)
        new[f"{name}/gzip"] = len(gzip.compress(m, compresslevel=9, mtime=0))
    new["slim/base"], _ = base(SLIM)
    # What the dumper adds to the shipped module (the dump module is the same build with it).
    new["dump/extra-gzip"] = new["dump/gzip"] - new["wasm/gzip"]
    return new


def speed(n):
    out = subprocess.run(
        ["python3", os.path.join(ROOT, "scripts/perf/wmeasure.py"), "-n", str(n), "--wasm", WASM, *SPEED],
        cwd=ROOT, capture_output=True, text=True, check=True).stdout
    new = {}
    for line in out.splitlines():
        m = re.match(r"(\S+)\s+(djs-wasm|qjs-wasi)\s+([\d.]+)\s", line)
        if m:
            new[f"speed/{m.group(1)}/{m.group(2)}"] = m.group(3)
    missing = [w for w in SPEED if f"speed/{w}/djs-wasm" not in new or f"speed/{w}/qjs-wasi" not in new]
    if missing:
        sys.exit(f"status.py: wmeasure gave no numbers for {', '.join(missing)}:\n{out}")
    return new


def node(*args, env=None):
    r = subprocess.run(["node", *args], cwd=ROOT, capture_output=True, text=True,
                       env=dict(os.environ, **(env or {})))
    if r.returncode != 0:
        sys.exit(f"status.py: node {' '.join(args)} failed:\n{r.stdout[-3000:]}\n{r.stderr[-3000:]}")
    return r.stdout


def median(xs):
    xs = sorted(xs)
    return xs[len(xs) // 2]


def pi():
    new = {}
    modes = [
        ("pi", [], None),
        ("pi/next", ["--next"], None),
        ("pi/dump", ["--next", "--dump-every-idle"], {"DJS_WASM": DUMP}),
    ]
    for key, flags, env in modes:
        out = node("js/pi/pi-tests.ts", *flags, "--out", f"target/status-{key.replace('/', '-')}",
                   env=env)
        m = re.search(r"^(\d+) / (\d+) passed", out, re.M)
        if not m:
            sys.exit(f"status.py: no pass count from pi-tests {' '.join(flags)}:\n{out[-2000:]}")
        new[f"{key}/passed"], new[f"{key}/total"] = int(m.group(1)), int(m.group(2))
    # pi.bench.ts's table: a row per case, a column per engine (quickjs-wasi first).
    rows = {"script start": "start", "tool-heavy": "tool-heavy", "interrupt polls": "polls"}
    cols = ["qjs", "djs", "djs-prelude"]
    runs = {}
    for _ in range(3):
        out = node("js/bench/pi.bench.ts")
        if "quickjs-wasi" not in out:
            sys.exit(f"status.py: pi.bench.ts ran without quickjs-wasi (no pi-private):\n{out}")
        for line in out.splitlines():
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            for prefix, row in rows.items():
                if cells[0].startswith(prefix):
                    for col, cell in zip(cols, cells[1:]):
                        runs.setdefault(f"pibench/{row}/{col}", []).append(float(cell.split()[0]))
    if len(runs) != len(rows) * len(cols):
        sys.exit(f"status.py: pi.bench.ts gave {sorted(runs)}")
    for k, xs in runs.items():
        v = median(xs)
        new[k] = round(v) if k.startswith("pibench/polls") else v
    return new


def startup():
    pats = {"startup/vm-new-us": r"^vm_new: ([\d.]+) us", "startup/sandbox-us": r"^Sandbox.create: ([\d.]+) us",
            "startup/sandbox-run-us": r"^Sandbox.create \+ first run: ([\d.]+) us",
            "startup/instantiate-us": r"^instantiate: ([\d.]+) us"}
    runs = {k: [] for k in pats}
    for _ in range(3):
        out = node("js/bench/sandbox.bench.ts", env={"DJS_WASM": WASM})
        for k, p in pats.items():
            m = re.search(p, out, re.M)
            if not m:
                sys.exit(f"status.py: no {k} in sandbox.bench.ts's output:\n{out}")
            runs[k].append(float(m.group(1)))
    return {k: median(xs) for k, xs in runs.items()}


def dumps():
    out_dir = os.path.join(ROOT, "work/scratch/status-dumps")
    report = node("scripts/dump-costs/dump-costs.ts", out_dir, "--packaging=B")
    import json
    points = json.load(open(os.path.join(out_dir, "points.json")))
    corpus = points["corpus B"]
    m = re.search(r"(\d+) cases ran \((\d+) failed to start\), (\d+) idle points, (\d+) refused", report)
    if not m:
        sys.exit(f"status.py: no corpus summary in dump-costs' report:\n{report}")

    def pct(k, p):
        xs = sorted(x[k] for x in corpus)
        return xs[min(len(xs) - 1, int(p / 100 * len(xs)))]
    log = points["B/log-5mb"]
    return {
        "dumps/corpus-cases": int(m.group(1)),
        "dumps/idle-points": int(m.group(3)),
        "dumps/refused": int(m.group(4)),
        "dumps/raw-p50": pct("raw", 50),
        "dumps/raw-p99": pct("raw", 99),
        "dumps/zstd-p50": pct("z3", 50),
        "dumps/zstd-p99": pct("z3", 99),
        "dumps/write-ms-p50": round(pct("dumpMs", 50), 3),
        "dumps/write-ms-p99": round(pct("dumpMs", 99), 3),
        "dumps/resume-ms-p50": round(pct("resumeMs", 50), 3),
        "dumps/resume-ms-p99": round(pct("resumeMs", 99), 3),
        "dumps/log-raw": max(x["raw"] for x in log),
        "dumps/log-zstd": max(x["z3"] for x in log),
    }


def main():
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    rows = load()
    if args[0] == "size":
        save(rows, size())
    elif args[0] == "speed":
        n = int(args[2]) if len(args) > 2 and args[1] == "-n" else 3
        build()
        save(rows, speed(n))
    elif args[0] == "pi":
        build()
        build("--dump")
        save(rows, pi())
    elif args[0] == "startup":
        build()
        save(rows, startup())
    elif args[0] == "dumps":
        build()
        build("--dump")
        save(rows, dumps())
    elif args[0] == "all":
        save(rows, size())
        save(rows, speed(3))
        save(rows, pi())
        save(rows, startup())
        save(rows, dumps())
    elif args[0] == "set":
        at = None
        if len(args) > 2 and args[1] == "--at":
            at, args = args[2], args[2:]
        kv = args[1:]
        if not kv or len(kv) % 2:
            sys.exit(__doc__)
        save(rows, dict(zip(kv[0::2], kv[1::2])), at)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
